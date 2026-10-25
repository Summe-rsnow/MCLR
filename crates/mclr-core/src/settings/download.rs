//! Download and instance-install settings.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct DownloadSettings {
    pub auto_select_instance: bool,
}

impl Default for DownloadSettings {
    fn default() -> Self {
        Self {
            auto_select_instance: true,
        }
    }
}
