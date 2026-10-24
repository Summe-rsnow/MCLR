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
        // [install]
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
