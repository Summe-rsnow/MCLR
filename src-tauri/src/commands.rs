//! Central Tauri Specta command registry.
//! Split into marked `// [feature]` blocks so feature slices append without conflicts.

use tauri_specta::{Builder, collect_commands};

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        // [app]
        crate::ipc::app::app_info,
        // [settings]
        // [tasks]
        // [instances]
        // [java]
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
