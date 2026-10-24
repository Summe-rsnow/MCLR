//! Java discovery and Mojang runtime download IPC commands.

use mclr_core::{AppError, SettingsStore, TaskCenter, paths};
use mclr_game::{JavaInstallation, resolve_mojang_runtime_downloads, scan_java};
use mclr_net::{DownloadSpec, Downloader, SourceKind, build_http_client, rewrite_url};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use url::Url;

#[tauri::command]
#[specta::specta]
pub async fn java_scan(state: State<'_, SettingsStore>) -> Result<Vec<JavaInstallation>, AppError> {
    let settings = state.get().await;
    let custom = settings.java.custom.clone();
    let list = tokio::task::spawn_blocking(move || scan_java(&custom))
        .await
        .map_err(|e| AppError::new("game.io").with_detail(e.to_string()))?;
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub async fn java_download(
    settings_state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    component: String,
) -> Result<String, AppError> {
    let settings = settings_state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    let mut params = HashMap::new();
    params.insert("component".to_string(), component.clone());

    let (task_id, _token) = tasks_state
        .create_task(None, "tasks.titles.javaDownload", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();
    let net_settings = settings.net.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        let dest_dir = paths::runtime_dir().join(&component);
        match resolve_mojang_runtime_downloads(&client, &component, &dest_dir).await {
            Ok(runtime_specs) => {
                let mut dl_specs = Vec::with_capacity(runtime_specs.len());
                for rspec in runtime_specs {
                    if let Ok(parsed) = Url::parse(&rspec.url) {
                        let rewritten = rewrite_url(&parsed, SourceKind::Assets);
                        let urls: Vec<Url> = rewritten.into_iter().map(|(_, u)| u).collect();
                        dl_specs.push(DownloadSpec {
                            urls,
                            dest: rspec.dest,
                            size: Some(rspec.size),
                            sha1: Some(rspec.sha1),
                            sha256: None,
                            md5: None,
                        });
                    }
                }
                let downloader = Downloader::new(client, net_settings);
                match downloader.download_many(dl_specs, Some((&tc, &tid))).await {
                    Ok(()) => {
                        let _ = tc.complete_task(&tid).await;
                    }
                    Err(e) => {
                        let _ = tc.fail_task(&tid, e.into()).await;
                    }
                }
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, e.into()).await;
            }
        }
    });

    Ok(task_id)
}
