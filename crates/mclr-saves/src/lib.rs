//! NBT saves, datapacks, servers.dat, and Server List Ping (SLP) / SRV resolution.

pub mod error;
pub mod saves;
pub mod servers;

pub use error::SavesError;
pub use saves::{
    DatapackInfo, InstanceFileItem, SaveInfo, SaveSpawn, ScreenshotInfo, ScreenshotMeta,
    ScreenshotsPagedResult, backup_save, copy_image_to_clipboard, delete_instance_file,
    delete_save, delete_screenshot, delete_screenshot_path, export_save_zip,
    generate_screenshot_thumb, import_save_zip, list_datapacks, list_instance_files, list_saves,
    list_screenshots, list_screenshots_paged, read_level_dat, toggle_datapack,
    toggle_instance_file, write_level_dat,
};
pub use servers::{
    ResolvedServerAddress, ServerEntry, ServerPingResult, SrvLookup, add_server, ping_server,
    ping_server_with_resolver, read_servers_dat, remove_server, resolve_server_address,
    resolve_server_address_with_resolver, update_server, write_servers_dat,
};
