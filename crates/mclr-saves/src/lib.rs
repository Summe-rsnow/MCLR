//! NBT saves, datapacks, servers.dat, and Server List Ping (SLP) / SRV resolution.

pub mod error;
pub mod saves;
pub mod servers;

pub use error::SavesError;
pub use saves::{
    DatapackInfo, InstanceFileItem, SaveInfo, SaveSpawn, ScreenshotInfo, ScreenshotMeta,
    ScreenshotsPagedResult, backup_save, copy_image_to_clipboard, delete_save, delete_screenshot,
    delete_screenshot_path, export_save_zip, generate_screenshot_thumb, import_save_zip,
    list_datapacks, list_saves, list_screenshots_paged, read_level_dat, toggle_datapack,
    write_level_dat,
};
pub use servers::{
    ResolvedServerAddress, ServerEntry, ServerPingResult, add_server, ping_server,
    read_servers_dat, remove_server, resolve_server_address, update_server, write_servers_dat,
};
