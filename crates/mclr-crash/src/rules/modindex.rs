//! Mod indexing and resolution for crash stack/evidence analysis.

use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct CrashModInfo {
    pub mod_id: Option<String>,
    pub display_name: Option<String>,
    pub file_name: Option<String>,
    pub version: Option<String>,
}

impl CrashModInfo {
    pub fn display_name_or_file_name(&self) -> String {
        if let Some(name) = &self.display_name
            && !name.trim().is_empty()
        {
            return name.clone();
        }
        if let Some(file) = &self.file_name
            && !file.trim().is_empty()
        {
            return file.clone();
        }
        self.mod_id.clone().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CrashModIndex {
    mods: Vec<CrashModInfo>,
}

impl CrashModIndex {
    pub fn new() -> Self {
        Self { mods: Vec::new() }
    }

    /// Builds index from instance mods folder and logs.
    pub fn build(instance_dir: &Path, all_logs: &str) -> Self {
        let mut index = Self::new();

        // 1. Scan mods directory if present
        let mods_dir = instance_dir.join("mods");
        if mods_dir.exists()
            && let Ok(scanned) = mclr_comp::scan_local_mods(&mods_dir)
        {
            for m in scanned {
                index.mods.push(CrashModInfo {
                    mod_id: Some(m.mod_id),
                    display_name: Some(m.name),
                    file_name: Some(m.file_name),
                    version: Some(m.version),
                });
            }
        }

        // 2. Extract mods from logs (Fabric / Forge lists in crash reports)
        index.extract_from_logs(all_logs);
        index.deduplicate();
        index
    }

    fn extract_from_logs(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        // Fabric mods section in crash reports: "\t\tmodid: Mod Name version"
        let fabric_regex =
            Regex::new(r"(?m)^\t\t([a-zA-Z0-9_\-]+):\s+([^\n]+?)\s+([0-9a-zA-Z\.\-]+)$").unwrap();
        for caps in fabric_regex.captures_iter(text) {
            let mod_id = caps.get(1).map(|m| m.as_str().to_string());
            let display_name = caps.get(2).map(|m| m.as_str().to_string());
            let version = caps.get(3).map(|m| m.as_str().to_string());
            self.mods.push(CrashModInfo {
                mod_id,
                display_name,
                file_name: None,
                version,
            });
        }

        // Forge mod files in crash reports: "... | modid | ... | filename.jar"
        let forge_jar_regex = Regex::new(r"(?i)([a-zA-Z0-9_\-\.]+?\.jar)").unwrap();
        for caps in forge_jar_regex.captures_iter(text) {
            if let Some(m) = caps.get(1) {
                let filename = m.as_str().to_string();
                if !self
                    .mods
                    .iter()
                    .any(|existing| existing.file_name.as_deref() == Some(&filename))
                {
                    self.mods.push(CrashModInfo {
                        mod_id: None,
                        display_name: None,
                        file_name: Some(filename),
                        version: None,
                    });
                }
            }
        }
    }

    fn deduplicate(&mut self) {
        let mut unique: Vec<CrashModInfo> = Vec::new();
        for m in self.mods.drain(..) {
            let exists = unique.iter().any(|u| {
                (u.mod_id.is_some() && u.mod_id == m.mod_id)
                    || (u.file_name.is_some() && u.file_name == m.file_name)
            });
            if !exists {
                unique.push(m);
            }
        }
        self.mods = unique;
    }

    pub fn resolve(&self, hint: &str) -> Vec<CrashModInfo> {
        let norm = normalize(hint);
        if norm.is_empty() {
            return Vec::new();
        }
        self.mods
            .iter()
            .filter(|m| {
                m.mod_id
                    .as_deref()
                    .map(normalize)
                    .map(|s| s.contains(&norm))
                    .unwrap_or(false)
                    || m.file_name
                        .as_deref()
                        .map(normalize)
                        .map(|s| s.contains(&norm))
                        .unwrap_or(false)
                    || m.display_name
                        .as_deref()
                        .map(normalize)
                        .map(|s| s.contains(&norm))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub fn resolve_many(&self, hints: &[String]) -> Vec<CrashModInfo> {
        let mut results = Vec::new();
        for hint in hints {
            for found in self.resolve(hint) {
                if !results.iter().any(|r: &CrashModInfo| {
                    r.display_name_or_file_name() == found.display_name_or_file_name()
                }) {
                    results.push(found);
                }
            }
        }
        results
    }

    pub fn resolve_to_display_names(&self, hints: &[String]) -> Vec<String> {
        let mut clean_hints: Vec<String> = hints
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        clean_hints.dedup();

        let resolved: Vec<String> = self
            .resolve_many(&clean_hints)
            .into_iter()
            .map(|m| m.display_name_or_file_name())
            .filter(|s| !s.is_empty())
            .collect();

        if !resolved.is_empty() {
            resolved
        } else {
            clean_hints
        }
    }
}

fn normalize(s: &str) -> String {
    s.to_ascii_lowercase().replace(['-', '_', '.', ' '], "")
}
