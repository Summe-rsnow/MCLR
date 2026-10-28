//! Java discovery IPC commands.

use mclr_core::{AppError, SettingsStore};
use mclr_game::{JavaInstallation, scan_java};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

/// Guards against overlapping background Java scans. A scan only starts when it can
/// flip this from `false` to `true`; it is reset when the scan finishes (success or error).
static JAVA_SCAN_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Resets [`JAVA_SCAN_IN_FLIGHT`] even if the scan future is dropped or panics.
struct ScanInFlightGuard;

impl Drop for ScanInFlightGuard {
    fn drop(&mut self) {
        JAVA_SCAN_IN_FLIGHT.store(false, Ordering::Release);
    }
}

#[tauri::command]
#[specta::specta]
pub async fn java_scan(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    force_refresh: Option<bool>,
) -> Result<Vec<JavaInstallation>, AppError> {
    let settings = state.snapshot().await;
    let custom = settings.java.custom.clone();
    let force = force_refresh.unwrap_or(false);

    // A populated cache is authoritative unless the caller explicitly forces a refresh:
    // never spawn a speculative background scan on the hot path.
    if !force {
        let cached = mclr_game::java::cached_java();
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    // Only one scan at a time; concurrent callers get whatever is cached right now.
    if JAVA_SCAN_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(mclr_game::java::cached_java());
    }
    let _guard = ScanInFlightGuard;

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
