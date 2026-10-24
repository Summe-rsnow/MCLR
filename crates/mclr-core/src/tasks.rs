//! Task management, hierarchical task tree, cancellation tokens, and progress reporting.

use crate::AppError;
use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// State of an asynchronous background task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskState {
    Waiting,
    Running,
    Paused,
    Success,
    Canceled,
    Failed,
}

/// A node in the hierarchical task tree.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub title_key: String,
    #[serde(default)]
    pub title_params: HashMap<String, String>,
    pub state: TaskState,
    pub progress: f32,
    #[specta(type = u32)]
    pub speed_bps: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

struct InternalTask {
    node: TaskNode,
    token: CancellationToken,
}

/// Thread-safe registry and coordinator for all long-running tasks.
pub struct TaskCenter {
    tasks: RwLock<HashMap<String, InternalTask>>,
    update_tx: broadcast::Sender<Vec<TaskNode>>,
}

impl TaskCenter {
    pub fn new() -> Self {
        let (update_tx, _) = broadcast::channel(128);
        Self {
            tasks: RwLock::new(HashMap::new()),
            update_tx,
        }
    }

    /// Subscribes to task diff / update broadcasts (`Vec<TaskNode>`).
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<TaskNode>> {
        self.update_tx.subscribe()
    }

    /// Creates a new task in `Waiting` state and returns its assigned ID and `CancellationToken`.
    pub async fn create_task(
        &self,
        parent_id: Option<String>,
        title_key: impl Into<String>,
        title_params: HashMap<String, String>,
    ) -> (String, CancellationToken) {
        let id = Uuid::new_v4().to_string();
        let token = CancellationToken::new();

        let node = TaskNode {
            id: id.clone(),
            parent_id,
            title_key: title_key.into(),
            title_params,
            state: TaskState::Waiting,
            progress: 0.0,
            speed_bps: 0,
            error: None,
        };

        {
            let mut lock = self.tasks.write().await;
            lock.insert(
                id.clone(),
                InternalTask {
                    node: node.clone(),
                    token: token.clone(),
                },
            );
        }

        let _ = self.update_tx.send(vec![node]);
        (id, token)
    }

    /// Marks a task as `Running`.
    pub async fn start_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            task.node.state = TaskState::Running;
            diff.push(task.node.clone());
        }
        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Updates task progress (clamped to 0.0..=1.0) and current speed in bytes/sec.
    pub async fn update_progress(
        &self,
        id: &str,
        progress: f32,
        speed_bps: u64,
    ) -> Result<(), CoreError> {
        let clamped = progress.clamp(0.0, 1.0);
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            task.node.progress = clamped;
            task.node.speed_bps = speed_bps;
            diff.push(task.node.clone());
        }
        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Marks a task as `Success` (progress set to 1.0, speed to 0).
    pub async fn complete_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            task.node.state = TaskState::Success;
            task.node.progress = 1.0;
            task.node.speed_bps = 0;
            diff.push(task.node.clone());
        }
        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Marks a task as `Failed` with an error.
    pub async fn fail_task(&self, id: &str, error: AppError) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            task.node.state = TaskState::Failed;
            task.node.error = Some(error);
            task.node.speed_bps = 0;
            diff.push(task.node.clone());
        }
        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Pauses a task if it is currently `Running`.
    pub async fn pause_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            if task.node.state == TaskState::Running {
                task.node.state = TaskState::Paused;
                diff.push(task.node.clone());
            }
        }
        if !diff.is_empty() {
            let _ = self.update_tx.send(diff);
        }
        Ok(())
    }

    /// Resumes a paused task to `Running`.
    pub async fn resume_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            if task.node.state == TaskState::Paused {
                task.node.state = TaskState::Running;
                diff.push(task.node.clone());
            }
        }
        if !diff.is_empty() {
            let _ = self.update_tx.send(diff);
        }
        Ok(())
    }

    /// Cancels a task, triggers its `CancellationToken`, and recursively cascades
    /// cancellation to all child tasks.
    pub async fn cancel_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            if !lock.contains_key(id) {
                return Err(CoreError::TaskNotFound(id.to_string()));
            }

            // Collect target and all transitive children
            let mut to_cancel = vec![id.to_string()];
            let mut idx = 0;
            while idx < to_cancel.len() {
                let current_id = to_cancel[idx].clone();
                idx += 1;
                for (child_id, task) in lock.iter() {
                    if let Some(pid) = &task.node.parent_id
                        && pid == &current_id
                        && !to_cancel.contains(child_id)
                    {
                        to_cancel.push(child_id.clone());
                    }
                }
            }

            for cid in to_cancel {
                if let Some(task) = lock.get_mut(&cid)
                    && task.node.state != TaskState::Success
                    && task.node.state != TaskState::Failed
                {
                    task.node.state = TaskState::Canceled;
                    task.node.speed_bps = 0;
                    task.token.cancel();
                    diff.push(task.node.clone());
                }
            }
        }

        if !diff.is_empty() {
            let _ = self.update_tx.send(diff);
        }
        Ok(())
    }

    /// Returns a list of all current tasks in the registry.
    pub async fn list_tasks(&self) -> Vec<TaskNode> {
        let lock = self.tasks.read().await;
        lock.values().map(|t| t.node.clone()).collect()
    }
}

impl Default for TaskCenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_lifecycle_and_clamping() {
        let center = TaskCenter::new();
        let mut rx = center.subscribe();

        let (task_id, token) = center
            .create_task(None, "task.install.vanilla", HashMap::new())
            .await;
        assert!(!token.is_cancelled());

        let initial_update = rx.recv().await.unwrap();
        assert_eq!(initial_update[0].state, TaskState::Waiting);

        // Start
        center.start_task(&task_id).await.unwrap();
        let update = rx.recv().await.unwrap();
        assert_eq!(update[0].state, TaskState::Running);

        // Progress clamp below 0
        center.update_progress(&task_id, -0.5, 1024).await.unwrap();
        let update = rx.recv().await.unwrap();
        assert_eq!(update[0].progress, 0.0);
        assert_eq!(update[0].speed_bps, 1024);

        // Progress clamp above 1.0
        center.update_progress(&task_id, 1.5, 2048).await.unwrap();
        let update = rx.recv().await.unwrap();
        assert_eq!(update[0].progress, 1.0);

        // Pause
        center.pause_task(&task_id).await.unwrap();
        let update = rx.recv().await.unwrap();
        assert_eq!(update[0].state, TaskState::Paused);

        // Resume
        center.resume_task(&task_id).await.unwrap();
        let update = rx.recv().await.unwrap();
        assert_eq!(update[0].state, TaskState::Running);

        // Complete
        center.complete_task(&task_id).await.unwrap();
        let update = rx.recv().await.unwrap();
        assert_eq!(update[0].state, TaskState::Success);
        assert_eq!(update[0].progress, 1.0);
    }

    #[tokio::test]
    async fn test_task_cancellation_cascade() {
        let center = TaskCenter::new();

        let (parent_id, parent_token) = center
            .create_task(None, "task.parent", HashMap::new())
            .await;
        let (child1_id, child1_token) = center
            .create_task(Some(parent_id.clone()), "task.child1", HashMap::new())
            .await;
        let (child2_id, child2_token) = center
            .create_task(Some(child1_id.clone()), "task.child2", HashMap::new())
            .await;

        center.start_task(&parent_id).await.unwrap();
        center.start_task(&child1_id).await.unwrap();
        center.start_task(&child2_id).await.unwrap();

        // Cancel parent -> cascades to child1 and child2
        center.cancel_task(&parent_id).await.unwrap();

        assert!(parent_token.is_cancelled());
        assert!(child1_token.is_cancelled());
        assert!(child2_token.is_cancelled());

        let tasks = center.list_tasks().await;
        let state_map: HashMap<String, TaskState> =
            tasks.into_iter().map(|t| (t.id, t.state)).collect();

        assert_eq!(state_map.get(&parent_id), Some(&TaskState::Canceled));
        assert_eq!(state_map.get(&child1_id), Some(&TaskState::Canceled));
        assert_eq!(state_map.get(&child2_id), Some(&TaskState::Canceled));
    }
}
