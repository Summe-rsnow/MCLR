//! Task engine and persistence settings.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct TaskSettings {
    pub max_concurrent: u32,
    pub auto_retry: u32,
    pub notify: bool,
    pub history_max: u32,
    pub history_days: u32,
}

impl Default for TaskSettings {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            auto_retry: 3,
            notify: true,
            history_max: 200,
            history_days: 30,
        }
    }
}

impl TaskSettings {
    pub fn clamped(&self) -> Self {
        Self {
            max_concurrent: self.max_concurrent.clamp(1, 8),
            auto_retry: self.auto_retry.clamp(0, 10),
            notify: self.notify,
            history_max: self.history_max.clamp(20, 2000),
            history_days: self.history_days.clamp(1, 365),
        }
    }
}
