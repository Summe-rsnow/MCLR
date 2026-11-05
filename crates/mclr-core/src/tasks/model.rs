//! Task domain models, states, and plan representations.

use crate::AppError;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

/// State of an asynchronous background task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskState {
    Waiting,
    Running,
    Paused,
    Retrying,
    Interrupted,
    Success,
    Canceled,
    Failed,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success | Self::Failed | Self::Canceled)
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Waiting | Self::Running | Self::Paused | Self::Retrying
        )
    }
}

pub fn is_terminal(state: TaskState) -> bool {
    state.is_terminal()
}

pub fn is_active(state: TaskState) -> bool {
    state.is_active()
}

/// State of a single step within a task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StepState {
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
}

/// A step in the task timeline.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StepNode {
    pub key: String,
    pub state: StepState,
    pub weight: f32,
    pub items_done: u32,
    pub items_total: u32,
    #[specta(type = f64)]
    pub started_at_ms: Option<u64>,
    #[specta(type = f64)]
    pub finished_at_ms: Option<u64>,
    pub error: Option<AppError>,
}

/// A top-level task node.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskNode {
    pub id: String,
    pub kind: String,
    pub title_key: String,
    #[serde(default)]
    pub title_params: HashMap<String, String>,
    pub state: TaskState,
    pub progress: f32,
    #[specta(type = u32)]
    pub speed_bps: u64,
    pub steps: Vec<StepNode>,
    pub current_step: Option<String>,
    pub attempt: u32,
    pub max_attempts: u32,
    #[specta(type = f64)]
    pub next_retry_at_ms: Option<u64>,
    pub resumable: bool,
    #[specta(type = f64)]
    pub queue_seq: u64,
    #[specta(type = f64)]
    pub created_at_ms: u64,
    #[specta(type = f64)]
    pub started_at_ms: Option<u64>,
    #[specta(type = f64)]
    pub finished_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

/// Specification for creating a task with predefined steps.
#[derive(Debug, Clone)]
pub struct TaskPlan {
    pub title_key: String,
    pub title_params: HashMap<String, String>,
    pub steps: Vec<StepPlan>,
}

/// Specification for a single step within a task plan.
#[derive(Debug, Clone)]
pub struct StepPlan {
    pub key: &'static str,
    pub weight: f32,
}
