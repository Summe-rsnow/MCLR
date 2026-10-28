//! Java discovery and selection settings.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct JavaSettings {
    /// Extra paths the user added by hand; scanned together with auto-discovered ones.
    pub custom: Vec<String>,
    pub prefer_jdk: bool,
}
