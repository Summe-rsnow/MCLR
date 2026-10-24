use crate::error::NetError;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use mclr_core::{NetSettings, ProxyMode};
use reqwest::dns::Resolve;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// Custom DNS resolver wrapping Hickory DNS-over-HTTPS.
pub struct DohResolver {
    resolver: TokioAsyncResolver,
}

impl DohResolver {
    pub fn new() -> Result<Self, NetError> {
        let mut config = ResolverConfig::new();

        // AliDNS DoH: 223.5.5.5:443 / dns.alidns.com
        let alidns_addr: SocketAddr = "223.5.5.5:443"
            .parse()
            .map_err(|e| NetError::Proxy(format!("Invalid AliDNS IP: {e}")))?;
        config.add_name_server(NameServerConfig {
            socket_addr: alidns_addr,
            protocol: Protocol::Https,
            tls_dns_name: Some("dns.alidns.com".to_string()),
            trust_negative_responses: true,
            tls_config: None,
            bind_addr: None,
        });

        // Cloudflare DoH: 1.1.1.1:443 / cloudflare-dns.com
        let cf_addr: SocketAddr = "1.1.1.1:443"
            .parse()
            .map_err(|e| NetError::Proxy(format!("Invalid Cloudflare IP: {e}")))?;
        config.add_name_server(NameServerConfig {
            socket_addr: cf_addr,
            protocol: Protocol::Https,
            tls_dns_name: Some("cloudflare-dns.com".to_string()),
            trust_negative_responses: true,
            tls_config: None,
            bind_addr: None,
        });

        let mut opts = ResolverOpts::default();
        opts.timeout = Duration::from_secs(5);
        opts.attempts = 2;

        let resolver = TokioAsyncResolver::tokio(config, opts);
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
