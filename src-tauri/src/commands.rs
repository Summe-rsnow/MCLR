//! Central Tauri Specta command registry.
//! Split into marked `// [feature]` blocks so feature slices append without conflicts.

use tauri_specta::{Builder, collect_commands};

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        // [app]
        crate::ipc::app::app_info,
        crate::ipc::app::secrets_status,
        crate::ipc::app::window_set_material,
        // [settings]
        crate::ipc::settings::settings_get,
        crate::ipc::settings::settings_patch,
        // [tasks]
        crate::ipc::tasks::tasks_list,
        crate::ipc::tasks::task_cancel,
        crate::ipc::tasks::task_pause,
        crate::ipc::tasks::task_resume,
        // [instances]
        crate::ipc::instances::instances_list,
        crate::ipc::instances::instance_rename,
        crate::ipc::instances::instance_duplicate,
        crate::ipc::instances::instance_delete,
        crate::ipc::instances::instance_update_config,
        crate::ipc::instances::instance_open_folder,
        // [java]
        crate::ipc::java::java_scan,
        crate::ipc::java::java_download,
        // [accounts]
        crate::ipc::accounts::accounts_list,
        crate::ipc::accounts::accounts_select,
        crate::ipc::accounts::accounts_remove,
        crate::ipc::accounts::accounts_add_offline,
        crate::ipc::accounts::accounts_add_microsoft,
        crate::ipc::accounts::accounts_add_authlib,
        crate::ipc::accounts::accounts_select_profile,
        crate::ipc::accounts::accounts_refresh,
        crate::ipc::accounts::accounts_skin_upload,
        crate::ipc::accounts::accounts_cape_set,
        // [install]
        crate::ipc::install::install_manifest,
        crate::ipc::install::install_vanilla,
        crate::ipc::install::install_repair,
        // [launch]
        // [loaders]
        // [comp]
        // [modpack]
        // [localmods]
        // [saves]
        // [servers]
        // [export]
        // [crash]
        // [home]
        // [tools]
        // [update]
    ])
}
