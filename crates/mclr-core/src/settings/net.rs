//! Network, proxy, DoH, and download concurrency settings.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ProxyMode {
    #[default]
    None,
    System,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SourcePriority {
    #[default]
    MirrorFirst,
    Smart,
    OfficialFirst,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct NetSettings {
    pub proxy_mode: ProxyMode,
    pub proxy_url: String,
    pub doh: bool,
    pub source_priority: SourcePriority,
    pub connections_per_file: u32,
    pub max_concurrent_files: u32,
}

impl Default for NetSettings {
    fn default() -> Self {
        Self {
            proxy_mode: ProxyMode::None,
            proxy_url: String::new(),
            doh: false,
            source_priority: SourcePriority::MirrorFirst,
            connections_per_file: 8,
            max_concurrent_files: 16,
        }
    }
}
