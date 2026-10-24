//! Modpack import and inspection (CurseForge, Modrinth, MultiMC/Prism, MCBBS, HMCL, Nested, RawMinecraft).

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use mclr_core::TaskCenter;
use mclr_game::instance::{
    InstanceConfig, InstanceInfo, InstanceKind, InstanceState, classify_instance,
    extract_mc_version, scan_instances, validate_instance_name,
};
use mclr_game::version::VersionJson;
use mclr_loaders::{LoaderInstallContext, LoaderKind, install_with_loader};
use mclr_net::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use specta::Type;
use tracing::{debug, info};
use url::Url;

use crate::curseforge::CurseForgeClient;
use crate::error::CompError;

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
    if let Some((entry, root_prefix)) = find_entry_name(&entry_names, "mcbbs.packmeta") {
        return inspect_mcbbs(&mut zip, entry, &root_prefix, archive_path);
    }

    // 2. MultiMC / Prism (mmc-pack.json)
    if let Some((entry, root_prefix)) = find_entry_name(&entry_names, "mmc-pack.json") {
        return inspect_prism(&mut zip, entry, &root_prefix, &entry_names, archive_path);
    }

    // 3. Modrinth (modrinth.index.json)
    if let Some((entry, root_prefix)) = find_entry_name(&entry_names, "modrinth.index.json") {
        return inspect_modrinth(&mut zip, entry, &root_prefix, archive_path);
    }

    // 4. CurseForge or MCBBS manifest (manifest.json)
    if let Some((entry, root_prefix)) = find_entry_name(&entry_names, "manifest.json") {
        return inspect_manifest(&mut zip, entry, &root_prefix, archive_path);
    }

    // 5. HMCL (modpack.json)
    if let Some((entry, root_prefix)) = find_entry_name(&entry_names, "modpack.json") {
        return inspect_hmcl(&mut zip, entry, &root_prefix, archive_path);
    }

    // 6. Nested launcher pack (modpack.zip or modpack.mrpack)
    if let Some(entry) = entry_names.iter().find(|n| {
        let base = n.rsplit('/').next().unwrap_or(n);
        base == "modpack.zip" || base == "modpack.mrpack"
    }) {
        return inspect_nested(&mut zip, entry, archive_path);
    }

    // 7. Raw .minecraft zip
    if let Some(plan) = try_inspect_raw_minecraft(&mut zip, &entry_names, archive_path)? {
        return Ok(plan);
    }

    Err(CompError::UnsupportedModpack(
        "Could not detect supported modpack format in archive".to_string(),
    ))
}

fn find_entry_name<'a>(names: &'a [String], target: &str) -> Option<(&'a str, String)> {
    for name in names {
        if name == target {
            return Some((name.as_str(), String::new()));
        }
        if let Some(rest) = name.strip_suffix(target) {
            let trimmed = rest.trim_end_matches('/');
            if !trimmed.contains('/') && rest.ends_with('/') {
                return Some((name.as_str(), rest.to_string()));
            }
        }
    }
    None
}

// ----------------------------------------------------------------------------
// Modrinth inspection
// ----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModrinthIndex {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version_id: Option<String>,
    #[serde(default)]
    files: Vec<ModrinthIndexFile>,
    #[serde(default)]
    dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModrinthIndexFile {
    path: String,
    #[serde(default)]
    hashes: HashMap<String, String>,
    #[serde(default)]
    env: Option<ModrinthEnv>,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default)]
    file_size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModrinthEnv {
    #[serde(default)]
    client: Option<String>,
}

fn inspect_modrinth(
    zip: &mut zip::ZipArchive<std::fs::File>,
    entry_name: &str,
    root_prefix: &str,
    archive_path: &Path,
) -> Result<ModpackInstallPlan, CompError> {
    let mut entry = zip.by_name(entry_name).map_err(CompError::Zip)?;
    let mut content = String::new();
    entry.read_to_string(&mut content).map_err(CompError::Io)?;
    let index: ModrinthIndex = serde_json::from_str(&content).map_err(CompError::Json)?;

    // Check Quilt
    if index.dependencies.contains_key("quilt-loader") || index.dependencies.contains_key("quilt") {
        return Err(CompError::QuiltUnsupported);
    }

    let mc_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let mut loader = None;
    let mut loader_version = None;
    if let Some(ver) = index.dependencies.get("fabric-loader") {
        loader = Some("fabric".to_string());
        loader_version = Some(ver.clone());
    } else if let Some(ver) = index.dependencies.get("neoforge") {
        loader = Some("neoforge".to_string());
        loader_version = Some(ver.clone());
    } else if let Some(ver) = index.dependencies.get("forge") {
        loader = Some("forge".to_string());
        loader_version = Some(ver.clone());
    }

    let mut files = Vec::new();
    for f in index.files {
        if f.env.as_ref().and_then(|e| e.client.as_deref()) == Some("unsupported") {
            continue;
        }
        files.push(ModpackFileDownload {
            rel_path: f.path,
            urls: f.downloads,
            sha1: f.hashes.get("sha1").cloned(),
            sha512: f.hashes.get("sha512").cloned(),
            size: if f.file_size > 0 {
                Some(f.file_size)
            } else {
                None
            },
            cf_project_id: None,
            cf_file_id: None,
        });
    }

    let fallback_name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ModrinthPack")
        .to_string();

    let name = if index.name.trim().is_empty() {
        fallback_name
    } else {
        index.name
    };

    let override_prefixes = vec![
        format!("{root_prefix}overrides/"),
        format!("{root_prefix}client-overrides/"),
    ];

    let inspect = ModpackInspectResult {
        format: ModpackFormat::Modrinth,
        name,
        version: index.version_id.unwrap_or_else(|| "1.0.0".to_string()),
        author: None,
        mc_version,
        loader,
        loader_version,
        file_count: files.len() as u32,
    };

    Ok(ModpackInstallPlan {
        inspect,
        files,
        override_prefixes,
        nested_archive_entry: None,
    })
}

// ----------------------------------------------------------------------------
// CurseForge / Manifest inspection
// ----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifest {
    #[serde(default)]
    minecraft: Option<CfMinecraft>,
    #[serde(default)]
    _manifest_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    files: Vec<CfManifestFile>,
    #[serde(default)]
    overrides: Option<String>,
    #[serde(default)]
    addons: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMinecraft {
    #[serde(default)]
    version: String,
    #[serde(default)]
    mod_loaders: Vec<CfModLoader>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfModLoader {
    id: String,
    #[serde(default)]
    _primary: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    #[serde(default = "default_true")]
    required: Option<bool>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

fn inspect_manifest(
    zip: &mut zip::ZipArchive<std::fs::File>,
    entry_name: &str,
    root_prefix: &str,
    archive_path: &Path,
) -> Result<ModpackInstallPlan, CompError> {
    let mut entry = zip.by_name(entry_name).map_err(CompError::Zip)?;
    let mut content = String::new();
    entry.read_to_string(&mut content).map_err(CompError::Io)?;
    let manifest: CfManifest = serde_json::from_str(&content).map_err(CompError::Json)?;

    // If manifest has addons, treat as MCBBS pack
    if manifest.addons.is_some() {
        return inspect_mcbbs_from_manifest(&manifest, root_prefix, archive_path);
    }

    let mc_info = manifest.minecraft.ok_or_else(|| {
        CompError::UnsupportedModpack("Missing minecraft field in manifest".to_string())
    })?;

    let mut loader = None;
    let mut loader_version = None;

    for ml in &mc_info.mod_loaders {
        let lower = ml.id.to_lowercase();
        if lower.starts_with("quilt") {
            return Err(CompError::QuiltUnsupported);
        } else if let Some(ver) = lower.strip_prefix("fabric-") {
            loader = Some("fabric".to_string());
            loader_version = Some(ver.to_string());
            break;
        } else if let Some(ver) = lower.strip_prefix("neoforge-") {
            loader = Some("neoforge".to_string());
            loader_version = Some(ver.to_string());
            break;
        } else if let Some(ver) = lower.strip_prefix("forge-") {
            loader = Some("forge".to_string());
            loader_version = Some(ver.to_string());
            break;
        } else if let Some(ver) = lower.strip_prefix("cleanroom-") {
            loader = Some("cleanroom".to_string());
            loader_version = Some(ver.to_string());
            break;
        } else if let Some(ver) = lower.strip_prefix("optifine-") {
            loader = Some("optifine".to_string());
            loader_version = Some(ver.to_string());
            break;
        }
    }

    let mut files = Vec::new();
    for f in manifest.files {
        if f.required == Some(false) {
            continue;
        }
        files.push(ModpackFileDownload {
            rel_path: String::new(),
            urls: Vec::new(),
            sha1: None,
            sha512: None,
            size: None,
            cf_project_id: Some(f.project_id),
            cf_file_id: Some(f.file_id),
        });
    }

    let overrides_folder = manifest
        .overrides
        .unwrap_or_else(|| "overrides".to_string());
    let normalized_overrides = overrides_folder.trim_matches('/').to_string();
    let override_prefix = if normalized_overrides.is_empty() {
        root_prefix.to_string()
    } else {
        format!("{root_prefix}{normalized_overrides}/")
    };

    let fallback_name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("CurseForgePack")
        .to_string();

    let name = manifest.name.unwrap_or(fallback_name);

    let inspect = ModpackInspectResult {
        format: ModpackFormat::Curseforge,
        name,
        version: manifest.version.unwrap_or_else(|| "1.0.0".to_string()),
        author: manifest.author,
        mc_version: mc_info.version,
        loader,
        loader_version,
        file_count: files.len() as u32,
    };

    Ok(ModpackInstallPlan {
        inspect,
        files,
        override_prefixes: vec![override_prefix],
        nested_archive_entry: None,
    })
}

// ----------------------------------------------------------------------------
// MultiMC / Prism inspection
// ----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MmcPack {
    #[serde(default)]
    components: Vec<MmcComponent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MmcComponent {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    version: String,
}

fn inspect_prism(
    zip: &mut zip::ZipArchive<std::fs::File>,
    entry_name: &str,
    root_prefix: &str,
    entry_names: &[String],
    archive_path: &Path,
) -> Result<ModpackInstallPlan, CompError> {
    let pack: MmcPack = {
        let mut entry = zip.by_name(entry_name).map_err(CompError::Zip)?;
        let mut content = String::new();
        entry.read_to_string(&mut content).map_err(CompError::Io)?;
        serde_json::from_str(&content).map_err(CompError::Json)?
    };

    let mut mc_version = "unknown".to_string();
    let mut loader = None;
    let mut loader_version = None;

    for comp in pack.components {
        let uid = comp.uid.to_lowercase();
        if uid.contains("quilt") {
            return Err(CompError::QuiltUnsupported);
        } else if uid == "net.minecraft" {
            mc_version = comp.version;
        } else if uid == "net.fabricmc.fabric-loader" {
            loader = Some("fabric".to_string());
            loader_version = Some(comp.version);
        } else if uid == "net.minecraftforge" {
            loader = Some("forge".to_string());
            loader_version = Some(comp.version);
        } else if uid == "net.neoforged" || uid == "net.neoforged.neoforge" {
            loader = Some("neoforge".to_string());
            loader_version = Some(comp.version);
        }
    }

    // Try reading name from instance.cfg
    let mut name = None;
    let cfg_candidates = [
        format!("{root_prefix}instance.cfg"),
        "instance.cfg".to_string(),
        format!("{root_prefix}.minecraft/instance.cfg"),
        format!("{root_prefix}minecraft/instance.cfg"),
    ];
    for cfg_path in &cfg_candidates {
        if let Ok(mut cfg_entry) = zip.by_name(cfg_path) {
            let mut cfg_content = String::new();
            if cfg_entry.read_to_string(&mut cfg_content).is_ok() {
                for line in cfg_content.lines() {
                    let trimmed = line.trim();
                    if let Some(val) = trimmed.strip_prefix("name=") {
                        let parsed = val.trim();
                        if !parsed.is_empty() {
                            name = Some(parsed.to_string());
                            break;
                        }
                    }
                }
            }
            if name.is_some() {
                break;
            }
        }
    }

    let fallback_name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("PrismPack")
        .to_string();
    let final_name = name.unwrap_or(fallback_name);

    let override_prefixes = vec![
        format!("{root_prefix}.minecraft/"),
        format!("{root_prefix}minecraft/"),
    ];

    // Count files in override directories
    let file_count = entry_names
        .iter()
        .filter(|n| override_prefixes.iter().any(|pfx| n.starts_with(pfx)) && !n.ends_with('/'))
        .count() as u32;

    let inspect = ModpackInspectResult {
        format: ModpackFormat::Prism,
        name: final_name,
        version: "1.0.0".to_string(),
        author: None,
        mc_version,
        loader,
        loader_version,
        file_count,
    };

    Ok(ModpackInstallPlan {
        inspect,
        files: Vec::new(),
        override_prefixes,
        nested_archive_entry: None,
    })
}

// ----------------------------------------------------------------------------
// MCBBS inspection
// ----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McbbsPackmeta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    addons: Vec<McbbsAddon>,
    #[serde(default)]
    files: Vec<McbbsFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McbbsAddon {
    id: String,
    #[serde(default)]
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McbbsFile {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    sha1: Option<String>,
}

fn inspect_mcbbs(
    zip: &mut zip::ZipArchive<std::fs::File>,
    entry_name: &str,
    root_prefix: &str,
    archive_path: &Path,
) -> Result<ModpackInstallPlan, CompError> {
    let mut entry = zip.by_name(entry_name).map_err(CompError::Zip)?;
    let mut content = String::new();
    entry.read_to_string(&mut content).map_err(CompError::Io)?;
    let meta: McbbsPackmeta = serde_json::from_str(&content).map_err(CompError::Json)?;

    let mut mc_version = "unknown".to_string();
    let mut loader = None;
    let mut loader_version = None;

    for addon in meta.addons {
        let id_lower = addon.id.to_lowercase();
        if id_lower == "quilt" {
            return Err(CompError::QuiltUnsupported);
        } else if id_lower == "game" {
            mc_version = addon.version;
        } else if id_lower == "fabric" || id_lower == "forge" || id_lower == "neoforge" {
            loader = Some(id_lower);
            loader_version = Some(addon.version);
        }
    }

    let mut files = Vec::new();
    for f in meta.files {
        if let (Some(path), Some(url)) = (f.path, f.url) {
            files.push(ModpackFileDownload {
                rel_path: path,
                urls: vec![url],
                sha1: f.sha1,
                sha512: None,
                size: None,
                cf_project_id: None,
                cf_file_id: None,
            });
        }
    }

    let fallback_name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("McbbsPack")
        .to_string();

    let inspect = ModpackInspectResult {
        format: ModpackFormat::Mcbbs,
        name: meta.name.unwrap_or(fallback_name),
        version: meta.version.unwrap_or_else(|| "1.0.0".to_string()),
        author: meta.author,
        mc_version,
        loader,
        loader_version,
        file_count: files.len() as u32,
    };

    Ok(ModpackInstallPlan {
        inspect,
        files,
        override_prefixes: vec![format!("{root_prefix}overrides/")],
        nested_archive_entry: None,
    })
}

fn inspect_mcbbs_from_manifest(
    manifest: &CfManifest,
    root_prefix: &str,
    archive_path: &Path,
) -> Result<ModpackInstallPlan, CompError> {
    let mut mc_version = "unknown".to_string();
    let mut loader = None;
    let mut loader_version = None;

    if let Some(arr) = manifest.addons.as_ref().and_then(|v| v.as_array()) {
        for item in arr {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let ver = item
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if id == "quilt" {
                return Err(CompError::QuiltUnsupported);
            } else if id == "game" {
                mc_version = ver;
            } else if id == "fabric" || id == "forge" || id == "neoforge" {
                loader = Some(id);
                loader_version = Some(ver);
            }
        }
    }

    let fallback_name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("McbbsPack")
        .to_string();

    let inspect = ModpackInspectResult {
        format: ModpackFormat::Mcbbs,
        name: manifest.name.clone().unwrap_or(fallback_name),
        version: manifest
            .version
            .clone()
            .unwrap_or_else(|| "1.0.0".to_string()),
        author: manifest.author.clone(),
        mc_version,
        loader,
        loader_version,
        file_count: manifest.files.len() as u32,
    };

    Ok(ModpackInstallPlan {
        inspect,
        files: Vec::new(),
        override_prefixes: vec![format!("{root_prefix}overrides/")],
        nested_archive_entry: None,
    })
}

// ----------------------------------------------------------------------------
// HMCL inspection
// ----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HmclModpack {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    loaders: Vec<HmclLoader>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HmclLoader {
    #[serde(rename = "type", default)]
    loader_type: String,
    #[serde(default)]
    version: String,
}

fn inspect_hmcl(
    zip: &mut zip::ZipArchive<std::fs::File>,
    entry_name: &str,
    root_prefix: &str,
    archive_path: &Path,
) -> Result<ModpackInstallPlan, CompError> {
    let mut entry = zip.by_name(entry_name).map_err(CompError::Zip)?;
    let mut content = String::new();
    entry.read_to_string(&mut content).map_err(CompError::Io)?;
    let pack: HmclModpack = serde_json::from_str(&content).map_err(CompError::Json)?;

    let mut loader = None;
    let mut loader_version = None;

    for l in pack.loaders {
        let t = l.loader_type.to_lowercase();
        if t == "quilt" {
            return Err(CompError::QuiltUnsupported);
        } else if t == "fabric" || t == "forge" || t == "neoforge" {
            loader = Some(t);
            loader_version = Some(l.version);
        }
    }

    let fallback_name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("HmclPack")
        .to_string();

    let inspect = ModpackInspectResult {
        format: ModpackFormat::Hmcl,
        name: pack.name.unwrap_or(fallback_name),
        version: pack.version.unwrap_or_else(|| "1.0.0".to_string()),
        author: pack.author,
        mc_version: pack.game_version.unwrap_or_else(|| "unknown".to_string()),
        loader,
        loader_version,
        file_count: 0,
    };

    Ok(ModpackInstallPlan {
        inspect,
        files: Vec::new(),
        override_prefixes: vec![
            format!("{root_prefix}minecraft/"),
            format!("{root_prefix}.minecraft/"),
            format!("{root_prefix}overrides/"),
            root_prefix.to_string(),
        ],
        nested_archive_entry: None,
    })
}

// ----------------------------------------------------------------------------
// Nested archive inspection
// ----------------------------------------------------------------------------

fn inspect_nested(
    zip: &mut zip::ZipArchive<std::fs::File>,
    inner_entry_name: &str,
    _archive_path: &Path,
) -> Result<ModpackInstallPlan, CompError> {
    let mut inner = zip.by_name(inner_entry_name).map_err(CompError::Zip)?;
    let mut temp = tempfile::Builder::new()
        .suffix(".zip")
        .tempfile()
        .map_err(CompError::Io)?;
    std::io::copy(&mut inner, &mut temp).map_err(CompError::Io)?;

    let mut plan = inspect_modpack(temp.path())?;
    plan.inspect.format = ModpackFormat::Nested;
    plan.nested_archive_entry = Some(inner_entry_name.to_string());
    Ok(plan)
}

// ----------------------------------------------------------------------------
// Raw .minecraft zip inspection
// ----------------------------------------------------------------------------

fn try_inspect_raw_minecraft(
    zip: &mut zip::ZipArchive<std::fs::File>,
    entry_names: &[String],
    archive_path: &Path,
) -> Result<Option<ModpackInstallPlan>, CompError> {
    // Look for entries matching `versions/<id>/<id>.json`
    let mut version_json_entry = None;
    let mut root_prefix = String::new();

    for name in entry_names {
        if name.ends_with(".json") && name.contains("versions/") {
            let parts: Vec<&str> = name.split('/').collect();
            if let Some(pos) = parts.iter().position(|&p| p == "versions") {
                let matches_structure = parts.len() == pos + 3
                    && parts[pos + 1] == parts[pos + 2].trim_end_matches(".json");
                if matches_structure {
                    version_json_entry = Some(name.clone());
                    if pos > 0 {
                        root_prefix = parts[0..pos].join("/") + "/";
                    }
                    break;
                }
            }
        }
    }

    if let Some(v_entry) = version_json_entry {
        let mut entry = zip.by_name(&v_entry).map_err(CompError::Zip)?;
        let mut content = String::new();
        entry.read_to_string(&mut content).map_err(CompError::Io)?;

        if let Ok(vjson) = VersionJson::parse(&content) {
            let fallback_id = archive_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("RawInstance");
            let mc_version = extract_mc_version(&vjson, fallback_id);
            let (_kind, loaders) = classify_instance(&vjson, &mc_version);
            let loader = loaders.first().cloned();

            let override_prefixes = if root_prefix.is_empty() {
                vec!["".to_string()]
            } else {
                vec![root_prefix.clone()]
            };

            let file_count = entry_names.iter().filter(|n| !n.ends_with('/')).count() as u32;

            let inspect = ModpackInspectResult {
                format: ModpackFormat::RawMinecraft,
                name: fallback_id.to_string(),
                version: "1.0.0".to_string(),
                author: None,
                mc_version,
                loader,
                loader_version: None,
                file_count,
            };

            return Ok(Some(ModpackInstallPlan {
                inspect,
                files: Vec::new(),
                override_prefixes,
                nested_archive_entry: None,
            }));
        }
    }

    // Secondary check: presence of "versions/" and "mods/"
    let has_versions = entry_names.iter().any(|n| n.contains("versions/"));
    let has_mods = entry_names.iter().any(|n| n.contains("mods/"));
    if has_versions || has_mods {
        let fallback_id = archive_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("RawInstance")
            .to_string();

        let override_prefixes = if entry_names.iter().any(|n| n.starts_with(".minecraft/")) {
            vec![".minecraft/".to_string()]
        } else if entry_names.iter().any(|n| n.starts_with("minecraft/")) {
            vec!["minecraft/".to_string()]
        } else {
            vec!["".to_string()]
        };

        let file_count = entry_names.iter().filter(|n| !n.ends_with('/')).count() as u32;

        let inspect = ModpackInspectResult {
            format: ModpackFormat::RawMinecraft,
            name: fallback_id,
            version: "1.0.0".to_string(),
            author: None,
            mc_version: "unknown".to_string(),
            loader: None,
            loader_version: None,
            file_count,
        };

        return Ok(Some(ModpackInstallPlan {
            inspect,
            files: Vec::new(),
            override_prefixes,
            nested_archive_entry: None,
        }));
    }

    Ok(None)
}

// ============================================================================
// Override extraction
// ============================================================================

/// Extracts all entries matching `override_prefixes` into `instance_dir`.
/// Prevents path traversal vulnerabilities (`..` and prefix components).
pub fn extract_modpack_overrides(
    archive_path: &Path,
    instance_dir: &Path,
    override_prefixes: &[String],
) -> Result<usize, CompError> {
    let file = std::fs::File::open(archive_path).map_err(CompError::Io)?;
    let mut zip = zip::ZipArchive::new(file).map_err(CompError::Zip)?;
    let mut extracted_count = 0;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(CompError::Zip)?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().replace('\\', "/");

        for prefix in override_prefixes {
            let normalized_prefix = prefix.replace('\\', "/");
            let matched = if normalized_prefix.is_empty() {
                Some(name.as_str())
            } else {
                name.strip_prefix(&normalized_prefix)
            };

            if let Some(rel) = matched {
                let rel = rel.trim_start_matches('/');
                if rel.is_empty() {
                    continue;
                }

                let rel_path = Path::new(rel);
                // Security: Prevent path traversal
                if rel_path.components().any(|c| {
                    matches!(
                        c,
                        std::path::Component::ParentDir | std::path::Component::Prefix(_)
                    )
                }) {
                    debug!("Skipping unsafe path traversal entry: {}", rel);
                    continue;
                }

                let target_path = instance_dir.join(rel_path);

                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent).map_err(CompError::Io)?;
                }

                let mut out = std::fs::File::create(&target_path).map_err(CompError::Io)?;
                std::io::copy(&mut entry, &mut out).map_err(CompError::Io)?;
                extracted_count += 1;
                break;
            }
        }
    }

    Ok(extracted_count)
}

// ============================================================================
// Modpack Import
// ============================================================================

fn cf_file_destination_folder(file_name: &str) -> &'static str {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".jar") {
        "mods"
    } else if lower.contains("shader") {
        "shaderpacks"
    } else if lower.contains("resource") || lower.contains("texture") {
        "resourcepacks"
    } else {
        "mods"
    }
}

/// Imports a modpack into `<mc_folder>/versions/<instance_name>`.
///
/// Steps:
/// 1. Inspect archive (rejecting Quilt).
/// 2. If nested, extracts inner archive.
/// 3. Installs base game (with loader if specified; or stub version JSON if `install_base_game` is false).
/// 4. Forces `isolation = "on"` in `<instance_dir>/mclr.instance.json`.
/// 5. Resolves CurseForge file IDs if present.
/// 6. Downloads all files via Downloader.
/// 7. Extracts overrides into `instance_dir`.
#[allow(clippy::too_many_arguments)]
pub async fn import_modpack(
    archive_path: &Path,
    instance_name: &str,
    mc_folder: &Path,
    client: &reqwest::Client,
    net_settings: &mclr_core::NetSettings,
    java_settings: Option<&mclr_core::JavaSettings>,
    cf_client: Option<&CurseForgeClient>,
    task_ctx: Option<(&TaskCenter, &str)>,
    install_base_game: bool,
) -> Result<InstanceInfo, CompError> {
    validate_instance_name(instance_name).map_err(CompError::Game)?;

    let mut plan = inspect_modpack(archive_path)?;

    // If nested archive, extract it and work with the inner archive
    let (effective_archive_path, _temp_guard) =
        if let Some(nested_entry) = &plan.nested_archive_entry {
            let file = std::fs::File::open(archive_path).map_err(CompError::Io)?;
            let mut zip = zip::ZipArchive::new(file).map_err(CompError::Zip)?;
            let mut inner = zip.by_name(nested_entry).map_err(CompError::Zip)?;
            let mut temp = tempfile::Builder::new()
                .suffix(".zip")
                .tempfile()
                .map_err(CompError::Io)?;
            std::io::copy(&mut inner, &mut temp).map_err(CompError::Io)?;
            let path = temp.path().to_path_buf();
            (path, Some(temp))
        } else {
            (archive_path.to_path_buf(), None)
        };

    let instance_dir = mc_folder.join("versions").join(instance_name);
    std::fs::create_dir_all(&instance_dir).map_err(CompError::Io)?;

    // 1. Base game / loader installation
    if install_base_game {
        if let Some(loader_name) = &plan.inspect.loader {
            let loader_kind = match loader_name.to_lowercase().as_str() {
                "fabric" => Some(LoaderKind::Fabric),
                "legacyfabric" => Some(LoaderKind::LegacyFabric),
                "liteloader" => Some(LoaderKind::LiteLoader),
                "labymod" => Some(LoaderKind::LabyMod),
                "optifine" => Some(LoaderKind::OptiFine),
                "forge" => Some(LoaderKind::Forge),
                "neoforge" => Some(LoaderKind::NeoForge),
                "cleanroom" => Some(LoaderKind::Cleanroom),
                "quilt" => return Err(CompError::QuiltUnsupported),
                _ => None,
            };

            if let Some(kind) = loader_kind {
                let default_java = mclr_core::JavaSettings::default();
                let java_cfg = java_settings.unwrap_or(&default_java);
                let ctx = LoaderInstallContext {
                    client,
                    net_settings,
                    java_settings: java_cfg,
                    mc_folder: mc_folder.to_path_buf(),
                    mc_version: plan.inspect.mc_version.clone(),
                    instance_name: instance_name.to_string(),
                    loader_version: plan.inspect.loader_version.clone().unwrap_or_default(),
                    optifine_version: None,
                    task_ctx,
                };
                install_with_loader(kind, ctx).await?;
            } else {
                mclr_game::install::install_vanilla(
                    client,
                    net_settings,
                    mc_folder,
                    &plan.inspect.mc_version,
                    instance_name,
                    task_ctx,
                )
                .await
                .map_err(CompError::Game)?;
            }
        } else {
            mclr_game::install::install_vanilla(
                client,
                net_settings,
                mc_folder,
                &plan.inspect.mc_version,
                instance_name,
                task_ctx,
            )
            .await
            .map_err(CompError::Game)?;
        }
    } else {
        // Offline / fixture mode: write minimal valid instance version JSON if missing
        let json_path = instance_dir.join(format!("{instance_name}.json"));
        if !json_path.exists() {
            let dummy_json = serde_json::json!({
                "id": instance_name,
                "time": "2026-01-01T00:00:00+00:00",
                "releaseTime": "2026-01-01T00:00:00+00:00",
                "type": "release",
                "mainClass": "net.minecraft.client.main.Main",
                "inheritsFrom": plan.inspect.mc_version,
            });
            std::fs::write(&json_path, serde_json::to_string_pretty(&dummy_json)?)
                .map_err(CompError::Io)?;
        }
    }

    // 2. Always force isolation = "on" in mclr.instance.json
    let config_path = instance_dir.join("mclr.instance.json");
    let mut cfg = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|c| serde_json::from_str::<InstanceConfig>(&c).ok())
            .unwrap_or_default()
    } else {
        InstanceConfig::default()
    };
    cfg.isolation = "on".to_string();
    std::fs::write(&config_path, serde_json::to_string_pretty(&cfg)?).map_err(CompError::Io)?;

    // 3. Resolve CurseForge file IDs if present
    let cf_file_ids: Vec<u64> = plan.files.iter().filter_map(|f| f.cf_file_id).collect();
    if !cf_file_ids.is_empty() {
        let cf = cf_client.ok_or(CompError::NotConfigured)?;
        let resolved_files = cf.get_files_batch(&cf_file_ids).await?;

        let mut resolved_map = HashMap::new();
        for rf in resolved_files {
            if let Ok(fid) = rf.id.parse::<u64>() {
                resolved_map.insert(fid, rf);
            }
        }

        for file in &mut plan.files {
            if let Some(rf) = file.cf_file_id.and_then(|fid| resolved_map.get(&fid)) {
                let subfolder = cf_file_destination_folder(&rf.file_name);
                file.rel_path = format!("{subfolder}/{}", rf.file_name);
                file.urls = rf.download_urls.clone();
                file.sha1 = rf.hashes.sha1.clone();
                file.size = Some(rf.size);
            }
        }
    }

    // 4. Download modpack files
    let mut specs = Vec::new();
    for file in &plan.files {
        if file.rel_path.is_empty() {
            continue;
        }
        let dest = instance_dir.join(&file.rel_path);
        let urls: Vec<Url> = file
            .urls
            .iter()
            .filter_map(|u| Url::parse(u).ok())
            .collect();
        if urls.is_empty() {
            continue;
        }
        specs.push(DownloadSpec {
            urls,
            dest,
            size: file.size,
            sha1: file.sha1.clone(),
            sha256: None,
            md5: None,
        });
    }

    if !specs.is_empty() {
        info!(
            "Downloading {} modpack files into {}",
            specs.len(),
            instance_dir.display()
        );
        let downloader = Downloader::new(client.clone(), net_settings.clone());
        downloader
            .download_many(specs, task_ctx)
            .await
            .map_err(CompError::Net)?;
    }

    // 5. Extract overrides into instance_dir
    let extracted = extract_modpack_overrides(
        &effective_archive_path,
        &instance_dir,
        &plan.override_prefixes,
    )?;
    info!(
        "Extracted {extracted} override files into {}",
        instance_dir.display()
    );

    // 6. Return refreshed InstanceInfo
    let instances = scan_instances(mc_folder).map_err(CompError::Game)?;
    if let Some(info) = instances.into_iter().find(|i| i.id == instance_name) {
        Ok(info)
    } else {
        Ok(InstanceInfo {
            id: instance_name.to_string(),
            name: instance_name.to_string(),
            folder: mc_folder.to_string_lossy().to_string(),
            path: instance_dir.to_string_lossy().to_string(),
            mc_version: plan.inspect.mc_version,
            kind: InstanceKind::Vanilla,
            loaders: plan.inspect.loader.into_iter().collect(),
            state: InstanceState::Ok,
            config: cfg,
        })
    }
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

        let info = import_modpack(
            zip_file.path(),
            "cf-pack-instance",
            mc_dir.path(),
            &client,
            &net_settings,
            None,
            Some(&cf_client),
            None,
            false,
        )
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

        let info = import_modpack(
            zip_file.path(),
            "mr-pack-instance",
            mc_dir.path(),
            &client,
            &net_settings,
            None,
            None,
            None,
            false,
        )
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

        let info = import_modpack(
            zip_file.path(),
            "prism-instance",
            mc_dir.path(),
            &client,
            &net_settings,
            None,
            None,
            None,
            false,
        )
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
