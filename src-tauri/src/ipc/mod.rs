//! IPC command modules grouped by feature.

pub mod accounts;
pub mod app;
pub mod assets;
pub mod comp;
pub mod crash;
pub mod export;
pub mod home;
pub mod install;
pub mod instances;
pub mod java;
pub mod launch;
pub mod loaders;
pub mod localmods;
pub mod modpack;
pub mod presets;
pub mod resources;
pub mod saves;
pub mod servers;
pub mod settings;
pub mod tasks;
pub mod tools;
pub mod update;

use mclr_core::AppError;

/// Run synchronous fs/zip/hash/NBT/image work on Tokio's blocking pool so IPC commands
/// never stall the async runtime. A panic or cancellation becomes `app.blockingJoin`.
pub async fn run_blocking<T, E>(
    f: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> Result<T, AppError>
where
    T: Send + 'static,
    E: Into<AppError> + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(res) => res.map_err(Into::into),
        Err(join) => Err(AppError::new("app.blockingJoin").with_detail(join.to_string())),
    }
}

use mclr_core::SettingsStore;
use std::path::PathBuf;
use tauri::State;

/// Currently selected Minecraft folder (settings, falling back to the default folder).
pub async fn current_mc_folder(state: &State<'_, SettingsStore>) -> PathBuf {
    let settings = state.snapshot().await;
    mc_folder_of(&settings)
}

/// Game directory of an instance via the single isolation resolver (`mclr_game::gamedir`).
pub async fn game_dir_for(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
    // One settings read for both the folder and the isolation policy.
    let settings = state.snapshot().await;
    let mc_folder = mc_folder_of(&settings);
    let id = instance_id.to_string();
    tokio::task::spawn_blocking(move || {
        mclr_game::gamedir::resolve_game_dir(&mc_folder, &id, &settings.game)
    })
    .await
    .unwrap_or_default()
}

/// Selected folder from an already-materialised settings snapshot.
fn mc_folder_of(settings: &mclr_core::Settings) -> PathBuf {
    if settings.game.current_folder.is_empty() {
        mclr_core::paths::default_minecraft_dir()
    } else {
        PathBuf::from(&settings.game.current_folder)
    }
}
