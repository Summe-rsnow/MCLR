//! Tools IPC commands.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use mclr_core::{AppError, SettingsStore, TaskCenter, is_launcher_log_file, paths};
use mclr_net::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SkinFetchResult {
    pub username: String,
    pub uuid: String,
    pub skin_url: String,
    pub skin_data_url: String,
    pub head_data_url: String,
    pub slim: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CacheSizeInfo {
    pub cache_bytes: u64,
    pub logs_bytes: u64,
    pub skin_cache_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Deserialize)]
struct MojangProfileResponse {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct SessionProperty {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct SessionProfileResponse {
    _id: Option<String>,
    _name: Option<String>,
    properties: Vec<SessionProperty>,
}

#[derive(Deserialize)]
struct TexturesPayload {
    textures: TexturesData,
}

#[derive(Deserialize)]
struct TexturesData {
    #[serde(rename = "SKIN")]
    skin: Option<SkinInfo>,
}

#[derive(Deserialize)]
struct SkinInfo {
    url: String,
    #[serde(default)]
    metadata: Option<SkinMetadata>,
}

#[derive(Deserialize)]
struct SkinMetadata {
    #[serde(default)]
    model: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn tools_fetch_skin(
    http: State<'_, crate::http::HttpState>,
    username: String,
) -> Result<SkinFetchResult, AppError> {
    let username_trimmed = username.trim();
    if username_trimmed.is_empty() {
        return Err(AppError::new("tools.skin.invalidUsername"));
    }

    let client = http.client();

    let mojang_url = format!(
        "https://api.mojang.com/users/profiles/minecraft/{}",
        urlencoding_encode(username_trimmed)
    );
    let profile_resp = client
        .get(&mojang_url)
        .send()
        .await
        .map_err(|e| AppError::new("tools.skin.networkError").with_detail(e.to_string()))?;

    if profile_resp.status() == reqwest::StatusCode::NOT_FOUND
        || profile_resp.status() == reqwest::StatusCode::NO_CONTENT
    {
        return Err(
            AppError::new("tools.skin.userNotFound").with_param("username", username_trimmed)
        );
    }

    if !profile_resp.status().is_success() {
        return Err(AppError::new("tools.skin.networkError")
            .with_detail(format!("HTTP {}", profile_resp.status())));
    }

    let mojang_profile: MojangProfileResponse = profile_resp
        .json()
        .await
        .map_err(|e| AppError::new("tools.skin.parseError").with_detail(e.to_string()))?;

    let session_url = format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{}",
        mojang_profile.id
    );
    let session_resp = client
        .get(&session_url)
        .send()
        .await
        .map_err(|e| AppError::new("tools.skin.networkError").with_detail(e.to_string()))?;

    if !session_resp.status().is_success() {
        return Err(AppError::new("tools.skin.networkError")
            .with_detail(format!("HTTP {}", session_resp.status())));
    }

    let session_profile: SessionProfileResponse = session_resp
        .json()
        .await
        .map_err(|e| AppError::new("tools.skin.parseError").with_detail(e.to_string()))?;

    let textures_prop = session_profile
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .ok_or_else(|| AppError::new("tools.skin.noSkinFound"))?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(&textures_prop.value)
        .map_err(|e| AppError::new("tools.skin.parseError").with_detail(e.to_string()))?;

    let textures_payload: TexturesPayload = serde_json::from_slice(&decoded_bytes)
        .map_err(|e| AppError::new("tools.skin.parseError").with_detail(e.to_string()))?;

    let skin = textures_payload
        .textures
        .skin
        .ok_or_else(|| AppError::new("tools.skin.noSkinFound"))?;

    let skin_bytes = client
        .get(&skin.url)
        .send()
        .await
        .map_err(|e| AppError::new("tools.skin.downloadFailed").with_detail(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| AppError::new("tools.skin.downloadFailed").with_detail(e.to_string()))?;

    let skin_base64 = base64::engine::general_purpose::STANDARD.encode(&skin_bytes);
    let skin_data_url = format!("data:image/png;base64,{skin_base64}");

    let id_for_crop = mojang_profile.id.clone();
    let head_data_url =
        super::run_blocking(move || mclr_auth::crop_skin_head(&skin_bytes, &id_for_crop)).await?;

    let is_slim = skin
        .metadata
        .as_ref()
        .and_then(|m| m.model.as_deref())
        .map(|model| model == "slim")
        .unwrap_or(false);

    Ok(SkinFetchResult {
        username: mojang_profile.name,
        uuid: mojang_profile.id,
        skin_url: skin.url,
        skin_data_url,
        head_data_url,
        slim: is_slim,
    })
}

fn urlencoding_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[tauri::command]
#[specta::specta]
pub async fn tools_save_skin(
    settings_state: State<'_, SettingsStore>,
    path: String,
    data_url: String,
) -> Result<(), AppError> {
    let settings = settings_state.get().await;
    // Writes are confined to the launcher's own directories.
    let target = crate::security::ensure_allowed(Path::new(&path), &settings)?;
    write_skin_png(&target, &data_url).await
}

/// Decodes a `data:image/png;base64,…` payload and writes it to `path`.
///
/// Split out from the command so the decoder stays unit-testable without a
/// `SettingsStore`/containment context.
async fn write_skin_png(path: &Path, data_url: &str) -> Result<(), AppError> {
    const PREFIX: &str = "data:image/png;base64,";
    let base64_data = data_url
        .strip_prefix(PREFIX)
        .ok_or_else(|| AppError::new("tools.invalidSkinData"))?;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| AppError::new("tools.invalidSkinData").with_detail(e.to_string()))?;

    if bytes.is_empty() {
        return Err(AppError::new("tools.invalidSkinData"));
    }

    tokio::fs::write(path, &bytes)
        .await
        .map_err(|e| AppError::new("tools.saveFailed").with_detail(e.to_string()))?;

    Ok(())
}

/// Generates a head avatar PNG (data URL) from a skin file on disk.
///
/// Mirrors PCL-CE's "皮肤头像生成器": the 8x8 face is upscaled with an inset
/// brim, then the hat/hair overlay is composited on top at full canvas size.
#[tauri::command]
#[specta::specta]
pub async fn tools_generate_skin_avatar(skin_path: String, size: u32) -> Result<String, AppError> {
    let skin_path = crate::security::ensure_not_system_path(Path::new(&skin_path))?
        .to_string_lossy()
        .to_string();
    let bytes = tokio::fs::read(&skin_path)
        .await
        .map_err(|e| AppError::new("tools.avatar.readFailed").with_detail(e.to_string()))?;

    super::run_blocking(move || -> Result<String, AppError> {
        let png = mclr_auth::generate_head_avatar(&bytes, size)
            .map_err(|e| AppError::new("tools.avatar.invalidSize").with_detail(e.to_string()))?;
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        ))
    })
    .await
}

fn compute_dir_size(dir: &Path) -> u64 {
    if !dir.exists() {
        return 0;
    }
    let mut total = 0u64;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(meta) = entry.metadata() {
            if meta.is_dir() {
                total += compute_dir_size(&path);
            } else {
                total += meta.len();
            }
        }
    }
    total
}

fn compute_logs_size(logs_dir: &Path) -> u64 {
    if !logs_dir.exists() {
        return 0;
    }
    let mut total = 0u64;
    let entries = match fs::read_dir(logs_dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if is_launcher_log_file(file_name) {
                    if let Ok(meta) = entry.metadata() {
                        total += meta.len();
                    }
                }
            }
        }
    }
    total
}

fn compute_cache_sizes() -> CacheSizeInfo {
    let cache_p = paths::cache_dir();
    let logs_p = paths::logs_dir();
    let skin_p = cache_p.join("skin");

    let cache_bytes = compute_dir_size(&cache_p);
    let logs_bytes = compute_logs_size(&logs_p);
    let skin_cache_bytes = compute_dir_size(&skin_p);
    let total_bytes = cache_bytes + logs_bytes;

    CacheSizeInfo {
        cache_bytes,
        logs_bytes,
        skin_cache_bytes,
        total_bytes,
    }
}

#[tauri::command]
#[specta::specta]
pub async fn tools_cache_info() -> Result<CacheSizeInfo, AppError> {
    tokio::task::spawn_blocking(compute_cache_sizes)
        .await
        .map_err(|e| AppError::new("core.io").with_detail(e.to_string()))
}

fn clear_dir_contents(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// Appends `dir` to `failed` when the removal failed, so the command can report it.
fn collect_failures(result: std::io::Result<()>, dir: &Path, failed: &mut Vec<String>) {
    if let Err(e) = result {
        failed.push(format!("{}: {e}", dir.display()));
    }
}

/// Clears a log directory but keeps the newest launcher log, which the running
/// appender holds open (deleting it fails on Windows or detaches the writer).
fn clear_log_dir_keep_newest(dir: &Path, failed: &mut Vec<String>) {
    if !dir.exists() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            failed.push(format!("{}: {e}", dir.display()));
            return;
        }
    };

    let mut logs: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(mclr_core::is_launcher_log_file)
        })
        .collect();
    logs.sort();
    logs.pop();

    for path in logs {
        collect_failures(fs::remove_file(&path), &path, failed);
    }

    // Anything else in the logs directory (unrelated files) is left untouched.
}

#[tauri::command]
#[specta::specta]
pub async fn tools_cache_clear(
    clear_cache: bool,
    clear_logs: bool,
    clear_skins: bool,
) -> Result<CacheSizeInfo, AppError> {
    tokio::task::spawn_blocking(move || {
        let cache_p = paths::cache_dir();
        let logs_p = paths::logs_dir();
        let skin_p = cache_p.join("skin");

        let mut failed: Vec<String> = Vec::new();

        if clear_cache {
            collect_failures(clear_dir_contents(&cache_p), &cache_p, &mut failed);
        } else if clear_skins {
            collect_failures(clear_dir_contents(&skin_p), &skin_p, &mut failed);
        }

        if clear_logs {
            clear_log_dir_keep_newest(&logs_p, &mut failed);
        }

        // Report failures instead of claiming success: the UI toasts "cleaned".
        if !failed.is_empty() {
            return Err(AppError::new("core.io")
                .with_detail(format!("failed to remove: {}", failed.join(", "))));
        }

        Ok(compute_cache_sizes())
    })
    .await
    .map_err(|e| AppError::new("core.io").with_detail(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn tools_custom_download(
    http: State<'_, crate::http::HttpState>,
    settings_state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    url: String,
    dest_path: String,
) -> Result<String, AppError> {
    let parsed_url = Url::parse(&url)
        .map_err(|e| AppError::new("tools.download.invalidUrl").with_detail(e.to_string()))?;

    // Arbitrary write target: confine it to the launcher's own directories.
    let settings_snapshot = settings_state.get().await;
    let dest = crate::security::ensure_allowed(Path::new(&dest_path), &settings_snapshot)?;
    if let Some(parent) = dest.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| AppError::new("core.io").with_detail(e.to_string()))?;
    }

    let file_name = dest
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "download.bin".to_string());

    let mut params = std::collections::HashMap::new();
    params.insert("name".to_string(), file_name);

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.customDownload", params)
        .await;

    let settings = settings_state.get().await;
    let client = http.client();
    let downloader = Downloader::new(client, settings.net.clone());

    let spec = DownloadSpec {
        urls: vec![parsed_url],
        dest,
        size: None,
        sha1: None,
        sha256: None,
        md5: None,
    };

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match downloader.download(spec, Some((&tc, &tid))).await {
            Ok(()) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn tools_create_shortcut(
    instance_id: String,
    shortcut_name: Option<String>,
) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let current_exe = std::env::current_exe()
            .map_err(|e| AppError::new("tools.shortcut.failed").with_detail(e.to_string()))?;

        let desktop_dir = dirs::desktop_dir().unwrap_or_else(paths::data_dir);
        if !desktop_dir.exists() {
            fs::create_dir_all(&desktop_dir)
                .map_err(|e| AppError::new("tools.shortcut.failed").with_detail(e.to_string()))?;
        }

        let clean_name = shortcut_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Minecraft - {instance_id}"));

        // Sanitize filename for Windows
        let sanitized_name: String = clean_name
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                other => other,
            })
            .collect();

        let shortcut_path = desktop_dir.join(format!("{sanitized_name}.lnk"));

        create_shell_link_win32(
            &shortcut_path,
            &current_exe,
            &format!("--launch \"{instance_id}\""),
            current_exe.parent().unwrap_or(&desktop_dir),
            &format!("Launch Minecraft {instance_id}"),
        )
        .map_err(|e| AppError::new("tools.shortcut.failed").with_detail(e))?;

        Ok(shortcut_path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| AppError::new("core.io").with_detail(e.to_string()))?
}

#[cfg(windows)]
fn create_shell_link_win32(
    link_path: &Path,
    target_path: &Path,
    arguments: &str,
    working_dir: &Path,
    description: &str,
) -> Result<(), String> {
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IPersistFile,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface, PCWSTR};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("CoCreateInstance failed: {e}"))?;

        let target_h = HSTRING::from(target_path.as_os_str());
        shell_link
            .SetPath(PCWSTR::from_raw(target_h.as_ptr()))
            .map_err(|e| format!("SetPath failed: {e}"))?;

        if !arguments.is_empty() {
            let args_h = HSTRING::from(arguments);
            shell_link
                .SetArguments(PCWSTR::from_raw(args_h.as_ptr()))
                .map_err(|e| format!("SetArguments failed: {e}"))?;
        }

        if working_dir.exists() {
            let work_h = HSTRING::from(working_dir.as_os_str());
            shell_link
                .SetWorkingDirectory(PCWSTR::from_raw(work_h.as_ptr()))
                .map_err(|e| format!("SetWorkingDirectory failed: {e}"))?;
        }

        if !description.is_empty() {
            let desc_h = HSTRING::from(description);
            shell_link
                .SetDescription(PCWSTR::from_raw(desc_h.as_ptr()))
                .map_err(|e| format!("SetDescription failed: {e}"))?;
        }

        let persist_file: IPersistFile = shell_link
            .cast()
            .map_err(|e| format!("QueryInterface IPersistFile failed: {e}"))?;

        let link_h = HSTRING::from(link_path.as_os_str());
        persist_file
            .Save(PCWSTR::from_raw(link_h.as_ptr()), true)
            .map_err(|e| format!("IPersistFile::Save failed: {e}"))?;

        CoUninitialize();
        Ok(())
    }
}

#[cfg(not(windows))]
fn create_shell_link_win32(
    link_path: &Path,
    _target_path: &Path,
    _arguments: &str,
    _working_dir: &Path,
    _description: &str,
) -> Result<(), String> {
    fs::write(link_path, b"mock-lnk-content")
        .map_err(|e| format!("failed to write mock shortcut: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_info_and_clear() {
        let temp_path =
            std::env::temp_dir().join(format!("mclr_tools_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_path);
        fs::create_dir_all(&temp_path).unwrap();
        unsafe {
            std::env::set_var("MCLR_DATA_DIR", temp_path.to_str().unwrap());
        }

        let cache_dir = paths::cache_dir();
        let logs_dir = paths::logs_dir();
        let skin_dir = cache_dir.join("skin");

        fs::create_dir_all(&skin_dir).unwrap();
        fs::create_dir_all(&logs_dir).unwrap();

        fs::write(skin_dir.join("test_skin.png"), b"skin_bytes_data").unwrap();
        fs::write(cache_dir.join("temp.dat"), b"cache_bytes").unwrap();
        fs::write(logs_dir.join("mclr.2026-10-26.log"), b"log_bytes").unwrap();

        let info = compute_cache_sizes();
        assert!(info.skin_cache_bytes > 0);
        assert!(info.cache_bytes >= info.skin_cache_bytes);
        assert!(info.logs_bytes > 0);
        assert_eq!(info.total_bytes, info.cache_bytes + info.logs_bytes);

        // Clear skins only
        let _ = clear_dir_contents(&skin_dir);
        let info2 = compute_cache_sizes();
        assert_eq!(info2.skin_cache_bytes, 0);
        assert!(info2.cache_bytes > 0); // temp.dat remains

        // Clear all cache and logs
        let _ = clear_dir_contents(&cache_dir);
        let _ = clear_dir_contents(&logs_dir);
        let info3 = compute_cache_sizes();
        assert_eq!(info3.cache_bytes, 0);
        assert_eq!(info3.logs_bytes, 0);
        assert_eq!(info3.total_bytes, 0);

        unsafe {
            std::env::remove_var("MCLR_DATA_DIR");
        }
        let _ = fs::remove_dir_all(&temp_path);
    }
    #[tokio::test]
    async fn test_tools_save_skin() {
        let temp_dir =
            std::env::temp_dir().join(format!("mclr_test_save_skin_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let target = temp_dir.join("skin.png");
        let target_str = target.to_string_lossy().to_string();

        // Valid 1x1 base64 png
        let valid_data_url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let res = write_skin_png(std::path::Path::new(&target_str), valid_data_url).await;
        assert!(res.is_ok());
        assert!(target.exists());
        assert!(fs::metadata(&target).unwrap().len() > 0);

        // Invalid prefix
        let err_res = write_skin_png(std::path::Path::new(&target_str), "not_data_url").await;
        assert!(err_res.is_err());
        assert_eq!(err_res.unwrap_err().code, "tools.invalidSkinData");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
