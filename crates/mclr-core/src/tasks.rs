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

/// Shared handle a worker polls to honour pause/cancel requests.
///
/// Cloned per task at creation time; workers call [`TaskControl::checkpoint`] between
/// files/chunks, which is what actually stops progress and bandwidth.
#[derive(Clone)]
pub struct TaskControl {
    token: CancellationToken,
    paused: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TaskControl {
    fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            paused: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// True once the user cancelled the task.
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn set_paused(&self, paused: bool) {
        self.paused
            .store(paused, std::sync::atomic::Ordering::Relaxed);
    }

    /// Cancels the task; workers observe it through [`TaskControl::is_cancelled`].
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Raw token, for `select!`-style racing against the work.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Blocks while the task is paused and fails once it is cancelled.
    pub async fn checkpoint(&self) -> Result<(), TaskAborted> {
        if self.token.is_cancelled() {
            return Err(TaskAborted);
        }
        while self.is_paused() {
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            if self.token.is_cancelled() {
                return Err(TaskAborted);
            }
        }
        Ok(())
    }
}

/// Returned by [`TaskControl::checkpoint`] once the task was cancelled.
#[derive(Debug, Clone, Copy)]
pub struct TaskAborted;

struct InternalTask {
    node: TaskNode,
    control: TaskControl,
    /// Slice of the bar this task's reports are mapped into: `(base, span)`.
    progress_window: (f32, f32),
    /// Highest report seen since the window was set — per-stage restarts hold, never rewind.
    progress_high: f32,
    /// False until `set_progress_window` is used: otherwise reports are stored verbatim.
    windowed: bool,
}

fn is_terminal(state: TaskState) -> bool {
    matches!(
        state,
        TaskState::Success | TaskState::Failed | TaskState::Canceled
    )
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
    ) -> (String, TaskControl) {
        let id = Uuid::new_v4().to_string();
        let control = TaskControl::new();

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
                    control: control.clone(),
                    progress_window: (0.0, 1.0),
                    progress_high: 0.0,
                    windowed: false,
                },
            );
        }

        let _ = self.update_tx.send(vec![node]);
        (id, control)
    }

    /// Handle a worker uses to honour pause/cancel requests for `id`.
    pub async fn control(&self, id: &str) -> Option<TaskControl> {
        let lock = self.tasks.read().await;
        lock.get(id).map(|task| task.control.clone())
    }

    /// Marks a task as `Running`.
    pub async fn start_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            // A cancelled task must not start: the worker would otherwise begin work
            // the user already aborted.
            if task.node.state == TaskState::Canceled {
                return Ok(());
            }
            task.node.state = TaskState::Running;
            diff.push(task.node.clone());
        }
        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Restricts this task's progress reports to a slice of the bar.
    ///
    /// `base` is where the slice starts and `span` its width, so a stage keeps reporting its own
    /// `0.0..=1.0` while the visible progress advances across the whole task. Within one slice
    /// progress never moves backwards, which is what absorbs stages that internally restart
    /// (libraries, then assets, then the loader installer).
    pub async fn set_progress_window(
        &self,
        id: &str,
        base: f32,
        span: f32,
    ) -> Result<(), CoreError> {
        let base = base.clamp(0.0, 1.0);
        let span = span.clamp(0.0, 1.0 - base);
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            if is_terminal(task.node.state) {
                return Ok(());
            }
            task.progress_window = (base, span);
            task.progress_high = 0.0;
            task.windowed = true;
            task.node.progress = task.node.progress.max(base);
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
        let local = progress.clamp(0.0, 1.0);
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            // A late report from a finished stage must not repaint a closed task.
            if is_terminal(task.node.state) {
                return Ok(());
            }
            let mapped = if task.windowed {
                task.progress_high = task.progress_high.max(local);
                let (base, span) = task.progress_window;
                base + task.progress_high * span
            } else {
                local
            };
            task.node.progress = mapped.clamp(0.0, 1.0);
            task.node.speed_bps = speed_bps;
            diff.push(task.node.clone());
        }
        let _ = self.update_tx.send(diff);
        Ok(())
    }
    /// Updates task title parameters.
    pub async fn update_title_params(
        &self,
        id: &str,
        params: HashMap<String, String>,
    ) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            task.node.title_params.extend(params);
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
            // A cancelled task stays cancelled: the worker may still be winding down.
            if task.node.state == TaskState::Canceled {
                return Ok(());
            }
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
            if task.node.state == TaskState::Canceled {
                return Ok(());
            }
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
                task.control.set_paused(true);
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
                task.control.set_paused(false);
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
                    task.control.cancel();
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

    /// True while any task is still in flight (`waiting` / `running` / `paused`).
    ///
    /// Callers that delete files a task may still be writing use this as a hard gate: a cleanup
    /// that runs mid-download cannot know which part files are live.
    pub async fn has_active_tasks(&self) -> bool {
        let lock = self.tasks.read().await;
        lock.values().any(|task| !is_terminal(task.node.state))
    }

    /// Drops every finished task (`success` / `failed` / `canceled`) and returns their ids.
    ///
    /// The registry lives in memory and would otherwise grow for the whole session: a task that
    /// ended an hour ago still holds its card, and the UI brings it back on every fetch. A task is
    /// only dropped when its parent is dropped too, so a running stage never loses the card it is
    /// nested under.
    pub async fn clear_finished(&self) -> Vec<String> {
        let mut lock = self.tasks.write().await;
        let terminal: Vec<String> = lock
            .iter()
            .filter(|(_, task)| is_terminal(task.node.state))
            .map(|(id, _)| id.clone())
            .collect();

        let mut removed = Vec::new();
        for id in terminal {
            let parent_terminal = match lock.get(&id).and_then(|t| t.node.parent_id.clone()) {
                Some(parent) => lock
                    .get(&parent)
                    .map(|p| is_terminal(p.node.state))
                    .unwrap_or(true),
                None => true,
            };
            if parent_terminal && lock.remove(&id).is_some() {
                removed.push(id);
            }
        }
        removed
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

    async fn progress_of(center: &TaskCenter, id: &str) -> f32 {
        center
            .list_tasks()
            .await
            .into_iter()
            .find(|t| t.id == id)
            .map(|t| t.progress)
            .unwrap_or(-1.0)
    }

    #[tokio::test]
    async fn test_progress_window_maps_stages_and_never_rewinds() {
        let center = TaskCenter::new();
        let (id, _) = center
            .create_task(None, "tasks.titles.modpackInstall", HashMap::new())
            .await;
        center.start_task(&id).await.unwrap();

        // Stage 1 owns 0..8% of the bar.
        center.set_progress_window(&id, 0.0, 0.08).await.unwrap();
        center.update_progress(&id, 0.5, 0).await.unwrap();
        assert!((progress_of(&center, &id).await - 0.04).abs() < 1e-6);

        // Stage 2 owns 8%..70% and reports its own 0..1 scale.
        center.set_progress_window(&id, 0.08, 0.62).await.unwrap();
        center.update_progress(&id, 0.5, 0).await.unwrap();
        assert!((progress_of(&center, &id).await - 0.39).abs() < 1e-6);

        // A stage that restarts internally holds the bar instead of rewinding it.
        center.update_progress(&id, 0.1, 0).await.unwrap();
        assert!((progress_of(&center, &id).await - 0.39).abs() < 1e-6);

        // Terminal tasks ignore late reports.
        center.complete_task(&id).await.unwrap();
        center.update_progress(&id, 0.0, 0).await.unwrap();
        assert_eq!(progress_of(&center, &id).await, 1.0);
    }

    #[tokio::test]
    async fn has_active_tasks_tracks_only_live_work() {
        let center = TaskCenter::new();
        assert!(!center.has_active_tasks().await);

        let (id, _) = center
            .create_task(None, "task.download", HashMap::new())
            .await;
        // Waiting counts: its worker has not touched the disk yet, but it will.
        assert!(center.has_active_tasks().await);

        center.start_task(&id).await.unwrap();
        assert!(center.has_active_tasks().await);

        center.pause_task(&id).await.unwrap();
        assert!(center.has_active_tasks().await);

        center.resume_task(&id).await.unwrap();
        center.complete_task(&id).await.unwrap();
        assert!(!center.has_active_tasks().await);

        let (failed, _) = center
            .create_task(None, "task.download", HashMap::new())
            .await;
        center
            .fail_task(&failed, AppError::new("net.timeout"))
            .await
            .unwrap();
        assert!(!center.has_active_tasks().await);
    }

    #[tokio::test]
    async fn clear_finished_drops_only_closed_tasks_and_their_children() {
        let center = TaskCenter::new();
        let (done, _) = center
            .create_task(None, "task.download", HashMap::new())
            .await;
        center.start_task(&done).await.unwrap();
        center.complete_task(&done).await.unwrap();

        let (failed, _) = center
            .create_task(None, "task.download", HashMap::new())
            .await;
        center.start_task(&failed).await.unwrap();
        center
            .fail_task(&failed, AppError::new("net.timeout"))
            .await
            .unwrap();

        let (running, _) = center
            .create_task(None, "task.download", HashMap::new())
            .await;
        center.start_task(&running).await.unwrap();

        // A finished child under a live parent survives: its card is nested under work that is
        // still on screen.
        let (child, _) = center
            .create_task(Some(running.clone()), "task.download", HashMap::new())
            .await;
        center.complete_task(&child).await.unwrap();

        let removed = center.clear_finished().await;
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&done) && removed.contains(&failed));

        let left: Vec<String> = center
            .list_tasks()
            .await
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(left.len(), 2);
        assert!(left.contains(&running) && left.contains(&child));

        // Clearing again is a no-op; closing the parent releases everything.
        assert!(center.clear_finished().await.is_empty());
        center.cancel_task(&running).await.unwrap();
        assert_eq!(center.clear_finished().await.len(), 2);
        assert!(center.list_tasks().await.is_empty());
    }

    #[tokio::test]
    async fn pause_and_cancel_are_visible_to_workers() {
        let center = TaskCenter::new();
        let (id, control) = center
            .create_task(None, "task.download", HashMap::new())
            .await;
        center.start_task(&id).await.unwrap();

        // Pause: the control a worker holds must report it, and resume must clear it.
        center.pause_task(&id).await.unwrap();
        assert!(control.is_paused());
        center.resume_task(&id).await.unwrap();
        assert!(!control.is_paused());
        assert!(!control.is_cancelled());
        assert!(control.checkpoint().await.is_ok());

        // Cancel: checkpoint fails, and a late complete must not resurrect the task.
        center.cancel_task(&id).await.unwrap();
        assert!(control.is_cancelled());
        assert!(control.checkpoint().await.is_err());
        center.complete_task(&id).await.unwrap();

        let tasks = center.list_tasks().await;
        assert_eq!(tasks[0].state, TaskState::Canceled);
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
