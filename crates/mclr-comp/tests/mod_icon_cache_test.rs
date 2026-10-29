//! End-to-end check that a mod jar's artwork reaches the icon cache the webview reads.
//!
//! `ResourceItemRow` renders mod icons through `convertFileSrc(entry.icon)`, so the path
//! handed to the frontend must be a real, decodable PNG on disk — not raw jar bytes.

use std::fs;
use std::io::Write;
use std::path::Path;

use mclr_comp::local::scan_local_mods;

/// 1x1 red PNG.
const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0xF7, 0x03, 0x41, 0x43, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn scanned_mod_exposes_cached_icon_path() {
    let data_dir = tempfile::tempdir().unwrap();
    // Single test in this binary: the process-wide data dir cannot race with another test.
    unsafe { std::env::set_var("MCLR_DATA_DIR", data_dir.path()) };

    let mods_dir = data_dir.path().join("mods");
    fs::create_dir_all(&mods_dir).unwrap();

    let jar_path = mods_dir.join("icon-mod.jar");
    let file = fs::File::create(&jar_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(br#"{"id": "icon_mod", "name": "Icon Mod", "version": "1.0.0"}"#)
        .unwrap();
    zip.start_file(
        "assets/icon_mod/icon.png",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(PNG_1X1).unwrap();
    zip.finish().unwrap();

    let mods = scan_local_mods(&mods_dir).unwrap();
    assert_eq!(mods.len(), 1);
    let icon = mods[0].icon.clone().expect("mod artwork should be exposed");

    let icon_path = Path::new(&icon);
    assert!(icon_path.is_file(), "cached icon must exist: {icon}");
    assert!(
        icon_path.starts_with(mclr_core::paths::cache_dir().join("icons")),
        "icon must live in the shared icon cache: {icon}"
    );

    let cached = fs::read(icon_path).unwrap();
    let image = image::load_from_memory(&cached).expect("cached icon must be a decodable image");
    // Artwork is pre-scaled in the cache: the UI only ever downscales it to its 40 px tile.
    assert_eq!(
        (image.width(), image.height()),
        (
            mclr_comp::resources::ICON_SIZE,
            mclr_comp::resources::ICON_SIZE
        ),
        "cached icon must be normalised to the shared icon canvas"
    );
}
