//! Central Tauri Specta command registry.
//! Split into marked `// [feature]` blocks so feature slices append without conflicts.

use tauri_specta::{Builder, collect_commands};

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        // [app]
        crate::ipc::app::app_info,
        crate::ipc::app::secrets_status,
        crate::ipc::app::window_set_material,
        // [assets]
        crate::ipc::assets::assets_allow,
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
        crate::ipc::instances::instance_open_path,
        // [java]
        crate::ipc::java::java_scan,
        crate::ipc::java::java_list,
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
        crate::ipc::install::install_create,
        crate::ipc::install::install_extras,
        crate::ipc::install::install_current_info,
        crate::ipc::install::install_validate_name,
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
        crate::ipc::comp::comp_pick_target,
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
        crate::ipc::localmods::resources_list,
        crate::ipc::localmods::local_mods_scan_start,
        crate::ipc::localmods::resource_toggle,
        crate::ipc::localmods::resources_delete,
        crate::ipc::localmods::resources_import,
        crate::ipc::localmods::resources_export,
        crate::ipc::localmods::schematic_inspect,
        crate::ipc::localmods::local_mods_dependents,
        crate::ipc::localmods::local_mods_update_many,
        // [saves]
        crate::ipc::saves::saves_list,
        crate::ipc::saves::save_delete,
        crate::ipc::saves::save_backup,
        crate::ipc::saves::save_export,
        crate::ipc::saves::save_import,
        crate::ipc::saves::save_datapacks_list,
        crate::ipc::saves::save_datapack_toggle,
        crate::ipc::saves::screenshots_list,
        crate::ipc::saves::screenshots_list_paged,
        crate::ipc::saves::screenshot_thumb,
        crate::ipc::saves::screenshot_delete,
        crate::ipc::saves::screenshot_reveal,
        crate::ipc::saves::screenshot_open,
        crate::ipc::saves::screenshot_copy,
        // [servers]
        crate::ipc::servers::servers_list,
        crate::ipc::servers::server_add,
        crate::ipc::servers::server_update,
        crate::ipc::servers::server_remove,
        crate::ipc::servers::server_ping,
        // [export]
        crate::ipc::export::export_scan_instance,
        crate::ipc::export::export_modpack,
        // [crash]
        crate::ipc::crash::crash_analyze,
        crate::ipc::crash::crash_export_report,
        // [home]
        crate::ipc::home::news_fetch,
        crate::ipc::home::announcements_fetch,
        // [tools]
        crate::ipc::tools::tools_fetch_skin,
        crate::ipc::tools::tools_cache_info,
        crate::ipc::tools::tools_cache_clear,
        crate::ipc::tools::tools_custom_download,
        crate::ipc::tools::tools_create_shortcut,
        crate::ipc::tools::tools_daily_luck,
        crate::ipc::tools::tools_list_media_files,
        // [update]
        crate::ipc::update::update_check,
        crate::ipc::update::update_install,
        crate::ipc::update::launcher_logs_list,
        crate::ipc::update::launcher_log_read,
    ])
}
