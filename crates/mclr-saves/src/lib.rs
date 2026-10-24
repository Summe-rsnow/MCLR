//! NBT saves, datapacks, servers.dat, and Server List Ping (SLP) / SRV resolution.

pub mod error;
pub mod saves;
pub mod servers;

pub use error::SavesError;
pub use saves::{
    DatapackInfo, InstanceFileItem, SaveInfo, SaveSpawn, ScreenshotInfo, backup_save,
    delete_instance_file, delete_save, delete_screenshot, export_save_zip, import_save_zip,
    list_datapacks, list_instance_files, list_saves, list_screenshots, read_level_dat,
    toggle_datapack, toggle_instance_file, write_level_dat,
};
pub use servers::{
    ResolvedServerAddress, ServerEntry, ServerPingResult, SrvLookup, add_server, ping_server,
    ping_server_with_resolver, read_servers_dat, remove_server, resolve_server_address,
    resolve_server_address_with_resolver, update_server, write_servers_dat,
};
