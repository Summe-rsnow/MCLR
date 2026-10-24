//! Minecraft version JSON model, inheritance merging, and rule evaluation.

use crate::error::GameError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rule evaluation context based on system environment and game launch parameters.
#[derive(Debug, Clone)]
pub struct RuleContext {
    pub os_name: String,
    pub os_arch: String,
    pub os_version: Option<String>,
    pub features: HashMap<String, bool>,
}

impl RuleContext {
    /// Windows x64 default rule context.
    pub fn windows_default() -> Self {
        let mut features = HashMap::new();
        features.insert("is_demo_user".to_string(), false);
        features.insert("has_custom_resolution".to_string(), false);
        features.insert("has_quick_plays_support".to_string(), false);
        features.insert("is_quick_play_singleplayer".to_string(), false);
        features.insert("is_quick_play_multiplayer".to_string(), false);
        features.insert("is_quick_play_realms".to_string(), false);

        Self {
            os_name: "windows".to_string(),
            os_arch: "x64".to_string(),
            os_version: None,
            features,
        }
    }
}

/// OS rule constraint inside rule definitions.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

/// Individual rule definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub action: String, // "allow" or "disallow"
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

/// Evaluate whether an item with rules should be applied in given context.
///
/// Default action is disallow if rules are present; if no rules are present, action is allow.
pub fn evaluate_rules(rules: &[Rule], ctx: &RuleContext) -> bool {
    if rules.is_empty() {
        return true;
    }

    let mut allowed = false;

    for rule in rules {
        let action_allow = rule.action == "allow";
        let mut matches = true;

        if let Some(os) = &rule.os {
            if let Some(name) = &os.name {
                if !name.eq_ignore_ascii_case(&ctx.os_name) {
                    matches = false;
                }
            }
            if matches {
                if let Some(arch) = &os.arch {
                    if !arch.eq_ignore_ascii_case(&ctx.os_arch) {
                        matches = false;
                    }
                }
            }
            if matches {
                if let (Some(rule_ver), Some(ctx_ver)) = (&os.version, &ctx.os_version) {
                    if let Ok(re) = regex::Regex::new(rule_ver) {
                        if !re.is_match(ctx_ver) {
                            matches = false;
                        }
                    }
                }
            }
        }

        if matches {
            if let Some(features) = &rule.features {
                for (k, v) in features {
                    let ctx_val = ctx.features.get(k).copied().unwrap_or(false);
                    if ctx_val != *v {
                        matches = false;
                        break;
                    }
                }
            }
        }

        if matches {
            allowed = action_allow;
        }
    }

    allowed
}

/// Argument value which can be either a single string or an array of strings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ArgumentValue {
    Single(String),
    Multiple(Vec<String>),
}

impl ArgumentValue {
    pub fn as_slice(&self) -> Vec<String> {
        match self {
            ArgumentValue::Single(s) => vec![s.clone()],
            ArgumentValue::Multiple(v) => v.clone(),
        }
    }
}

/// Conditional argument entry containing rules and argument values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalArgument {
    pub rules: Vec<Rule>,
    pub value: ArgumentValue,
}

/// Single argument entry: plain string or conditional with rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ArgumentEntry {
    Plain(String),
    Conditional(ConditionalArgument),
}

/// JVM and game arguments container (Minecraft 1.13+).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgumentEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgumentEntry>,
}

/// Extract rules for native libraries.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ExtractRules {
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Download artifact info for libraries or client/server.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Artifact {
    pub path: Option<String>,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// Library downloads section.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, Artifact>,
}

/// Library definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub name: String,
    pub url: Option<String>,
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub natives: HashMap<String, String>,
    pub extract: Option<ExtractRules>,
}

/// Asset index information.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub total_size: Option<u64>,
    pub url: Option<String>,
}

/// Download entry for client, server, etc.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DownloadEntry {
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: Option<String>,
}

/// Version downloads container.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VersionDownloads {
    pub client: Option<DownloadEntry>,
    pub server: Option<DownloadEntry>,
    #[serde(rename = "client_mappings")]
    pub client_mappings: Option<DownloadEntry>,
    #[serde(rename = "server_mappings")]
    pub server_mappings: Option<DownloadEntry>,
}

/// Java version requirement from version.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersionInfo {
    pub component: String,
    pub major_version: u32,
}

/// Logging configuration file download info.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LoggingFile {
    pub id: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: Option<String>,
}

/// Logging client configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LoggingClient {
    pub argument: Option<String>,
    pub file: Option<LoggingFile>,
    #[serde(rename = "type")]
    pub log_type: Option<String>,
}

/// Version logging block.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LoggingInfo {
    pub client: Option<LoggingClient>,
}

/// Complete Minecraft version JSON model.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: Option<String>,
    pub main_class: Option<String>,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub asset_index: Option<AssetIndexInfo>,
    pub assets: Option<String>,
    pub downloads: Option<VersionDownloads>,
    pub java_version: Option<JavaVersionInfo>,
    pub inherits_from: Option<String>,
    pub release_time: Option<String>,
    pub time: Option<String>,
    pub logging: Option<LoggingInfo>,
    pub compliance_level: Option<u32>,
    pub minimum_launcher_version: Option<u32>,
}

impl VersionJson {
    pub fn parse(content: &str) -> Result<Self, GameError> {
        serde_json::from_str(content).map_err(|e| GameError::InvalidVersionJson(e.to_string()))
    }
}

/// Library Maven coordinate representation: group:artifact[:version][:classifier].
/// Returns `group:artifact[:classifier]` as key for deduplication.
pub fn library_dedup_key(maven_name: &str) -> String {
    let parts: Vec<&str> = maven_name.split(':').collect();
    if parts.len() >= 4 {
        // group:artifact:version:classifier(@ext)
        // classifier might have extension like "natives-windows@jar"
        let classifier = parts[3].split('@').next().unwrap_or(parts[3]);
        format!("{}:{}:{}", parts[0], parts[1], classifier)
    } else if parts.len() >= 2 {
        // group:artifact:version
        format!("{}:{}", parts[0], parts[1])
    } else {
        maven_name.to_string()
    }
}

/// Merges `child` and `parent` version JSONs following Minecraft inheritance rules.
///
/// Rules:
/// - Child fields override parent fields when present (id, main_class, java_version, etc.).
/// - InheritsFrom: child's inherits_from is cleared or remains parent's inherits_from.
/// - Arguments: concatenated with child arguments first or merged.
/// - Libraries: concatenated with child libraries first, deduplicated by `group:artifact[:classifier]`
///   keeping the child version.
pub fn merge_version_json(child: VersionJson, parent: VersionJson) -> VersionJson {
    let id = if !child.id.is_empty() {
        child.id
    } else {
        parent.id
    };

    let version_type = child.version_type.or(parent.version_type);
    let main_class = child.main_class.or(parent.main_class);
    let minecraft_arguments = child.minecraft_arguments.or(parent.minecraft_arguments);

    // Merge arguments: child args + parent args
    let arguments = match (child.arguments, parent.arguments) {
        (Some(c), Some(p)) => {
            let mut game = c.game;
            game.extend(p.game);
            let mut jvm = c.jvm;
            jvm.extend(p.jvm);
            Some(Arguments { game, jvm })
        }
        (Some(c), None) => Some(c),
        (None, Some(p)) => Some(p),
        (None, None) => None,
    };

    // Merge libraries: child first, dedup keeping first (child's)
    let mut merged_libraries = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();

    for lib in child.libraries {
        let key = library_dedup_key(&lib.name);
        if seen_keys.insert(key) {
            merged_libraries.push(lib);
        }
    }

    for lib in parent.libraries {
        let key = library_dedup_key(&lib.name);
        if seen_keys.insert(key) {
            merged_libraries.push(lib);
        }
    }

    let asset_index = child.asset_index.or(parent.asset_index);
    let assets = child.assets.or(parent.assets);
    let downloads = child.downloads.or(parent.downloads);
    let java_version = child.java_version.or(parent.java_version);
    let release_time = child.release_time.or(parent.release_time);
    let time = child.time.or(parent.time);
    let logging = child.logging.or(parent.logging);
    let compliance_level = child.compliance_level.or(parent.compliance_level);
    let minimum_launcher_version = child
        .minimum_launcher_version
        .or(parent.minimum_launcher_version);

    VersionJson {
        id,
        version_type,
        main_class,
        minecraft_arguments,
        arguments,
        libraries: merged_libraries,
        asset_index,
        assets,
        downloads,
        java_version,
        inherits_from: parent.inherits_from,
        release_time,
        time,
        logging,
        compliance_level,
        minimum_launcher_version,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_dedup_key() {
        assert_eq!(
            library_dedup_key("org.lwjgl:lwjgl:3.3.1"),
            "org.lwjgl:lwjgl"
        );
        assert_eq!(
            library_dedup_key("org.lwjgl:lwjgl:3.3.1:natives-windows"),
            "org.lwjgl:lwjgl:natives-windows"
        );
        assert_eq!(
            library_dedup_key("org.lwjgl:lwjgl:3.3.1:natives-windows@jar"),
            "org.lwjgl:lwjgl:natives-windows"
        );
    }

    #[test]
    fn test_merge_version_json_dedup_libraries() {
        let child_raw = r#"{
            "id": "1.20.1-fabric",
            "inheritsFrom": "1.20.1",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [
                { "name": "org.ow2.asm:asm:9.5" },
                { "name": "net.fabricmc:fabric-loader:0.14.21" }
            ]
        }"#;

        let parent_raw = r#"{
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [
                { "name": "org.ow2.asm:asm:9.3" },
                { "name": "com.mojang:brigadier:1.0.18" }
            ],
            "assets": "1.20"
        }"#;

        let child = VersionJson::parse(child_raw).unwrap();
        let parent = VersionJson::parse(parent_raw).unwrap();

        let merged = merge_version_json(child, parent);

        assert_eq!(merged.id, "1.20.1-fabric");
        assert_eq!(
            merged.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        assert_eq!(merged.assets.as_deref(), Some("1.20"));
        assert_eq!(merged.libraries.len(), 3);
        assert_eq!(merged.libraries[0].name, "org.ow2.asm:asm:9.5");
        assert_eq!(
            merged.libraries[1].name,
            "net.fabricmc:fabric-loader:0.14.21"
        );
        assert_eq!(merged.libraries[2].name, "com.mojang:brigadier:1.0.18");
    }

    #[test]
    fn test_evaluate_rules() {
        let ctx = RuleContext::windows_default();

        let rules = vec![
            Rule {
                action: "allow".to_string(),
                os: None,
                features: None,
            },
            Rule {
                action: "disallow".to_string(),
                os: Some(OsRule {
                    name: Some("osx".to_string()),
                    version: None,
                    arch: None,
                }),
                features: None,
            },
        ];
        assert!(evaluate_rules(&rules, &ctx));

        let rules_windows_disallow = vec![
            Rule {
                action: "allow".to_string(),
                os: None,
                features: None,
            },
            Rule {
                action: "disallow".to_string(),
                os: Some(OsRule {
                    name: Some("windows".to_string()),
                    version: None,
                    arch: None,
                }),
                features: None,
            },
        ];
        assert!(!evaluate_rules(&rules_windows_disallow, &ctx));

        let rules_feature = vec![Rule {
            action: "allow".to_string(),
            os: None,
            features: Some([("is_demo_user".to_string(), true)].into()),
        }];
        assert!(!evaluate_rules(&rules_feature, &ctx));
    }

    #[test]
    fn test_legacy_1_7_10_version_json() {
        let legacy_raw = r#"{
            "id": "1.7.10",
            "time": "2014-06-26T16:00:00+02:00",
            "releaseTime": "2014-06-26T16:00:00+02:00",
            "type": "release",
            "minecraftArguments": "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userProperties ${user_properties} --userType ${user_type}",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [
                {
                    "name": "com.mojang:netty:1.6"
                }
            ],
            "assets": "legacy"
        }"#;

        let parsed = VersionJson::parse(legacy_raw).unwrap();
        assert_eq!(parsed.id, "1.7.10");
        assert_eq!(parsed.assets.as_deref(), Some("legacy"));
        assert!(parsed.minecraft_arguments.is_some());
        assert_eq!(parsed.libraries.len(), 1);
    }
}
