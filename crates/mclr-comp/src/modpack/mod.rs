//! Modpack import and inspection (CurseForge, Modrinth, MultiMC/Prism, MCBBS, HMCL, Nested, RawMinecraft).

pub mod formats;
pub mod installer;

use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::CompError;
pub use installer::{ModpackImportContext, extract_modpack_overrides, import_modpack};

// ============================================================================
// Types
// ============================================================================

/// Supported modpack archive formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ModpackFormat {
    Curseforge,
    Modrinth,
    Prism,
    Mcbbs,
    Hmcl,
    Nested,
    RawMinecraft,
}

/// Metadata extracted from a modpack archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModpackInspectResult {
    pub format: ModpackFormat,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub mc_version: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub file_count: u32,
}

/// Remote file to be downloaded during modpack import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModpackFileDownload {
    pub rel_path: String,
    pub urls: Vec<String>,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub size: Option<u64>,
    pub cf_project_id: Option<u64>,
    pub cf_file_id: Option<u64>,
}

/// Full plan for installing a modpack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModpackInstallPlan {
    pub inspect: ModpackInspectResult,
    pub files: Vec<ModpackFileDownload>,
    pub override_prefixes: Vec<String>,
    pub nested_archive_entry: Option<String>,
}

// ============================================================================
// Format inspection
// ============================================================================

/// Inspects a modpack zip archive and returns an installation plan.
pub fn inspect_modpack(archive_path: &Path) -> Result<ModpackInstallPlan, CompError> {
    let file = std::fs::File::open(archive_path).map_err(CompError::Io)?;
    let mut zip = zip::ZipArchive::new(file).map_err(CompError::Zip)?;

    let entry_names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|e| e.name().replace('\\', "/")))
        .collect();

    // 1. MCBBS (mcbbs.packmeta)
    if let Some((entry, root_prefix)) = formats::find_entry_name(&entry_names, "mcbbs.packmeta") {
        return formats::inspect_mcbbs(&mut zip, entry, &root_prefix, archive_path);
    }

    // 2. MultiMC / Prism (mmc-pack.json)
    if let Some((entry, root_prefix)) = formats::find_entry_name(&entry_names, "mmc-pack.json") {
        return formats::inspect_prism(&mut zip, entry, &root_prefix, &entry_names, archive_path);
    }

    // 3. Modrinth (modrinth.index.json)
    if let Some((entry, root_prefix)) =
        formats::find_entry_name(&entry_names, "modrinth.index.json")
    {
        return formats::inspect_modrinth(&mut zip, entry, &root_prefix, archive_path);
    }

    // 4. CurseForge or MCBBS manifest (manifest.json)
    if let Some((entry, root_prefix)) = formats::find_entry_name(&entry_names, "manifest.json") {
        return formats::inspect_manifest(&mut zip, entry, &root_prefix, archive_path);
    }

    // 5. HMCL (modpack.json)
    if let Some((entry, root_prefix)) = formats::find_entry_name(&entry_names, "modpack.json") {
        return formats::inspect_hmcl(&mut zip, entry, &root_prefix, archive_path);
    }

    // 6. Nested launcher pack (modpack.zip or modpack.mrpack)
    if let Some(entry) = entry_names.iter().find(|n| {
        let base = n.rsplit('/').next().unwrap_or(n);
        base == "modpack.zip" || base == "modpack.mrpack"
    }) {
        return formats::inspect_nested(&mut zip, entry, archive_path);
    }

    // 7. Raw .minecraft zip
    if let Some(plan) = formats::try_inspect_raw_minecraft(&mut zip, &entry_names, archive_path)? {
        return Ok(plan);
    }

    Err(CompError::UnsupportedModpack(
        "Could not detect supported modpack format in archive".to_string(),
    ))
}

// ============================================================================
// Unit & Fixture Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::io::Write;
    use tempfile::tempdir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use zip::write::{SimpleFileOptions, ZipWriter};

    use super::*;
    use crate::curseforge::CurseForgeClient;

    fn create_zip_with_files(files: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let temp = tempfile::Builder::new().suffix(".zip").tempfile().unwrap();
        let mut zip = ZipWriter::new(std::fs::File::create(temp.path()).unwrap());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, content) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content).unwrap();
        }

        zip.finish().unwrap();
        temp
    }

    #[tokio::test]
    async fn test_curseforge_modpack_fixture_import() {
        let server = MockServer::start().await;

        let cf_api_response = serde_json::json!({
            "data": [
                {
                    "id": 4593548,
                    "modId": 238222,
                    "displayName": "jei-1.20.1-forge-15.2.0.27.jar",
                    "fileName": "jei-1.20.1-forge-15.2.0.27.jar",
                    "releaseType": 1,
                    "fileDate": "2024-01-01T00:00:00Z",
                    "fileLength": 20,
                    "downloadUrl": format!("{}/download/jei.jar", server.uri()),
                    "allowModDistribution": true,
                    "gameVersions": ["1.20.1", "Forge"],
                    "hashes": [],
                    "dependencies": [],
                    "fileFingerprint": 123456789
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path("/v1/mods/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(cf_api_response))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/download/jei.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"dummy jei jar binary"))
            .mount(&server)
            .await;

        let manifest_json = serde_json::json!({
            "minecraft": {
                "version": "1.20.1",
                "modLoaders": [{ "id": "forge-47.2.0", "primary": true }]
            },
            "manifestType": "minecraftModpack",
            "manifestVersion": 1,
            "name": "CF Test Pack",
            "version": "1.0.0",
            "author": "Tester",
            "files": [{ "projectID": 238222, "fileID": 4593548, "required": true }],
            "overrides": "overrides"
        });
        let manifest_bytes = serde_json::to_vec(&manifest_json).unwrap();

        let zip_file = create_zip_with_files(&[
            ("manifest.json", &manifest_bytes),
            ("overrides/config/test.cfg", b"test_key=test_value\n"),
        ]);

        let mc_dir = tempdir().unwrap();
        let client = reqwest::Client::new();
        let net_settings = mclr_core::NetSettings::default();
        let cf_client = CurseForgeClient::new(mclr_core::settings::SourcePriority::Smart)
            .with_base_url(server.uri())
            .with_api_key(Some("test-key".to_string()));

        let info = import_modpack(ModpackImportContext {
            archive_path: zip_file.path(),
            instance_name: "cf-pack-instance",
            mc_folder: mc_dir.path(),
            client: &client,
            net_settings: &net_settings,
            java_settings: None,
            cf_client: Some(&cf_client),
            task_ctx: None,
            install_base_game: false,
        })
        .await
        .unwrap();

        assert_eq!(info.config.isolation, "on");

        let instance_dir = mc_dir.path().join("versions").join("cf-pack-instance");
        assert!(instance_dir.join("mclr.instance.json").exists());
        let jar_path = instance_dir
            .join("mods")
            .join("jei-1.20.1-forge-15.2.0.27.jar");
        assert!(jar_path.exists());
        assert_eq!(std::fs::read(&jar_path).unwrap(), b"dummy jei jar binary");

        let cfg_path = instance_dir.join("config").join("test.cfg");
        assert!(cfg_path.exists());
        assert_eq!(std::fs::read(&cfg_path).unwrap(), b"test_key=test_value\n");
    }

    #[tokio::test]
    async fn test_modrinth_mrpack_fixture_import() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/download/fabric-api.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"dummy fabric api binary"))
            .mount(&server)
            .await;

        let index_json = serde_json::json!({
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.0.0",
            "name": "MR Test Pack",
            "files": [
                {
                    "path": "mods/fabric-api.jar",
                    "hashes": {},
                    "env": { "client": "required" },
                    "downloads": [format!("{}/download/fabric-api.jar", server.uri())],
                    "fileSize": 23
                }
            ],
            "dependencies": {
                "minecraft": "1.20.1",
                "fabric-loader": "0.15.11"
            }
        });
        let index_bytes = serde_json::to_vec(&index_json).unwrap();

        let zip_file = create_zip_with_files(&[
            ("modrinth.index.json", &index_bytes),
            ("overrides/options.txt", b"fov:70\n"),
            ("client-overrides/config/c.json", b"{\"theme\":\"dark\"}"),
        ]);

        let mc_dir = tempdir().unwrap();
        let client = reqwest::Client::new();
        let net_settings = mclr_core::NetSettings::default();

        let info = import_modpack(ModpackImportContext {
            archive_path: zip_file.path(),
            instance_name: "mr-pack-instance",
            mc_folder: mc_dir.path(),
            client: &client,
            net_settings: &net_settings,
            java_settings: None,
            cf_client: None,
            task_ctx: None,
            install_base_game: false,
        })
        .await
        .unwrap();

        assert_eq!(info.config.isolation, "on");

        let instance_dir = mc_dir.path().join("versions").join("mr-pack-instance");
        assert!(instance_dir.join("mclr.instance.json").exists());
        let jar_path = instance_dir.join("mods").join("fabric-api.jar");
        assert!(jar_path.exists());
        assert_eq!(
            std::fs::read(&jar_path).unwrap(),
            b"dummy fabric api binary"
        );

        let opt_path = instance_dir.join("options.txt");
        assert!(opt_path.exists());
        assert_eq!(std::fs::read(&opt_path).unwrap(), b"fov:70\n");

        let cfg_path = instance_dir.join("config").join("c.json");
        assert!(cfg_path.exists());
        assert_eq!(std::fs::read(&cfg_path).unwrap(), b"{\"theme\":\"dark\"}");
    }

    #[tokio::test]
    async fn test_prism_multimc_zip_fixture_import() {
        let mmc_json = serde_json::json!({
            "components": [
                { "cachedName": "Minecraft", "uid": "net.minecraft", "version": "1.20.1" },
                { "cachedName": "Fabric", "uid": "net.fabricmc.fabric-loader", "version": "0.15.11" }
            ],
            "formatVersion": 1
        });
        let mmc_bytes = serde_json::to_vec(&mmc_json).unwrap();

        let zip_file = create_zip_with_files(&[
            ("mmc-pack.json", &mmc_bytes),
            ("instance.cfg", b"name=Prism Test Pack\n"),
            (".minecraft/mods/sample.jar", b"sample jar binary content"),
        ]);

        let plan = inspect_modpack(zip_file.path()).unwrap();
        assert_eq!(plan.inspect.format, ModpackFormat::Prism);
        assert_eq!(plan.inspect.name, "Prism Test Pack");
        assert_eq!(plan.inspect.mc_version, "1.20.1");
        assert_eq!(plan.inspect.loader.as_deref(), Some("fabric"));

        let mc_dir = tempdir().unwrap();
        let client = reqwest::Client::new();
        let net_settings = mclr_core::NetSettings::default();

        let info = import_modpack(ModpackImportContext {
            archive_path: zip_file.path(),
            instance_name: "prism-instance",
            mc_folder: mc_dir.path(),
            client: &client,
            net_settings: &net_settings,
            java_settings: None,
            cf_client: None,
            task_ctx: None,
            install_base_game: false,
        })
        .await
        .unwrap();

        assert_eq!(info.config.isolation, "on");
        let instance_dir = mc_dir.path().join("versions").join("prism-instance");
        let sample_jar = instance_dir.join("mods").join("sample.jar");
        assert!(sample_jar.exists());
        assert_eq!(
            std::fs::read(&sample_jar).unwrap(),
            b"sample jar binary content"
        );
    }

    #[test]
    fn test_quilt_modpack_rejection() {
        // 1. Modrinth with quilt
        let mr_quilt = serde_json::json!({
            "formatVersion": 1,
            "game": "minecraft",
            "dependencies": {
                "minecraft": "1.20.1",
                "quilt-loader": "0.20.0"
            }
        });
        let zip_mr = create_zip_with_files(&[(
            "modrinth.index.json",
            &serde_json::to_vec(&mr_quilt).unwrap(),
        )]);
        let err = inspect_modpack(zip_mr.path()).unwrap_err();
        assert_eq!(err.code(), "modpack.quiltUnsupported");

        // 2. CurseForge with quilt
        let cf_quilt = serde_json::json!({
            "minecraft": {
                "version": "1.20.1",
                "modLoaders": [{ "id": "quilt-0.20.0", "primary": true }]
            },
            "manifestType": "minecraftModpack"
        });
        let zip_cf =
            create_zip_with_files(&[("manifest.json", &serde_json::to_vec(&cf_quilt).unwrap())]);
        let err = inspect_modpack(zip_cf.path()).unwrap_err();
        assert_eq!(err.code(), "modpack.quiltUnsupported");

        // 3. MultiMC with quilt
        let mmc_quilt = serde_json::json!({
            "components": [
                { "uid": "net.minecraft", "version": "1.20.1" },
                { "uid": "org.quiltmc.quilt-loader", "version": "0.20.0" }
            ]
        });
        let zip_mmc =
            create_zip_with_files(&[("mmc-pack.json", &serde_json::to_vec(&mmc_quilt).unwrap())]);
        let err = inspect_modpack(zip_mmc.path()).unwrap_err();
        assert_eq!(err.code(), "modpack.quiltUnsupported");
    }

    #[test]
    fn test_nested_and_hmcl_and_raw_inspection() {
        // HMCL
        let hmcl_json = serde_json::json!({
            "name": "HMCL Test",
            "version": "1.2.3",
            "author": "HMCL Team",
            "gameVersion": "1.19.2",
            "loaders": [{ "type": "fabric", "version": "0.14.9" }]
        });
        let zip_hmcl =
            create_zip_with_files(&[("modpack.json", &serde_json::to_vec(&hmcl_json).unwrap())]);
        let plan_hmcl = inspect_modpack(zip_hmcl.path()).unwrap();
        assert_eq!(plan_hmcl.inspect.format, ModpackFormat::Hmcl);
        assert_eq!(plan_hmcl.inspect.name, "HMCL Test");
        assert_eq!(plan_hmcl.inspect.mc_version, "1.19.2");
        assert_eq!(plan_hmcl.inspect.loader.as_deref(), Some("fabric"));

        // Nested
        let zip_nested =
            create_zip_with_files(&[("modpack.zip", &std::fs::read(zip_hmcl.path()).unwrap())]);
        let plan_nested = inspect_modpack(zip_nested.path()).unwrap();
        assert_eq!(plan_nested.inspect.format, ModpackFormat::Nested);
        assert_eq!(
            plan_nested.nested_archive_entry.as_deref(),
            Some("modpack.zip")
        );
        assert_eq!(plan_nested.inspect.mc_version, "1.19.2");

        // Raw .minecraft
        let version_json = serde_json::json!({
            "id": "1.20.1-fabric",
            "inheritsFrom": "1.20.1",
            "time": "2026-01-01T00:00:00+00:00",
            "releaseTime": "2026-01-01T00:00:00+00:00",
            "type": "release",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient"
        });
        let zip_raw = create_zip_with_files(&[
            (
                ".minecraft/versions/1.20.1-fabric/1.20.1-fabric.json",
                &serde_json::to_vec(&version_json).unwrap(),
            ),
            (".minecraft/mods/test.jar", b"jar content"),
        ]);
        let plan_raw = inspect_modpack(zip_raw.path()).unwrap();
        assert_eq!(plan_raw.inspect.format, ModpackFormat::RawMinecraft);
        assert_eq!(plan_raw.inspect.mc_version, "1.20.1");
    }

    #[test]
    fn test_path_traversal_prevention() {
        let zip_traversal = create_zip_with_files(&[
            ("overrides/../dangerous.txt", b"danger"),
            ("overrides/normal.txt", b"safe"),
        ]);
        let target_dir = tempdir().unwrap();
        let extracted = extract_modpack_overrides(
            zip_traversal.path(),
            target_dir.path(),
            &["overrides/".to_string()],
        )
        .unwrap();

        assert_eq!(extracted, 1);
        assert!(target_dir.path().join("normal.txt").exists());
        assert!(
            !target_dir
                .path()
                .parent()
                .unwrap()
                .join("dangerous.txt")
                .exists()
        );
    }
}
