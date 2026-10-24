//! Persistent UI state settings (`settings.states`).

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct StatesSettings {
    pub dismissed_announcements: Vec<String>,
}
