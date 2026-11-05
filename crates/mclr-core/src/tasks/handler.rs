//! Task handler trait, execution context, and network probe definition.

use crate::AppError;
use crate::tasks::center::{TaskCenter, TaskControl};
use crate::tasks::model::TaskPlan;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::path::Path;
use std::sync::Arc;

/// Trait implemented by executable background task definitions.
#[async_trait]
pub trait TaskHandler: Send + Sync + 'static {
    /// Unique kind string (e.g. "install.vanilla", "comp.download").
    fn kind(&self) -> &'static str;

    /// Whether this task kind supports manual restoration or retries after failure or interruption.
    fn resumable(&self) -> bool {
        true
    }

    /// Validates payload and generates the initial step plan and title.
    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError>;

    /// Executes the task.
    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError>;
}

/// Execution context provided to a running `TaskHandler`.
pub struct TaskCtx {
    pub id: String,
    pub resumed: bool,
    pub attempt: u32,
    pub(crate) payload: serde_json::Value,
    pub(crate) center: Arc<TaskCenter>,
    pub(crate) control: TaskControl,
}

impl TaskCtx {
    pub fn new(
        id: String,
        resumed: bool,
        attempt: u32,
        payload: serde_json::Value,
        center: Arc<TaskCenter>,
        control: TaskControl,
    ) -> Self {
        Self {
            id,
            resumed,
            attempt,
            payload,
            center,
            control,
        }
    }

    /// Deserializes the task payload.
    pub fn payload<T: DeserializeOwned>(&self) -> Result<T, AppError> {
        serde_json::from_value(self.payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))
    }

    /// Borrowed reference suitable for pipeline functions expecting `Option<(&TaskCenter, &str)>`.
    pub fn tc(&self) -> Option<(&TaskCenter, &str)> {
        Some((&self.center, &self.id))
    }
    /// Cloned reference to the task center.
    pub fn center(&self) -> Arc<TaskCenter> {
        Arc::clone(&self.center)
    }

    /// Task control handle for checkpointing and cancellation checking.
    pub fn control(&self) -> &TaskControl {
        &self.control
    }

    /// Activates a named step. Returns false if the step is already Done.
    pub async fn begin_step(&self, key: &str) -> bool {
        self.center.begin_step(&self.id, key).await
    }

    /// Marks a named step as Done.
    pub async fn finish_step(&self, key: &str) {
        let _ = self.center.finish_step(&self.id, key).await;
    }

    /// Marks a named step as Skipped.
    pub async fn skip_step(&self, key: &str) {
        let _ = self.center.skip_step(&self.id, key).await;
    }

    /// Updates item counters for the current step.
    pub async fn set_items(&self, done: u32, total: u32) {
        let _ = self.center.set_current_items(&self.id, done, total).await;
    }

    /// Updates progress and speed.
    pub async fn progress(&self, p: f32, speed_bps: u64) {
        let _ = self.center.update_progress(&self.id, p, speed_bps).await;
    }

    /// Adds an artifact path to be preserved while the task is active or resumable.
    pub async fn add_artifact(&self, p: &Path) {
        self.center.add_artifact(&self.id, p).await;
    }

    /// Idempotently finishes a tracked task with either success or failure.
    pub async fn finish(&self, r: Result<(), AppError>) {
        match r {
            Ok(()) => {
                let _ = self.center.complete_task(&self.id).await;
            }
            Err(e) => {
                let _ = self.center.fail_task(&self.id, e).await;
            }
        }
    }
}

/// Interface for probing network connectivity during automatic retry backoff.
#[async_trait]
pub trait NetProbe: Send + Sync {
    async fn online(&self) -> bool;
}
