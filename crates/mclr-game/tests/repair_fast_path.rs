//! Startup repair fast path: a matching `.mclr-verified.json` must let `RepairMode::Fast`
//! finish without ever touching the asset index.

use std::path::Path;
use std::time::UNIX_EPOCH;

use mclr_core::settings::{NetSettings, SourcePriority};
use mclr_game::install::verified_manifest::{self, VerifiedManifest};
use mclr_game::{GameError, RepairMode, repair_instance};

const INSTANCE_ID: &str = "fastpath_inst";

fn write_instance(mc_folder: &Path) -> std::path::PathBuf {
    let instance_dir = mc_folder.join("versions").join(INSTANCE_ID);
    std::fs::create_dir_all(&instance_dir).unwrap();

    // The asset index URL is deliberately unreachable and no local index file exists,
    // so any code path that actually resolves assets fails loudly instead of silently
    // succeeding (which would make the fast path indistinguishable from the full one).
    let version_json = serde_json::json!({
        "id": INSTANCE_ID,
        "type": "release",
        "mainClass": "net.minecraft.client.main.Main",
        "assetIndex": {
            "id": "1.20",
            "url": "http://127.0.0.1:1/1.20.json"
        }
    });
    let version_json_path = instance_dir.join(format!("{INSTANCE_ID}.json"));
    std::fs::write(
        &version_json_path,
        serde_json::to_vec(&version_json).unwrap(),
    )
    .unwrap();
    version_json_path
}

fn matching_manifest(version_json_path: &Path) -> VerifiedManifest {
    let metadata = std::fs::metadata(version_json_path).unwrap();
    let mtime_ms = metadata
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    VerifiedManifest {
        version_json_mtime_ms: mtime_ms,
        version_json_len: metadata.len(),
        asset_index_id: "1.20".to_string(),
        lib_count: 0,
    }
}

#[tokio::test]
async fn test_fast_repair_skips_with_verified_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let mc_folder = temp.path();
    let version_json_path = write_instance(mc_folder);
    let instance_dir = mc_folder.join("versions").join(INSTANCE_ID);

    let manifest = matching_manifest(&version_json_path);
    verified_manifest::save(&instance_dir, &manifest);
    let manifest_path = instance_dir.join(".mclr-verified.json");
    assert!(
        manifest_path.exists(),
        "manifest must be written atomically"
    );
    assert_eq!(
        verified_manifest::load(&instance_dir).unwrap(),
        manifest,
        "manifest must round-trip unchanged"
    );

    let client = reqwest::Client::new();
    let net_settings = NetSettings {
        source_priority: SourcePriority::OfficialFirst,
        ..Default::default()
    };

    let fast = repair_instance(
        &client,
        &net_settings,
        mc_folder,
        INSTANCE_ID,
        None,
        RepairMode::Fast,
    )
    .await;

    // Ok(0) can only be reached before the asset index is resolved: resolving it would
    // hit the unreachable URL above and return a network error.
    assert_eq!(fast.unwrap(), 0);
    assert!(
        !mc_folder.join("assets/indexes/1.20.json").exists(),
        "fast path must not create the asset index"
    );

    // Without the manifest the run must take the full path, which now fails because the
    // asset index is missing and its URL is unreachable.
    std::fs::remove_file(&manifest_path).unwrap();
    let full = repair_instance(
        &client,
        &net_settings,
        mc_folder,
        INSTANCE_ID,
        None,
        RepairMode::Fast,
    )
    .await;
    match full {
        Err(GameError::Net(_)) => {}
        other => panic!("expected the asset index fetch to fail, got {other:?}"),
    }
    assert!(
        !manifest_path.exists(),
        "a failed repair must not leave a fingerprint behind"
    );
}

#[tokio::test]
async fn test_full_repair_ignores_matching_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let mc_folder = temp.path();
    let version_json_path = write_instance(mc_folder);
    let instance_dir = mc_folder.join("versions").join(INSTANCE_ID);

    verified_manifest::save(&instance_dir, &matching_manifest(&version_json_path));

    let client = reqwest::Client::new();
    let net_settings = NetSettings {
        source_priority: SourcePriority::OfficialFirst,
        ..Default::default()
    };

    // `Full` is the "check and repair" contract: it must never trust the fingerprint.
    let full = repair_instance(
        &client,
        &net_settings,
        mc_folder,
        INSTANCE_ID,
        None,
        RepairMode::Full,
    )
    .await;
    match full {
        Err(GameError::Net(_)) => {}
        other => panic!("expected explicit repair to verify assets, got {other:?}"),
    }
}

#[test]
fn test_manifest_ignores_corrupt_file() {
    let temp = tempfile::tempdir().unwrap();
    let instance_dir = temp.path();
    std::fs::write(instance_dir.join(".mclr-verified.json"), b"{ not json").unwrap();
    assert!(verified_manifest::load(instance_dir).is_none());
}
