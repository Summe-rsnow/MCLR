use mclr_app_lib::commands;
use specta_typescript::{BigIntExportBehavior, Typescript};
use std::path::PathBuf;

#[test]
fn export_bindings() {
    let builder = commands::builder();
    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/ipc/bindings.ts");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("create src/ipc dir");
    }
    builder
        .export(
            Typescript::default()
                .bigint(BigIntExportBehavior::Number)
                .header("// @ts-nocheck\n"),
            &out_path,
        )
        .expect("Failed to export TypeScript bindings");
}
