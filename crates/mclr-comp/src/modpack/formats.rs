//! Modpack format inspection (Modrinth, CurseForge, MultiMC/Prism, MCBBS, HMCL, Nested, Raw).

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use mclr_game::instance::{classify_instance, extract_mc_version};
use mclr_game::version::VersionJson;
use serde::Deserialize;

use super::{
    ModpackFileDownload, ModpackFormat, ModpackInspectResult, ModpackInstallPlan, inspect_modpack,
};
use crate::error::CompError;

pub(crate) fn find_entry_name<'a>(names: &'a [String], target: &str) -> Option<(&'a str, String)> {
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

pub(crate) fn inspect_modrinth(
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
pub(crate) struct CfManifest {
    #[serde(default)]
    pub(crate) minecraft: Option<CfMinecraft>,
    #[serde(default)]
    _manifest_type: Option<String>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) version: Option<String>,
    #[serde(default)]
    pub(crate) author: Option<String>,
    #[serde(default)]
    pub(crate) files: Vec<CfManifestFile>,
    #[serde(default)]
    pub(crate) overrides: Option<String>,
    #[serde(default)]
    pub(crate) addons: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CfMinecraft {
    #[serde(default)]
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) mod_loaders: Vec<CfModLoader>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CfModLoader {
    pub(crate) id: String,
    #[serde(default)]
    _primary: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CfManifestFile {
    #[serde(rename = "projectID")]
    pub(crate) project_id: u64,
    #[serde(rename = "fileID")]
    pub(crate) file_id: u64,
    #[serde(default = "default_true")]
    pub(crate) required: Option<bool>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

pub(crate) fn inspect_manifest(
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

pub(crate) fn inspect_prism(
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

pub(crate) fn inspect_mcbbs(
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

pub(crate) fn inspect_mcbbs_from_manifest(
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

    let overrides_folder = manifest
        .overrides
        .clone()
        .unwrap_or_else(|| "overrides".to_string());
    let normalized_overrides = overrides_folder.trim_matches('/').to_string();
    let override_prefix = if normalized_overrides.is_empty() {
        root_prefix.to_string()
    } else {
        format!("{root_prefix}{normalized_overrides}/")
    };

    Ok(ModpackInstallPlan {
        inspect,
        files: Vec::new(),
        override_prefixes: vec![override_prefix],
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

pub(crate) fn inspect_hmcl(
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

pub(crate) fn inspect_nested(
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

pub(crate) fn try_inspect_raw_minecraft(
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
