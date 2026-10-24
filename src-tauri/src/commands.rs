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
        crate::ipc::launch::launch_instance,
        crate::ipc::launch::launch_logs,
        crate::ipc::launch::launch_processes,
        crate::ipc::launch::launch_kill,
        // [loaders]
        crate::ipc::loaders::loaders_versions,
        crate::ipc::loaders::loaders_install,
        // [comp]
        crate::ipc::comp::comp_search,
        crate::ipc::comp::comp_project,
        crate::ipc::comp::comp_files,
        crate::ipc::comp::comp_download,
        // [modpack]
        crate::ipc::modpack::modpack_inspect,
        crate::ipc::modpack::modpack_import_file,
        crate::ipc::modpack::modpack_install_remote,
        // [localmods]
        crate::ipc::localmods::local_mods_list,
        crate::ipc::localmods::local_mod_toggle,
        crate::ipc::localmods::local_mod_delete,
        crate::ipc::localmods::local_mods_check_updates,
        crate::ipc::localmods::local_mod_update,
        crate::ipc::localmods::local_mod_jar_in_jar,
        crate::ipc::localmods::comp_resolve_dependencies,
        // [saves]
        crate::ipc::saves::saves_list,
        crate::ipc::saves::save_delete,
        crate::ipc::saves::save_backup,
        crate::ipc::saves::save_export,
        crate::ipc::saves::save_import,
        crate::ipc::saves::save_datapacks_list,
        crate::ipc::saves::save_datapack_toggle,
        crate::ipc::saves::screenshots_list,
        crate::ipc::saves::screenshot_delete,
        crate::ipc::saves::instance_files_list,
        crate::ipc::saves::instance_file_toggle,
        crate::ipc::saves::instance_file_delete,
        // [servers]
        crate::ipc::servers::servers_list,
        crate::ipc::servers::server_add,
        crate::ipc::servers::server_update,
        crate::ipc::servers::server_remove,
        crate::ipc::servers::server_ping,
        // [export]
        // [crash]
        // [home]
        // [tools]
        // [update]
    ])
}
