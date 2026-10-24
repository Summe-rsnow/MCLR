//! Modpack export functionality (.mrpack for Modrinth and .zip for CurseForge).

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use hex;
use mclr_core::TaskCenter;
use mclr_game::instance::{classify_instance, extract_mc_version};
use mclr_game::version::VersionJson;
use serde::{Deserialize, Serialize};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha512;
use specta::Type;
use tracing::{debug, info};
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::curseforge::CurseForgeClient;
use crate::error::CompError;
use crate::local::compute_curseforge_murmur2;
use crate::modrinth::ModrinthClient;

// ============================================================================
// Types
// ============================================================================

/// Candidate file or folder that can be selected for modpack export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportCandidateEntry {
    pub rel_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub default_selected: bool,
}

/// Target export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ModpackExportFormat {
    Modrinth,
    Curseforge,
}

/// Options configured by the user for modpack export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModpackExportOptions {
    pub instance_id: String,
    pub format: ModpackExportFormat,
    pub pack_name: String,
    pub pack_version: String,
    pub author: String,
    pub summary: String,
    pub selected_paths: Vec<String>,
    pub output_path: String,
}

// ============================================================================
// Scanning Candidates
// ============================================================================

/// Checks if a file or folder name matches the exclusion rules.
fn is_excluded_name(name: &str, instance_id: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.starts_with(".mclr")
        || lower.starts_with(".fabric")
        || lower.starts_with(".quilt")
        || lower.starts_with(".git")
    {
        return true;
    }
    if lower == "mclr.instance.json" || lower == "mclr-launch.args" {
        return true;
    }
    if lower == "saves"
        || lower == "logs"
        || lower == "crash-reports"
        || lower == "screenshots"
        || lower == "backups"
    {
        return true;
    }
    // Instance jar / json / natives
    let id_lower = instance_id.to_lowercase();
    if lower == format!("{id_lower}.json")
        || lower == format!("{id_lower}.jar")
        || lower == format!("{id_lower}-natives")
    {
        return true;
    }
    false
}

/// Scans top-level files/folders and specific subdirectories in `instance_dir`
/// to provide candidate files and directories for modpack export.
pub fn scan_export_candidates(instance_dir: &Path) -> Result<Vec<ExportCandidateEntry>, CompError> {
    if !instance_dir.is_dir() {
        return Err(CompError::FileNotFound(instance_dir.display().to_string()));
    }

    let instance_id = instance_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let mut entries = Vec::new();

    let rd = std::fs::read_dir(instance_dir).map_err(CompError::Io)?;

    // We scan top-level entries
    for entry_res in rd {
        let entry = entry_res.map_err(CompError::Io)?;
        let name_os = entry.file_name();
        let name = name_os.to_string_lossy().to_string();
        let path = entry.path();
        let file_type = entry.file_type().map_err(CompError::Io)?;

        if is_excluded_name(&name, instance_id) {
            // Add as default_selected = false
            let is_dir = file_type.is_dir();
            let size = if is_dir {
                0
            } else {
                entry.metadata().map(|m| m.len()).unwrap_or(0)
            };
            entries.push(ExportCandidateEntry {
                rel_path: name,
                is_dir,
                size,
                default_selected: false,
            });
            continue;
        }

        let lower = name.to_lowercase();
        // Top-level known include folders: we can recurse into these
        let is_deep_folder = lower == "mods"
            || lower == "config"
            || lower == "resourcepacks"
            || lower == "shaderpacks"
            || lower == "scripts"
            || lower == "kubejs";

        if file_type.is_dir() {
            if is_deep_folder {
                // Scan files inside this folder recursively
                scan_dir_recursive(&path, &name, &mut entries)?;
            } else {
                entries.push(ExportCandidateEntry {
                    rel_path: name,
                    is_dir: true,
                    size: 0,
                    default_selected: false,
                });
            }
        } else if file_type.is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let default_selected = lower == "options.txt"
                || lower == "servers.dat"
                || lower == "optionsof.txt"
                || lower == "optionsshaders.txt";

            entries.push(ExportCandidateEntry {
                rel_path: name,
                is_dir: false,
                size,
                default_selected,
            });
        }
    }

    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(entries)
}

fn scan_dir_recursive(
    dir: &Path,
    rel_prefix: &str,
    out: &mut Vec<ExportCandidateEntry>,
) -> Result<(), CompError> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry_res in std::fs::read_dir(dir).map_err(CompError::Io)? {
        let entry = entry_res.map_err(CompError::Io)?;
        let name_os = entry.file_name();
        let name = name_os.to_string_lossy();
        let path = entry.path();
        let rel_path = format!("{rel_prefix}/{name}");
        let file_type = entry.file_type().map_err(CompError::Io)?;

        if file_type.is_dir() {
            scan_dir_recursive(&path, &rel_path, out)?;
        } else if file_type.is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            // By default, files inside mods, config, resourcepacks, shaderpacks, scripts, kubejs are selected
            out.push(ExportCandidateEntry {
                rel_path,
                is_dir: false,
                size,
                default_selected: true,
            });
        }
    }
    Ok(())
}

// ============================================================================
// Modpack Export
// ============================================================================

/// Reads version JSON and extracts (mc_version, loader, loader_version).
fn read_instance_version(
    instance_dir: &Path,
    instance_id: &str,
) -> Result<(String, Option<String>, Option<String>), CompError> {
    let json_path = instance_dir.join(format!("{instance_id}.json"));
    if !json_path.exists() {
        return Err(CompError::FileNotFound(json_path.display().to_string()));
    }
    let content = std::fs::read_to_string(&json_path).map_err(CompError::Io)?;
    let version_json = VersionJson::parse(&content)?;
    let mc_version = extract_mc_version(&version_json, instance_id);
    let (_kind, loaders) = classify_instance(&version_json, &mc_version);

    let mut loader = None;
    let mut loader_version = None;

    for l in loaders {
        let parts: Vec<&str> = l.split_whitespace().collect();
        if let Some(&first) = parts.first() {
            let lower = first.to_lowercase();
            if lower == "fabric" || lower == "neoforge" || lower == "forge" {
                loader = Some(lower);
                if parts.len() > 1 {
                    loader_version = Some(parts[1].to_string());
                }
                break;
            }
        }
    }

    Ok((mc_version, loader, loader_version))
}

fn compute_hashes(data: &[u8]) -> (String, String) {
    let mut sha1_hasher = Sha1::new();
    sha1_hasher.update(data);
    let sha1 = hex::encode(sha1_hasher.finalize());

    let mut sha512_hasher = Sha512::new();
    sha512_hasher.update(data);
    let sha512 = hex::encode(sha512_hasher.finalize());

    (sha1, sha512)
}

/// Modrinth index file schema structs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModrinthExportIndex {
    format_version: u32,
    game: String,
    version_id: String,
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    summary: String,
    files: Vec<ModrinthExportFile>,
    dependencies: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModrinthExportFile {
    path: String,
    hashes: HashMap<String, String>,
    env: ModrinthExportEnv,
    downloads: Vec<String>,
    file_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModrinthExportEnv {
    client: String,
    server: String,
}

/// CurseForge manifest schema structs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfExportManifest {
    minecraft: CfExportMinecraft,
    manifest_type: String,
    manifest_version: u32,
    name: String,
    version: String,
    author: String,
    files: Vec<CfExportFile>,
    overrides: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfExportMinecraft {
    version: String,
    mod_loaders: Vec<CfExportModLoader>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfExportModLoader {
    id: String,
    primary: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfExportFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    required: bool,
}

/// Collects all individual file paths relative to `instance_dir` based on `selected_paths`.
/// A selected path can be a file or a folder (which includes all files recursively).
fn collect_selected_files(
    instance_dir: &Path,
    selected_paths: &[String],
) -> Result<Vec<String>, CompError> {
    let mut files = Vec::new();

    for rel in selected_paths {
        let normalized = rel.replace('\\', "/");
        let path = instance_dir.join(&normalized);
        if !path.exists() {
            continue;
        }

        if path.is_file() {
            if !files.contains(&normalized) {
                files.push(normalized);
            }
        } else if path.is_dir() {
            collect_files_in_dir_recursive(&path, &normalized, &mut files)?;
        }
    }

    files.sort();
    Ok(files)
}

fn collect_files_in_dir_recursive(
    dir: &Path,
    prefix: &str,
    out: &mut Vec<String>,
) -> Result<(), CompError> {
    for entry_res in std::fs::read_dir(dir).map_err(CompError::Io)? {
        let entry = entry_res.map_err(CompError::Io)?;
        let name_os = entry.file_name();
        let name = name_os.to_string_lossy();
        let path = entry.path();
        let rel = format!("{prefix}/{name}");

        if path.is_dir() {
            collect_files_in_dir_recursive(&path, &rel, out)?;
        } else if path.is_file() && !out.contains(&rel) {
            out.push(rel);
        }
    }
    Ok(())
}

/// Exports a Minecraft instance into a modpack archive (.mrpack or .zip).
pub async fn export_modpack(
    instance_dir: &Path,
    options: &ModpackExportOptions,
    modrinth_client: Option<&ModrinthClient>,
    curseforge_client: Option<&CurseForgeClient>,
    task_info: Option<(&TaskCenter, &str)>,
) -> Result<PathBuf, CompError> {
    info!(
        "Starting modpack export for instance '{}' (format: {:?})",
        options.instance_id, options.format
    );

    if let Some((tc, tid)) = task_info {
        let _ = tc.update_progress(tid, 0.05, 0).await;
    }

    // 1. Determine MC version and modloader
    let (mc_version, loader, loader_version) =
        read_instance_version(instance_dir, &options.instance_id)?;

    // 2. Collect all selected files
    let selected_files = collect_selected_files(instance_dir, &options.selected_paths)?;

    if let Some((tc, tid)) = task_info {
        let _ = tc.update_progress(tid, 0.15, 0).await;
    }

    let out_path = PathBuf::from(&options.output_path);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(CompError::Io)?;
    }

    let out_file = File::create(&out_path).map_err(CompError::Io)?;
    let mut zip = ZipWriter::new(out_file);
    let zip_options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    match options.format {
        ModpackExportFormat::Modrinth => {
            export_as_modrinth(
                instance_dir,
                options,
                &mc_version,
                loader.as_deref(),
                loader_version.as_deref(),
                &selected_files,
                modrinth_client,
                &mut zip,
                zip_options,
                task_info,
            )
            .await?;
        }
        ModpackExportFormat::Curseforge => {
            export_as_curseforge(
                instance_dir,
                options,
                &mc_version,
                loader.as_deref(),
                loader_version.as_deref(),
                &selected_files,
                curseforge_client,
                &mut zip,
                zip_options,
                task_info,
            )
            .await?;
        }
    }

    zip.finish().map_err(CompError::Zip)?;

    if let Some((tc, tid)) = task_info {
        let _ = tc.update_progress(tid, 1.0, 0).await;
    }

    info!("Modpack exported successfully to {}", out_path.display());
    Ok(out_path)
}

#[allow(clippy::too_many_arguments)]
async fn export_as_modrinth<W: Write + std::io::Seek>(
    instance_dir: &Path,
    options: &ModpackExportOptions,
    mc_version: &str,
    loader: Option<&str>,
    loader_version: Option<&str>,
    selected_files: &[String],
    modrinth_client: Option<&ModrinthClient>,
    zip: &mut ZipWriter<W>,
    zip_options: SimpleFileOptions,
    task_info: Option<(&TaskCenter, &str)>,
) -> Result<(), CompError> {
    // Partition files into: candidate community files (mods/, resourcepacks/, shaderpacks/) and others
    struct CandidateFile {
        rel_path: String,
        sha1: String,
        sha512: String,
        size: u64,
        data: Vec<u8>,
    }

    let mut candidate_files = Vec::new();
    let mut other_files = Vec::new();

    for rel in selected_files {
        let full_path = instance_dir.join(rel);
        let mut file = File::open(&full_path).map_err(CompError::Io)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(CompError::Io)?;

        let is_candidate = rel.starts_with("mods/")
            || rel.starts_with("resourcepacks/")
            || rel.starts_with("shaderpacks/");

        if is_candidate && modrinth_client.is_some() {
            let (sha1, sha512) = compute_hashes(&data);
            let size = data.len() as u64;
            candidate_files.push(CandidateFile {
                rel_path: rel.clone(),
                sha1,
                sha512,
                size,
                data,
            });
        } else {
            other_files.push((rel.clone(), data));
        }
    }

    let mut resolved_index_files = Vec::new();

    // Query Modrinth if client is available
    if let Some(mr) = modrinth_client
        && !candidate_files.is_empty()
    {
        let hashes: Vec<String> = candidate_files.iter().map(|c| c.sha1.clone()).collect();
        let resolved_map = match mr.get_version_files_by_sha1(&hashes).await {
            Ok(m) => m,
            Err(e) => {
                debug!("Modrinth batch hash lookup failed: {e}; putting files to overrides");
                HashMap::new()
            }
        };

        for c in candidate_files {
            let mut added_to_index = false;
            if let Some(cf) = resolved_map.get(&c.sha1)
                && !cf.download_urls.is_empty()
            {
                let mut hashes_map = HashMap::new();
                hashes_map.insert("sha1".to_string(), c.sha1.clone());
                hashes_map.insert("sha512".to_string(), c.sha512.clone());

                resolved_index_files.push(ModrinthExportFile {
                    path: c.rel_path.clone(),
                    hashes: hashes_map,
                    env: ModrinthExportEnv {
                        client: "required".to_string(),
                        server: "required".to_string(),
                    },
                    downloads: cf.download_urls.clone(),
                    file_size: c.size,
                });
                added_to_index = true;
            }
            if !added_to_index {
                other_files.push((c.rel_path, c.data));
            }
        }
    }

    if let Some((tc, tid)) = task_info {
        let _ = tc.update_progress(tid, 0.5, 0).await;
    }

    // Write all overrides
    let total_other = other_files.len().max(1);
    for (idx, (rel, data)) in other_files.iter().enumerate() {
        let zip_path = format!("overrides/{rel}");
        zip.start_file(&zip_path, zip_options)
            .map_err(CompError::Zip)?;
        zip.write_all(data).map_err(CompError::Io)?;

        if let Some((tc, tid)) = task_info {
            let p = 0.5 + 0.35 * (idx as f32 / total_other as f32);
            let _ = tc.update_progress(tid, p, 0).await;
        }
    }

    // Prepare modrinth.index.json
    let mut dependencies = HashMap::new();
    dependencies.insert("minecraft".to_string(), mc_version.to_string());
    if let Some(l) = loader {
        let dep_key = match l {
            "fabric" => "fabric-loader",
            "neoforge" => "neoforge",
            "forge" => "forge",
            _ => l,
        };
        let dep_val = loader_version.unwrap_or("0.1.0");
        dependencies.insert(dep_key.to_string(), dep_val.to_string());
    }

    let index = ModrinthExportIndex {
        format_version: 1,
        game: "minecraft".to_string(),
        version_id: options.pack_version.clone(),
        name: options.pack_name.clone(),
        summary: options.summary.clone(),
        files: resolved_index_files,
        dependencies,
    };

    let index_bytes = serde_json::to_vec_pretty(&index).map_err(CompError::Json)?;
    zip.start_file("modrinth.index.json", zip_options)
        .map_err(CompError::Zip)?;
    zip.write_all(&index_bytes).map_err(CompError::Io)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn export_as_curseforge<W: Write + std::io::Seek>(
    instance_dir: &Path,
    options: &ModpackExportOptions,
    mc_version: &str,
    loader: Option<&str>,
    loader_version: Option<&str>,
    selected_files: &[String],
    curseforge_client: Option<&CurseForgeClient>,
    zip: &mut ZipWriter<W>,
    zip_options: SimpleFileOptions,
    task_info: Option<(&TaskCenter, &str)>,
) -> Result<(), CompError> {
    struct CandidateFile {
        rel_path: String,
        murmur2: u32,
        data: Vec<u8>,
    }

    let mut candidate_files = Vec::new();
    let mut other_files = Vec::new();

    for rel in selected_files {
        let full_path = instance_dir.join(rel);
        let mut file = File::open(&full_path).map_err(CompError::Io)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(CompError::Io)?;

        let is_candidate = rel.starts_with("mods/") && rel.ends_with(".jar");

        if is_candidate && curseforge_client.is_some() {
            let murmur2 = compute_curseforge_murmur2(&data);
            candidate_files.push(CandidateFile {
                rel_path: rel.clone(),
                murmur2,
                data,
            });
        } else {
            other_files.push((rel.clone(), data));
        }
    }

    let mut cf_manifest_files = Vec::new();

    if let Some(cf) = curseforge_client
        && !candidate_files.is_empty()
    {
        let fingerprints: Vec<u32> = candidate_files.iter().map(|c| c.murmur2).collect();
        let matched_files = match cf.match_fingerprints(&fingerprints).await {
            Ok(files) => files,
            Err(e) => {
                debug!("CurseForge fingerprint matching failed: {e}; putting files to overrides");
                Vec::new()
            }
        };

        let mut matched_map: HashMap<u32, crate::model::CompFile> = HashMap::new();
        for f in matched_files {
            if let Some(fp) = f.hashes.murmur2 {
                matched_map.insert(fp, f);
            }
        }

        for c in candidate_files {
            let mut added_to_manifest = false;
            if let Some(mf) = matched_map.get(&c.murmur2)
                && let (Ok(pid), Ok(fid)) = (mf.project_id.parse::<u64>(), mf.id.parse::<u64>())
            {
                cf_manifest_files.push(CfExportFile {
                    project_id: pid,
                    file_id: fid,
                    required: true,
                });
                added_to_manifest = true;
            }
            if !added_to_manifest {
                other_files.push((c.rel_path, c.data));
            }
        }
    }

    if let Some((tc, tid)) = task_info {
        let _ = tc.update_progress(tid, 0.5, 0).await;
    }

    // Write overrides
    let total_other = other_files.len().max(1);
    for (idx, (rel, data)) in other_files.iter().enumerate() {
        let zip_path = format!("overrides/{rel}");
        zip.start_file(&zip_path, zip_options)
            .map_err(CompError::Zip)?;
        zip.write_all(data).map_err(CompError::Io)?;

        if let Some((tc, tid)) = task_info {
            let p = 0.5 + 0.35 * (idx as f32 / total_other as f32);
            let _ = tc.update_progress(tid, p, 0).await;
        }
    }

    // Prepare manifest.json
    let mut mod_loaders = Vec::new();
    if let Some(l) = loader {
        let ver = loader_version.unwrap_or("0.1.0");
        let id = format!("{l}-{ver}");
        mod_loaders.push(CfExportModLoader { id, primary: true });
    }

    let manifest = CfExportManifest {
        minecraft: CfExportMinecraft {
            version: mc_version.to_string(),
            mod_loaders,
        },
        manifest_type: "minecraftModpack".to_string(),
        manifest_version: 1,
        name: options.pack_name.clone(),
        version: options.pack_version.clone(),
        author: options.author.clone(),
        files: cf_manifest_files,
        overrides: "overrides".to_string(),
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(CompError::Json)?;
    zip.start_file("manifest.json", zip_options)
        .map_err(CompError::Zip)?;
    zip.write_all(&manifest_bytes).map_err(CompError::Io)?;

    Ok(())
}

// ============================================================================
// Unit & Integration Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_scan_export_candidates_defaults() {
        let dir = tempdir().unwrap();
        let inst = dir.path();

        // Create files & folders
        std::fs::create_dir_all(inst.join("mods")).unwrap();
        std::fs::write(inst.join("mods/test-mod.jar"), b"mod content").unwrap();

        std::fs::create_dir_all(inst.join("config")).unwrap();
        std::fs::write(inst.join("config/test.toml"), b"key = 1").unwrap();

        std::fs::write(inst.join("options.txt"), b"fov:70").unwrap();
        std::fs::write(inst.join("servers.dat"), b"servers").unwrap();

        std::fs::create_dir_all(inst.join("saves/world1")).unwrap();
        std::fs::write(inst.join("saves/world1/level.dat"), b"world data").unwrap();

        std::fs::create_dir_all(inst.join("logs")).unwrap();
        std::fs::write(inst.join("logs/latest.log"), b"log").unwrap();

        std::fs::create_dir_all(inst.join("screenshots")).unwrap();
        std::fs::write(inst.join("screenshots/screen.png"), b"img").unwrap();

        std::fs::write(inst.join("inst1.json"), b"{}").unwrap();
        std::fs::write(inst.join("inst1.jar"), b"").unwrap();
        std::fs::write(inst.join("mclr.instance.json"), b"{}").unwrap();

        let candidates = scan_export_candidates(inst).unwrap();

        let find = |p: &str| candidates.iter().find(|c| c.rel_path == p);

        // Included by default
        assert!(find("mods/test-mod.jar").unwrap().default_selected);
        assert!(find("config/test.toml").unwrap().default_selected);
        assert!(find("options.txt").unwrap().default_selected);
        assert!(find("servers.dat").unwrap().default_selected);

        // Excluded by default
        assert!(!find("saves").unwrap().default_selected);
        assert!(!find("logs").unwrap().default_selected);
        assert!(!find("screenshots").unwrap().default_selected);
        assert!(!find("inst1.json").unwrap().default_selected);
        assert!(!find("inst1.jar").unwrap().default_selected);
        assert!(!find("mclr.instance.json").unwrap().default_selected);
    }

    #[tokio::test]
    async fn test_modpack_export_roundtrip_mrpack() {
        let server = MockServer::start().await;

        let sample_jar_bytes = b"sample fabric mod jar binary";
        let mut hasher = Sha1::new();
        hasher.update(sample_jar_bytes);
        let sample_sha1 = hex::encode(hasher.finalize());

        let version_files_json = serde_json::json!({
            &sample_sha1: {
                "id": "v100",
                "project_id": "proj100",
                "name": "sample-mod-1.0.jar",
                "files": [
                    {
                        "hashes": {
                            "sha1": &sample_sha1
                        },
                        "url": format!("{}/download/sample.jar", server.uri()),
                        "filename": "sample.jar",
                        "primary": true,
                        "size": sample_jar_bytes.len()
                    }
                ],
                "game_versions": ["1.20.1"],
                "loaders": ["fabric"],
                "version_type": "release",
                "date_published": "2023-01-01T00:00:00Z"
            }
        });

        Mock::given(method("POST"))
            .and(path("/v2/version_files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&version_files_json))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/download/sample.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(sample_jar_bytes))
            .mount(&server)
            .await;

        let inst_dir = tempdir().unwrap();
        let inst_path = inst_dir.path();
        let inst_id = "test-instance";

        // Setup instance files
        let version_json = serde_json::json!({
            "id": inst_id,
            "inheritsFrom": "1.20.1",
            "libraries": [
                {
                    "name": "net.fabricmc:fabric-loader:0.15.11"
                }
            ],
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient"
        });
        std::fs::write(
            inst_path.join(format!("{inst_id}.json")),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();

        std::fs::create_dir_all(inst_path.join("mods")).unwrap();
        std::fs::write(inst_path.join("mods/sample.jar"), sample_jar_bytes).unwrap();

        std::fs::create_dir_all(inst_path.join("config")).unwrap();
        std::fs::write(inst_path.join("config/test.toml"), b"setting = true\n").unwrap();

        std::fs::write(inst_path.join("options.txt"), b"fov:90\n").unwrap();

        // Scan candidates
        let candidates = scan_export_candidates(inst_path).unwrap();
        let selected_paths: Vec<String> = candidates
            .into_iter()
            .filter(|c| c.default_selected)
            .map(|c| c.rel_path)
            .collect();

        assert!(selected_paths.contains(&"mods/sample.jar".to_string()));
        assert!(selected_paths.contains(&"config/test.toml".to_string()));
        assert!(selected_paths.contains(&"options.txt".to_string()));

        let export_out = inst_path.join("test.mrpack");
        let options = ModpackExportOptions {
            instance_id: inst_id.to_string(),
            format: ModpackExportFormat::Modrinth,
            pack_name: "Exported Test Pack".to_string(),
            pack_version: "1.0.0".to_string(),
            author: "Tester".to_string(),
            summary: "A test export pack".to_string(),
            selected_paths,
            output_path: export_out.display().to_string(),
        };

        let mr_client = ModrinthClient::default().with_base_url(server.uri());

        let result_path = export_modpack(inst_path, &options, Some(&mr_client), None, None)
            .await
            .unwrap();

        assert!(result_path.exists());

        // Re-import round-trip test via inspect_modpack and import_modpack
        let plan = crate::modpack::inspect_modpack(&result_path).unwrap();
        assert_eq!(plan.inspect.format, crate::modpack::ModpackFormat::Modrinth);
        assert_eq!(plan.inspect.mc_version, "1.20.1");
        assert_eq!(plan.inspect.loader.as_deref(), Some("fabric"));
        assert_eq!(plan.inspect.name, "Exported Test Pack");

        let dest_mc = tempdir().unwrap();
        let http_client = reqwest::Client::new();
        let net_settings = mclr_core::NetSettings::default();

        let imported_info = crate::modpack::import_modpack(
            &result_path,
            "imported-instance",
            dest_mc.path(),
            &http_client,
            &net_settings,
            None,
            None,
            None,
            false,
        )
        .await
        .unwrap();

        assert_eq!(imported_info.name, "imported-instance");
        let imported_dir = dest_mc.path().join("versions/imported-instance");
        assert!(imported_dir.join("mods/sample.jar").exists());
        assert_eq!(
            std::fs::read(imported_dir.join("mods/sample.jar")).unwrap(),
            sample_jar_bytes
        );
        assert!(imported_dir.join("config/test.toml").exists());
        assert_eq!(
            std::fs::read(imported_dir.join("config/test.toml")).unwrap(),
            b"setting = true\n"
        );
        assert!(imported_dir.join("options.txt").exists());
        assert_eq!(
            std::fs::read(imported_dir.join("options.txt")).unwrap(),
            b"fov:90\n"
        );
    }
}
