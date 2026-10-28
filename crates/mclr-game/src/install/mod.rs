//! Minecraft vanilla installation, manifest fetching, and instance repair.

pub mod manifest;
pub mod pipeline;
pub mod specs;

pub use manifest::{
    VersionManifestEntry, VersionManifestList, fetch_version_manifest, load_resolved_version_json,
};
pub use pipeline::{cleanup_stale, install_vanilla, repair_instance};
pub use specs::{collect_version_download_specs, maven_coordinate_to_path};

#[cfg(test)]
mod tests {
    use mclr_core::settings::{NetSettings, SourcePriority};
    use sha1::{Digest, Sha1};
    use std::collections::HashSet;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::manifest::VersionManifestEntry;
    use super::pipeline::{cleanup_stale, install_vanilla, repair_instance};
    use super::specs::maven_coordinate_to_path;

    #[test]
    fn test_maven_coordinate_to_path() {
        assert_eq!(
            maven_coordinate_to_path("net.sf.jopt-simple:jopt-simple:5.0.4"),
            "net/sf/jopt-simple/jopt-simple/5.0.4/jopt-simple-5.0.4.jar"
        );
        assert_eq!(
            maven_coordinate_to_path("org.lwjgl:lwjgl:3.3.1:natives-windows"),
            "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"
        );
        assert_eq!(
            maven_coordinate_to_path("com.mojang:patchy:1.1@jar"),
            "com/mojang/patchy/1.1/patchy-1.1.jar"
        );
    }

    #[tokio::test]
    async fn test_install_and_repair_vanilla_wiremock() {
        let mock_server = MockServer::start().await;

        let client_jar_bytes = b"fake-minecraft-client-jar-data";
        let client_jar_sha1 = hex::encode(Sha1::digest(client_jar_bytes));

        let lib_bytes = b"fake-library-jar-content";
        let lib_sha1 = hex::encode(Sha1::digest(lib_bytes));

        let asset_obj_bytes = b"fake-sound-asset-bytes";
        let asset_obj_sha1 = hex::encode(Sha1::digest(asset_obj_bytes));

        let asset_index_json = serde_json::json!({
            "objects": {
                "minecraft/sounds/click.ogg": {
                    "hash": asset_obj_sha1,
                    "size": asset_obj_bytes.len()
                }
            }
        });
        let asset_index_bytes = serde_json::to_vec(&asset_index_json).unwrap();
        let asset_index_sha1 = hex::encode(Sha1::digest(&asset_index_bytes));

        let version_json_obj = serde_json::json!({
            "id": "1.20.1",
            "type": "release",
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": {
                "client": {
                    "sha1": client_jar_sha1,
                    "size": client_jar_bytes.len(),
                    "url": format!("{}/client.jar", mock_server.uri())
                }
            },
            "libraries": [
                {
                    "name": "org.example:testlib:1.0.0",
                    "downloads": {
                        "artifact": {
                            "path": "org/example/testlib/1.0.0/testlib-1.0.0.jar",
                            "sha1": lib_sha1,
                            "size": lib_bytes.len(),
                            "url": format!("{}/testlib-1.0.0.jar", mock_server.uri())
                        }
                    }
                }
            ],
            "assetIndex": {
                "id": "1.20",
                "sha1": asset_index_sha1,
                "size": asset_index_bytes.len(),
                "totalSize": asset_index_bytes.len(),
                "url": format!("{}/1.20.json", mock_server.uri())
            }
        });
        let version_json_bytes = serde_json::to_vec(&version_json_obj).unwrap();

        let manifest_json = serde_json::json!({
            "latest": {
                "release": "1.20.1",
                "snapshot": "23w31a"
            },
            "versions": [
                {
                    "id": "1.20.1",
                    "type": "release",
                    "url": format!("{}/1.20.1.json", mock_server.uri()),
                    "time": "2023-06-12T15:00:00+00:00",
                    "releaseTime": "2023-06-12T15:00:00+00:00"
                }
            ]
        });
        let manifest_bytes = serde_json::to_vec(&manifest_json).unwrap();

        Mock::given(method("GET"))
            .and(path("/1.20.1.json"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(version_json_bytes))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/client.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(client_jar_bytes.to_vec()))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/testlib-1.0.0.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(lib_bytes.to_vec()))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/1.20.json"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(asset_index_bytes))
            .mount(&mock_server)
            .await;

        let temp_dir = tempfile::tempdir().unwrap();
        let mc_folder = temp_dir.path();
        let cache_dir = mc_folder.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join("version_manifest_v2.json"), manifest_bytes).unwrap();

        // Pre-populate asset object so install_vanilla skips downloading it from internet
        let asset_sub = &asset_obj_sha1[0..2];
        let asset_dir = mc_folder.join("assets/objects").join(asset_sub);
        std::fs::create_dir_all(&asset_dir).unwrap();
        std::fs::write(asset_dir.join(&asset_obj_sha1), asset_obj_bytes).unwrap();
        let client = reqwest::Client::new();
        let net_settings = NetSettings {
            source_priority: SourcePriority::OfficialFirst,
            ..Default::default()
        };

        // Override MCLR_DATA_DIR so cache is read from temp_dir
        unsafe {
            std::env::set_var("MCLR_DATA_DIR", mc_folder.to_str().unwrap());
        }

        let instance_name = "test_instance";
        let info = install_vanilla(
            &client,
            &net_settings,
            mc_folder,
            "1.20.1",
            instance_name,
            None,
        )
        .await
        .unwrap();

        assert_eq!(info.id, "test_instance");
        assert_eq!(info.mc_version, "1.20.1");

        // Verify files exist on disk
        let client_jar = mc_folder.join("versions/test_instance/test_instance.jar");
        assert!(client_jar.exists());
        assert_eq!(std::fs::read(&client_jar).unwrap(), client_jar_bytes);

        let lib_jar = mc_folder.join("libraries/org/example/testlib/1.0.0/testlib-1.0.0.jar");
        assert!(lib_jar.exists());
        assert_eq!(std::fs::read(&lib_jar).unwrap(), lib_bytes);

        let asset_index = mc_folder.join("assets/indexes/1.20.json");
        assert!(asset_index.exists());

        // A second run (repair) should download 0 files because everything is already verified!
        let repaired = repair_instance(&client, &net_settings, mc_folder, instance_name, None)
            .await
            .unwrap();

        assert_eq!(repaired, 0);

        unsafe {
            std::env::remove_var("MCLR_DATA_DIR");
        }
    }

    #[test]
    fn test_cleanup_stale() {
        let temp = tempfile::tempdir().unwrap();
        let mc_folder = temp.path();
        let versions = mc_folder.join("versions");
        let incomplete = versions.join("incomplete_inst");
        let complete = versions.join("complete_inst");
        let staging = mc_folder.join(".mclr-staging");

        std::fs::create_dir_all(&incomplete).unwrap();
        let stale_marker = incomplete.join(".mclr-installing");
        std::fs::write(&stale_marker, b"").unwrap();
        // Backdate the marker: only genuinely abandoned installs are reclaimed.
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 60 * 60);
        std::fs::File::options()
            .write(true)
            .open(&stale_marker)
            .unwrap()
            .set_modified(old)
            .unwrap();
        std::fs::create_dir_all(&complete).unwrap();
        std::fs::write(complete.join("complete_inst.json"), b"{}").unwrap();
        // A recent marker must be left alone: a finished install whose marker
        // could not be unlinked must not lose its instance.
        let fresh = versions.join("fresh_inst");
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::write(fresh.join(".mclr-installing"), b"").unwrap();
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("temp.txt"), b"temp").unwrap();

        cleanup_stale(mc_folder);

        assert!(!incomplete.exists());
        assert!(complete.exists());
        assert!(fresh.exists(), "recent marker must not trigger deletion");
        assert!(!staging.exists());
    }

    #[test]
    fn test_uvmc_manifest_merge() {
        #[derive(Debug, serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct RawManifest {
            latest: serde_json::Value,
            versions: Vec<VersionManifestEntry>,
        }

        #[derive(Debug, serde::Deserialize)]
        struct UvmcManifestTest {
            versions: Vec<VersionManifestEntry>,
        }

        let raw_json = r#"{
            "latest": { "release": "1.20.1", "snapshot": "24w14a" },
            "versions": [
                { "id": "1.20.1", "type": "release", "url": "https://example.com/1.20.1.json", "releaseTime": "2023-06-12T10:00:00+00:00" }
            ]
        }"#;
        let uvmc_json = r#"{
            "versions": [
                { "id": "1.20.1", "type": "release", "url": "https://dup.com/1.20.1.json", "releaseTime": "2023-06-12T10:00:00+00:00" },
                { "id": "1.19_deep_dark_experimental_snapshot-1", "type": "pending", "url": "https://example.com/deep_dark.json", "releaseTime": "2022-02-17T13:55:59+00:00" }
            ]
        }"#;

        let mut raw: RawManifest = serde_json::from_str(raw_json).unwrap();
        let uvmc: UvmcManifestTest = serde_json::from_str(uvmc_json).unwrap();

        let existing_ids: HashSet<String> = raw.versions.iter().map(|v| v.id.clone()).collect();
        for u in uvmc.versions {
            if !existing_ids.contains(&u.id) {
                raw.versions.push(u);
            }
        }

        assert_eq!(raw.versions.len(), 2);
        assert_eq!(raw.versions[0].id, "1.20.1");
        assert_eq!(raw.versions[1].id, "1.19_deep_dark_experimental_snapshot-1");
        assert_eq!(raw.versions[1].version_type, "pending");
        assert_eq!(raw.versions[1].release_time, "2022-02-17T13:55:59+00:00");

        let serialized = serde_json::to_vec(&raw).unwrap();
        let deserialized: RawManifest = serde_json::from_slice(&serialized).unwrap();
        assert_eq!(deserialized.versions.len(), 2);
    }
}
