//! Guardrails for `tauri.conf.json` security settings.
//!
//! The CSP is a hand-maintained list of allowed origins; these tests fail when it
//! drifts from what the app actually loads (news image hosts, asset protocol) or
//! when someone loosens the important directives.

use std::path::PathBuf;

fn security() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let raw = std::fs::read_to_string(&path).expect("read tauri.conf.json");
    let conf: serde_json::Value = serde_json::from_str(&raw).expect("parse tauri.conf.json");
    conf["app"]["security"].clone()
}

fn directive<'a>(csp: &'a serde_json::Value, name: &str) -> &'a str {
    csp.get(name)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("csp.{name} missing"))
}

#[test]
fn production_csp_keeps_scripts_and_objects_locked() {
    let csp = security()["csp"].clone();

    let script = directive(&csp, "script-src");
    assert!(
        script.contains("'self'"),
        "script-src must allow self: {script}"
    );
    assert!(
        !script.contains("'unsafe-eval'"),
        "production must not allow eval: {script}"
    );
    assert!(
        !script.contains("'unsafe-inline'"),
        "production must not allow inline scripts: {script}"
    );

    assert_eq!(directive(&csp, "object-src"), "'none'");
    assert_eq!(directive(&csp, "base-uri"), "'self'");
    assert_eq!(directive(&csp, "frame-ancestors"), "'none'");
    assert_eq!(directive(&csp, "default-src"), "'self'");
}

#[test]
fn production_csp_has_no_direct_network_escape() {
    let csp = security()["csp"].clone();
    let connect = directive(&csp, "connect-src");

    // All network I/O goes through Rust over IPC; the webview must not fetch directly.
    assert!(connect.contains("ipc:"), "connect-src needs ipc: {connect}");
    assert!(
        connect.contains("http://ipc.localhost"),
        "connect-src needs the Windows IPC origin {connect}"
    );
    // Strip the one allowed http origin (Tauri's Windows IPC endpoint) first.
    let without_ipc = connect.replace("http://ipc.localhost", "");
    assert!(
        !without_ipc.contains("https:") && !without_ipc.contains("http://"),
        "connect-src must not allow general web origins: {connect}"
    );
}

#[test]
fn image_sources_cover_every_remote_host_the_app_renders() {
    let csp = security()["csp"].clone();
    let img = directive(&csp, "img-src");

    // Local sources: bundled assets, skin-head data URLs, asset protocol files.
    for required in ["'self'", "data:", "asset:", "http://asset.localhost"] {
        assert!(img.contains(required), "img-src missing {required}: {img}");
    }

    // Remote hosts. `mclr-app`'s news pipeline only forwards these domains
    // (`src-tauri/src/ipc/home.rs::ALLOWED_NEWS_DOMAINS`); keep the two in sync.
    for host in [
        "https://cdn.modrinth.com",
        "https://media.forgecdn.net",
        "https://minecraft.net",
        "https://*.minecraft.net",
        "https://*.minecraft-services.net",
        "https://*.microsoft.com",
    ] {
        assert!(img.contains(host), "img-src missing {host}: {img}");
    }

    // A blanket scheme would defeat the purpose of enumerating hosts.
    assert!(
        !img.split_whitespace().any(|source| source == "https:"),
        "img-src must enumerate hosts instead of allowing https: {img}"
    );
}

#[test]
fn dev_csp_only_relaxes_what_vite_needs() {
    let dev = security()["devCsp"].clone();

    let connect = directive(&dev, "connect-src");
    assert!(
        connect.contains("ws://localhost:"),
        "HMR needs a ws origin: {connect}"
    );

    // Even in dev the dangerous directives stay locked.
    assert_eq!(directive(&dev, "object-src"), "'none'");
    assert_eq!(directive(&dev, "base-uri"), "'self'");
}

#[test]
fn asset_protocol_is_enabled_and_scoped_at_runtime() {
    let security = security();
    assert_eq!(security["assetProtocol"]["enable"], serde_json::json!(true));

    // Paths are granted at runtime by `ipc::assets::allow_defaults`
    // (data dir + configured game folders), so the static list stays empty on
    // purpose: no blanket `**` glob is ever shipped.
    let scope = &security["assetProtocol"]["scope"];
    assert_eq!(
        scope,
        &serde_json::json!([]),
        "static asset scope must stay empty"
    );

    let assets = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ipc/assets.rs"),
    )
    .expect("read assets.rs");
    for expected in ["allow_defaults", "allow_game_folders", "allow_directory"] {
        assert!(
            assets.contains(expected),
            "asset scope runtime allowlist lost `{expected}`"
        );
    }
}
