//! Shared `reqwest::Client` managed state, rebuilt only when `settings.net` changes.

use mclr_core::AppError;
use mclr_core::settings::NetSettings;
use mclr_net::build_http_client;
use std::sync::RwLock;

pub struct HttpState {
    inner: RwLock<(NetSettings, reqwest::Client)>,
}

impl HttpState {
    pub fn new(net: &NetSettings) -> Result<Self, AppError> {
        let client = build_http_client(net).map_err(AppError::from)?;
        Ok(Self {
            inner: RwLock::new((net.clone(), client)),
        })
    }

    /// Cheap clone (`reqwest::Client` is an `Arc` internally).
    pub fn client(&self) -> reqwest::Client {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .1
            .clone()
    }

    /// Rebuild the client when the net settings differ from the current ones.
    pub fn sync(&self, net: &NetSettings) -> Result<(), AppError> {
        if self.inner.read().unwrap_or_else(|e| e.into_inner()).0 == *net {
            return Ok(());
        }
        let client = build_http_client(net).map_err(AppError::from)?;
        *self.inner.write().unwrap_or_else(|e| e.into_inner()) = (net.clone(), client);
        Ok(())
    }
}
