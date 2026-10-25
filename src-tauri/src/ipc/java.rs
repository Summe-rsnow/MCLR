//! Java discovery and Mojang runtime download IPC commands.

use mclr_core::{AppError, SettingsStore, TaskCenter, paths};
use mclr_game::{JavaInstallation, resolve_mojang_runtime_downloads, scan_java};
use mclr_net::{DownloadSpec, Downloader, SourceKind, rewrite_url};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use url::Url;

#[tauri::command]
#[specta::specta]
pub async fn java_scan(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    force_refresh: Option<bool>,
) -> Result<Vec<JavaInstallation>, AppError> {
    let settings = state.get().await;
    let custom = settings.java.custom.clone();
    let force = force_refresh.unwrap_or(false);

    if !force {
        let cached = mclr_game::java::cached_java();
        if !cached.is_empty() {
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                let updated =
                    super::run_blocking(move || Ok::<_, AppError>(scan_java(&custom))).await;
                if let Ok(list) = updated {
                    let _ = app_clone.emit("java://updated", &list);
                }
            });
            return Ok(cached);
        }
    }

    let list = super::run_blocking(move || Ok::<_, AppError>(scan_java(&custom))).await?;
    let _ = app.emit("java://updated", &list);
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub async fn java_list(
    app: AppHandle,
    state: State<'_, SettingsStore>,
) -> Result<Vec<JavaInstallation>, AppError> {
    java_scan(app, state, Some(false)).await
}

#[tauri::command]
#[specta::specta]
pub async fn java_download(
    http: State<'_, crate::http::HttpState>,
    settings_state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    component: String,
) -> Result<String, AppError> {
    let settings = settings_state.get().await;
    let client = http.client();
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
