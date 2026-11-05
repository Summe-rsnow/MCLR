//! Task engine integration, HTTP network probe, and task handlers registration.

pub mod handlers;

use crate::http::HttpState;
use futures_util::future;
use mclr_core::tasks::{NetProbe, TaskEngine};
use std::time::Duration;
use tauri::AppHandle;

pub struct HttpNetProbe {
    http: HttpState,
}

impl HttpNetProbe {
    pub fn new(http: HttpState) -> Self {
        Self { http }
    }
}

#[async_trait::async_trait]
impl NetProbe for HttpNetProbe {
    async fn online(&self) -> bool {
        let client = self.http.client();
        let timeout = Duration::from_secs(4);

        let mojang_url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
        let bmclapi_url = "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";

        let fetch = |url: &'static str| {
            let client = client.clone();
            async move {
                client
                    .get(url)
                    .timeout(timeout)
                    .send()
                    .await
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
        };

        let fut1 = Box::pin(fetch(mojang_url));
        let fut2 = Box::pin(fetch(bmclapi_url));

        future::select_ok(vec![fut1, fut2]).await.is_ok()
    }
}

pub async fn register_all(engine: &TaskEngine, app: &AppHandle) {
    handlers::register_all(engine, app).await;
}
