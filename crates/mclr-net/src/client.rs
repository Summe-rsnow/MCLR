use crate::error::NetError;
use hickory_resolver::Resolver;
use hickory_resolver::config::{NameServerConfig, ResolverConfig, ResolverOpts};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use mclr_core::{NetSettings, ProxyMode};
use reqwest::dns::Resolve;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// Custom DNS resolver wrapping Hickory DNS-over-HTTPS.
pub struct DohResolver {
    resolver: hickory_resolver::TokioResolver,
}

impl DohResolver {
    pub fn new() -> Result<Self, NetError> {
        let ip = |addr: &str| {
            addr.parse::<std::net::IpAddr>()
                .map_err(|e| NetError::Proxy(format!("Invalid DNS server IP {addr}: {e}")))
        };

        // AliDNS DoH (223.5.5.5 / dns.alidns.com) and Cloudflare DoH (1.1.1.1).
        let mut config = ResolverConfig::from_name_servers(vec![
            NameServerConfig::https(ip("223.5.5.5")?, Arc::from("dns.alidns.com"), None),
            NameServerConfig::https(ip("1.1.1.1")?, Arc::from("cloudflare-dns.com"), None),
        ]);
        config.add_search(hickory_resolver::proto::rr::Name::root());

        let mut opts = ResolverOpts::default();
        opts.timeout = Duration::from_secs(5);
        opts.attempts = 2;

        let resolver = Resolver::builder_with_config(config, TokioRuntimeProvider::default())
            .with_options(opts)
            .build()
            .map_err(|e| NetError::Proxy(format!("DNS resolver init failed: {e}")))?;
        Ok(Self { resolver })
    }
}

impl Resolve for DohResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let resolver = self.resolver.clone();
        let name_str = name.as_str().to_string();
        Box::pin(async move {
            match resolver.lookup_ip(name_str).await {
                Ok(lookup) => {
                    let addrs: Box<dyn Iterator<Item = SocketAddr> + Send> =
                        Box::new(lookup.into_iter().map(|ip| SocketAddr::new(ip, 0)));
                    Ok(addrs)
                }
                Err(err) => Err(Box::new(err) as Box<dyn std::error::Error + Send + Sync>),
            }
        })
    }
}

/// Constructs a configured `reqwest::Client` based on `NetSettings`.
pub fn build_http_client(settings: &NetSettings) -> Result<reqwest::Client, NetError> {
    let mut builder = reqwest::Client::builder()
        .user_agent(format!("MCLR/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .tcp_keepalive(Duration::from_secs(60));

    match &settings.proxy_mode {
        ProxyMode::None => {
            builder = builder.no_proxy();
        }
        ProxyMode::System => {
            // Default reqwest behavior uses system proxy
        }
        ProxyMode::Custom => {
            if !settings.proxy_url.trim().is_empty() {
                let proxy = reqwest::Proxy::all(&settings.proxy_url)
                    .map_err(|e| NetError::Proxy(format!("Invalid custom proxy URL: {e}")))?;
                builder = builder.proxy(proxy);
            }
        }
    }

    if settings.doh {
        let resolver = DohResolver::new()?;
        builder = builder.dns_resolver(Arc::new(resolver));
    }

    builder.build().map_err(NetError::Http)
}
