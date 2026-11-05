//! TaskCenter coordinator, runtime state management, step transitions, and progress windowing.

use crate::AppError;
use crate::error::CoreError;
use crate::tasks::model::{
    StepNode, StepPlan, StepState, TaskNode, TaskPlan, TaskState, is_active, is_terminal,
};
use crate::tasks::store::{StoreMode, TaskRecord, TaskStore, TaskStoreStatus};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Normalizes path according to OS rules (Windows: replace `/` with `\`, strip leading `\\?\`, lowercase).
pub fn norm_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    #[cfg(windows)]
    {
        let stripped = s.strip_prefix(r"\\?\").unwrap_or(&s);
        stripped.replace('/', "\\").to_lowercase()
    }
    #[cfg(not(windows))]
    {
        s.into_owned()
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Shared handle a worker polls to honour pause/cancel requests.
#[derive(Clone)]
pub struct TaskControl {
    token: CancellationToken,
    paused: Arc<AtomicBool>,
}

impl TaskControl {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    /// True once the user cancelled the task.
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
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
            tokio::time::sleep(Duration::from_millis(120)).await;
            if self.token.is_cancelled() {
                return Err(TaskAborted);
            }
        }
        Ok(())
    }
}

impl Default for TaskControl {
    fn default() -> Self {
        Self::new()
    }
}

/// Returned by [`TaskControl::checkpoint`] once the task was cancelled.
#[derive(Debug, Clone, Copy)]
pub struct TaskAborted;

pub(crate) struct InternalTask {
    pub(crate) node: TaskNode,
    pub(crate) control: TaskControl,
    pub(crate) progress_window: (f32, f32),
    pub(crate) progress_high: f32,
    pub(crate) windowed: bool,
    pub(crate) payload: serde_json::Value,
    pub(crate) artifacts: Vec<String>,
    pub(crate) last_progress_upsert_at: Instant,
}

impl InternalTask {
    pub(crate) fn to_record(&self) -> TaskRecord {
        TaskRecord {
            schema: 1,
            node: self.node.clone(),
            payload: self.payload.clone(),
            artifacts: self.artifacts.clone(),
        }
    }
}

/// Thread-safe registry and coordinator for all long-running tasks.
pub struct TaskCenter {
    pub(crate) tasks: RwLock<HashMap<String, InternalTask>>,
    pub(crate) update_tx: broadcast::Sender<Vec<TaskNode>>,
    pub(crate) removed_tx: broadcast::Sender<Vec<String>>,
    pub(crate) status_tx: broadcast::Sender<TaskStoreStatus>,
    pub(crate) store: Option<Arc<TaskStore>>,
    pub(crate) status: RwLock<TaskStoreStatus>,
    pub(crate) seq: AtomicU64,
}

impl TaskCenter {
    pub fn new() -> Self {
        Self::with_store(
            None,
            TaskStoreStatus {
                mode: StoreMode::Memory,
                reset_notice: false,
            },
        )
    }
    pub fn with_store(store: Option<Arc<TaskStore>>, initial: TaskStoreStatus) -> Self {
        let (update_tx, _) = broadcast::channel(128);
        let (removed_tx, _) = broadcast::channel(64);
        let (status_tx, _) = broadcast::channel(16);
        Self {
            tasks: RwLock::new(HashMap::new()),
            update_tx,
            removed_tx,
            status_tx,
            store,
            status: RwLock::new(initial),
            seq: AtomicU64::new(1),
        }
    }

    pub fn init_seq(&self, val: u64) {
        let _ = self.seq.fetch_max(val, Ordering::SeqCst);
    }

    pub fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }

    /// Subscribes to task diff / update broadcasts (`Vec<TaskNode>`).
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<TaskNode>> {
        self.update_tx.subscribe()
    }

    /// Subscribes to task deletion broadcasts (`Vec<String>` of removed task IDs).
    pub fn subscribe_removed(&self) -> broadcast::Receiver<Vec<String>> {
        self.removed_tx.subscribe()
    }

    /// Subscribes to store status broadcasts (`TaskStoreStatus`).
    pub fn subscribe_status(&self) -> broadcast::Receiver<TaskStoreStatus> {
        self.status_tx.subscribe()
    }

    /// Current store status. Also detects and broadcasts fallback from persistent to memory mode.
    pub async fn store_status(&self) -> TaskStoreStatus {
        if let Some(store) = &self.store {
            if store.check_and_clear_status_changed() {
                let mut lock = self.status.write().await;
                lock.mode = store.current_mode();
                let status = lock.clone();
                let _ = self.status_tx.send(status.clone());
                return status;
            }
        }
        self.status.read().await.clone()
    }

    /// Acknowledges database reset notification, setting `reset_notice = false`.
    pub async fn ack_reset(&self) {
        let mut lock = self.status.write().await;
        lock.reset_notice = false;
        let status = lock.clone();
        let _ = self.status_tx.send(status);
    }

    /// Loads initial task records into the registry (used during startup recovery).
    pub async fn load_records(&self, records: Vec<TaskRecord>) {
        let mut max_seq = 0u64;
        let mut lock = self.tasks.write().await;
        for rec in records {
            if rec.node.queue_seq > max_seq {
                max_seq = rec.node.queue_seq;
            }
            let control = TaskControl::new();
            if rec.node.state == TaskState::Paused {
                control.set_paused(true);
            } else if rec.node.state == TaskState::Canceled {
                control.cancel();
            }
            lock.insert(
                rec.node.id.clone(),
                InternalTask {
                    node: rec.node,
                    control,
                    progress_window: (0.0, 1.0),
                    progress_high: 0.0,
                    windowed: false,
                    payload: rec.payload,
                    artifacts: rec.artifacts,
                    last_progress_upsert_at: Instant::now(),
                },
            );
        }
        self.init_seq(max_seq + 1);
    }

    /// Creates a new task in `Waiting` state and returns its assigned ID and `TaskControl`.
    pub async fn create_task(
        &self,
        kind: impl Into<String>,
        plan: TaskPlan,
    ) -> (String, TaskControl) {
        self.create_task_with_payload(kind, plan, serde_json::Value::Null, true, 1)
            .await
    }

    pub(crate) async fn create_task_with_payload(
        &self,
        kind: impl Into<String>,
        plan: TaskPlan,
        payload: serde_json::Value,
        resumable: bool,
        max_attempts: u32,
    ) -> (String, TaskControl) {
        let id = Uuid::new_v4().to_string();
        let control = TaskControl::new();

        // Normalize step weights
        let raw_steps = if plan.steps.is_empty() {
            vec![StepPlan {
                key: "run",
                weight: 1.0,
            }]
        } else {
            plan.steps
        };

        let positive_weights: Vec<f32> = raw_steps
            .iter()
            .map(|s| if s.weight <= 0.0 { 1.0 } else { s.weight })
            .collect();
        let total_weight: f32 = positive_weights.iter().sum();
        let norm_factor = if total_weight > 0.0 {
            total_weight
        } else {
            1.0
        };

        let steps: Vec<StepNode> = raw_steps
            .iter()
            .zip(positive_weights.iter())
            .map(|(s, &w)| StepNode {
                key: s.key.to_string(),
                state: StepState::Pending,
                weight: w / norm_factor,
                items_done: 0,
                items_total: 0,
                started_at_ms: None,
                finished_at_ms: None,
                error: None,
            })
            .collect();

        let node = TaskNode {
            id: id.clone(),
            kind: kind.into(),
            title_key: plan.title_key,
            title_params: plan.title_params,
            state: TaskState::Waiting,
            progress: 0.0,
            speed_bps: 0,
            steps,
            current_step: None,
            attempt: 0,
            max_attempts,
            next_retry_at_ms: None,
            resumable,
            queue_seq: self.next_seq(),
            created_at_ms: now_ms(),
            started_at_ms: None,
            finished_at_ms: None,
            error: None,
        };

        let internal = InternalTask {
            node: node.clone(),
            control: control.clone(),
            progress_window: (0.0, 1.0),
            progress_high: 0.0,
            windowed: false,
            payload,
            artifacts: Vec::new(),
            last_progress_upsert_at: Instant::now(),
        };

        if let Some(store) = &self.store {
            store.upsert(internal.to_record(), true);
        }

        {
            let mut lock = self.tasks.write().await;
            lock.insert(id.clone(), internal);
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
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            if task.node.state == TaskState::Canceled {
                return Ok(());
            }
            task.node.state = TaskState::Running;
            if task.node.started_at_ms.is_none() {
                task.node.started_at_ms = Some(now_ms());
            }

            // A task created with exactly one step auto-begins it in start_task
            if task.node.steps.len() == 1 {
                let key = task.node.steps[0].key.clone();
                let now = now_ms();
                task.node.steps[0].state = StepState::Running;
                if task.node.steps[0].started_at_ms.is_none() {
                    task.node.steps[0].started_at_ms = Some(now);
                }
                task.node.steps[0].error = None;
                task.node.current_step = Some(key);
                task.progress_window = (0.0, task.node.steps[0].weight);
                task.progress_high = 0.0;
                task.windowed = true;
            }

            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Marks a step as `Running`, setting window base and span according to step weights.
    ///
    /// Unknown key -> logs debug and returns `true`.
    /// Step already `Done` -> sets window to `(end_of_step, 0.0)` and returns `false`.
    /// Otherwise marks any currently `Running` step as `Done`, activates this step, and returns `true`.
    pub async fn begin_step(&self, id: &str, key: &str) -> bool {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        let mut result = true;

        {
            let mut lock = self.tasks.write().await;
            let task = match lock.get_mut(id) {
                Some(t) => t,
                None => return false,
            };

            let step_idx = match task.node.steps.iter().position(|s| s.key == key) {
                Some(idx) => idx,
                None => {
                    tracing::debug!("Task {} begin_step unknown key: {}", id, key);
                    return true;
                }
            };

            let now = now_ms();

            // Calculate base and end for step_idx
            let mut base = 0.0f32;
            for i in 0..step_idx {
                base += task.node.steps[i].weight;
            }
            let own_weight = task.node.steps[step_idx].weight;
            let end_of_step = base + own_weight;

            if task.node.steps[step_idx].state == StepState::Done {
                // Step already done: zero span window, leave states untouched, return false
                task.progress_window = (end_of_step, 0.0);
                task.progress_high = 0.0;
                task.windowed = true;
                return false;
            }

            // Mark any other running step as Done
            for s in &mut task.node.steps {
                if s.key != key && s.state == StepState::Running {
                    s.state = StepState::Done;
                    s.finished_at_ms = Some(now);
                }
            }

            let step = &mut task.node.steps[step_idx];
            step.state = StepState::Running;
            if step.started_at_ms.is_none() {
                step.started_at_ms = Some(now);
            }
            step.error = None;

            task.node.current_step = Some(key.to_string());
            task.progress_window = (base, own_weight);
            task.progress_high = 0.0;
            task.windowed = true;
            task.node.progress = task.node.progress.max(base);

            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
            result = true;
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        let _ = self.update_tx.send(diff);
        result
    }

    /// Marks a step as `Done`.
    pub async fn finish_step(&self, id: &str, key: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;

            let now = now_ms();
            if let Some(step) = task.node.steps.iter_mut().find(|s| s.key == key) {
                step.state = StepState::Done;
                step.finished_at_ms = Some(now);
            }

            if task.node.current_step.as_deref() == Some(key) {
                task.node.current_step = None;
            }

            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Marks a step as `Skipped`, advancing progress to the step's end.
    pub async fn skip_step(&self, id: &str, key: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;

            let now = now_ms();
            let mut base = 0.0f32;
            let mut own_weight = 0.0f32;
            for s in &mut task.node.steps {
                if s.key == key {
                    s.state = StepState::Skipped;
                    s.finished_at_ms = Some(now);
                    own_weight = s.weight;
                    break;
                }
                base += s.weight;
            }

            task.node.progress = task.node.progress.max(base + own_weight);
            if task.node.current_step.as_deref() == Some(key) {
                task.node.current_step = None;
            }

            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Sets items processed for the currently running step.
    pub async fn set_current_items(
        &self,
        id: &str,
        done: u32,
        total: u32,
    ) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;

            if let Some(current_key) = &task.node.current_step {
                if let Some(step) = task.node.steps.iter_mut().find(|s| &s.key == current_key) {
                    step.items_done = done;
                    step.items_total = total;
                    diff.push(task.node.clone());

                    if task.last_progress_upsert_at.elapsed() >= Duration::from_secs(1) {
                        task.last_progress_upsert_at = Instant::now();
                        to_upsert = Some(task.to_record());
                    }
                }
            }
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, false);
        }

        if !diff.is_empty() {
            let _ = self.update_tx.send(diff);
        }
        Ok(())
    }

    /// Restricts this task's progress reports to a slice of the bar. Transitional helper.
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
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
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
            task.node.progress = task.node.progress.max(mapped).clamp(0.0, 1.0);
            task.node.speed_bps = speed_bps;
            diff.push(task.node.clone());

            if task.last_progress_upsert_at.elapsed() >= Duration::from_secs(1) {
                task.last_progress_upsert_at = Instant::now();
                to_upsert = Some(task.to_record());
            }
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, false);
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
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            task.node.title_params.extend(params);
            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Marks a task as `Success` (progress set to 1.0, speed to 0, all steps Done).
    pub async fn complete_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            if is_terminal(task.node.state) {
                return Ok(());
            }
            let now = now_ms();
            task.node.state = TaskState::Success;
            task.node.progress = 1.0;
            task.node.speed_bps = 0;
            task.node.finished_at_ms = Some(now);
            task.node.current_step = None;
            for s in &mut task.node.steps {
                if s.state != StepState::Skipped {
                    s.state = StepState::Done;
                    if s.finished_at_ms.is_none() {
                        s.finished_at_ms = Some(now);
                    }
                }
            }
            task.artifacts.clear();
            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Marks a task as `Failed` with an error.
    pub async fn fail_task(&self, id: &str, error: AppError) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            if is_terminal(task.node.state) {
                return Ok(());
            }
            let now = now_ms();
            task.node.state = TaskState::Failed;
            task.node.error = Some(error.clone());
            task.node.speed_bps = 0;
            task.node.finished_at_ms = Some(now);
            for s in &mut task.node.steps {
                if s.state == StepState::Running {
                    s.state = StepState::Failed;
                    s.error = Some(error.clone());
                    s.finished_at_ms = Some(now);
                }
            }
            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        let _ = self.update_tx.send(diff);
        Ok(())
    }

    /// Pauses a task if it is currently `Running`.
    pub async fn pause_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            if task.node.state == TaskState::Running {
                task.node.state = TaskState::Paused;
                task.control.set_paused(true);
                diff.push(task.node.clone());
                to_upsert = Some(task.to_record());
            }
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        if !diff.is_empty() {
            let _ = self.update_tx.send(diff);
        }
        Ok(())
    }

    /// Resumes a paused task to `Running`.
    pub async fn resume_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;
            if task.node.state == TaskState::Paused {
                task.node.state = TaskState::Running;
                task.control.set_paused(false);
                diff.push(task.node.clone());
                to_upsert = Some(task.to_record());
            }
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        if !diff.is_empty() {
            let _ = self.update_tx.send(diff);
        }
        Ok(())
    }

    /// Cancels a task and triggers its `CancellationToken`.
    pub async fn cancel_task(&self, id: &str) -> Result<(), CoreError> {
        let mut diff = Vec::new();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;

            if task.node.state == TaskState::Interrupted {
                return Err(CoreError::TaskInvalidState(
                    "cannot cancel interrupted task".to_string(),
                ));
            }

            if is_terminal(task.node.state) {
                return Ok(());
            }

            task.node.state = TaskState::Canceled;
            task.node.speed_bps = 0;
            task.node.finished_at_ms = Some(now_ms());
            task.control.cancel();
            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }

        if !diff.is_empty() {
            let _ = self.update_tx.send(diff);
        }
        Ok(())
    }

    /// Registers a file artifact associated with a task.
    pub async fn add_artifact(&self, id: &str, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        let mut to_upsert = None;
        {
            let mut lock = self.tasks.write().await;
            if let Some(task) = lock.get_mut(id) {
                if !task.artifacts.contains(&path_str) {
                    task.artifacts.push(path_str);
                    to_upsert = Some(task.to_record());
                }
            }
        }
        if let (Some(store), Some(rec)) = (&self.store, to_upsert) {
            store.upsert(rec, true);
        }
    }

    /// Returns set of normalized paths that cannot be safely cleaned up by toolbox.
    pub async fn protected_paths(&self) -> HashSet<String> {
        let lock = self.tasks.read().await;
        let mut set = HashSet::new();
        for task in lock.values() {
            let is_protected = is_active(task.node.state)
                || task.node.state == TaskState::Interrupted
                || (task.node.state == TaskState::Failed && task.node.resumable);
            if is_protected {
                for art in &task.artifacts {
                    set.insert(norm_path(Path::new(art)));
                }
            }
        }
        set
    }

    /// Returns a list of all current tasks in the registry.
    pub async fn list_tasks(&self) -> Vec<TaskNode> {
        let lock = self.tasks.read().await;
        lock.values().map(|t| t.node.clone()).collect()
    }

    /// True while any task is in flight (`Waiting | Running | Paused | Retrying`).
    pub async fn has_active_tasks(&self) -> bool {
        let lock = self.tasks.read().await;
        lock.values().any(|task| is_active(task.node.state))
    }

    /// Drops every finished task (`Success | Failed | Canceled`) and deletes their artifacts.
    pub async fn clear_finished(&self) -> Vec<String> {
        let mut removed_ids = Vec::new();
        let mut artifacts_to_delete = Vec::new();

        {
            let mut lock = self.tasks.write().await;
            let terminal_ids: Vec<String> = lock
                .iter()
                .filter(|(_, t)| is_terminal(t.node.state))
                .map(|(id, _)| id.clone())
                .collect();

            for id in terminal_ids {
                if let Some(task) = lock.remove(&id) {
                    if task.node.state == TaskState::Failed
                        || task.node.state == TaskState::Canceled
                    {
                        artifacts_to_delete.extend(task.artifacts);
                    }
                    removed_ids.push(id);
                }
            }
        }

        for art in artifacts_to_delete {
            let _ = tokio::fs::remove_file(art).await;
        }

        if let Some(store) = &self.store {
            store.remove(removed_ids.clone());
        }

        if !removed_ids.is_empty() {
            let _ = self.removed_tx.send(removed_ids.clone());
        }

        removed_ids
    }

    /// Removes a specific task if it is terminal or Interrupted, deleting artifacts.
    pub async fn remove_task(&self, id: &str) -> Result<(), CoreError> {
        let mut artifacts_to_delete = Vec::new();
        {
            let mut lock = self.tasks.write().await;
            let task = lock
                .get(id)
                .ok_or_else(|| CoreError::TaskNotFound(id.to_string()))?;

            if !is_terminal(task.node.state) && task.node.state != TaskState::Interrupted {
                return Err(CoreError::TaskInvalidState(format!(
                    "cannot remove active task in state {:?}",
                    task.node.state
                )));
            }

            let task = lock.remove(id).unwrap();
            artifacts_to_delete = task.artifacts;
        }

        for art in artifacts_to_delete {
            let _ = tokio::fs::remove_file(art).await;
        }

        if let Some(store) = &self.store {
            store.remove(vec![id.to_string()]);
        }

        let _ = self.removed_tx.send(vec![id.to_string()]);
        Ok(())
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
    async fn create_test_task(center: &TaskCenter, title_key: &str) -> (String, TaskControl) {
        center
            .create_task(
                "test",
                TaskPlan {
                    title_key: title_key.into(),
                    title_params: HashMap::new(),
                    steps: vec![StepPlan {
                        key: "run",
                        weight: 1.0,
                    }],
                },
            )
            .await
    }

    #[tokio::test]
    async fn test_task_lifecycle_and_clamping() {
        let center = TaskCenter::new();
        let mut rx = center.subscribe();

        let (task_id, token) = create_test_task(&center, "task.install.vanilla").await;
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
        let (id, _) = create_test_task(&center, "tasks.titles.modpackInstall").await;
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
    async fn test_step_lifecycle_and_weights() {
        let center = TaskCenter::new();
        let (id, _) = center
            .create_task(
                "test",
                TaskPlan {
                    title_key: "task.test".into(),
                    title_params: HashMap::new(),
                    steps: vec![
                        StepPlan {
                            key: "step1",
                            weight: 1.0,
                        },
                        StepPlan {
                            key: "step2",
                            weight: 3.0,
                        },
                    ],
                },
            )
            .await;

        center.start_task(&id).await.unwrap();
        let tasks = center.list_tasks().await;
        assert_eq!(tasks[0].steps.len(), 2);
        assert_eq!(tasks[0].steps[0].weight, 0.25);
        assert_eq!(tasks[0].steps[1].weight, 0.75);

        // Begin step 1
        assert!(center.begin_step(&id, "step1").await);
        center.update_progress(&id, 0.5, 500).await.unwrap();
        // 0.5 of step 1 (0.25) = 0.125
        assert!((progress_of(&center, &id).await - 0.125).abs() < 1e-6);

        // Begin step 2 (auto completes step 1)
        assert!(center.begin_step(&id, "step2").await);
        let tasks = center.list_tasks().await;
        assert_eq!(tasks[0].steps[0].state, StepState::Done);
        assert_eq!(tasks[0].steps[1].state, StepState::Running);
        assert!(tasks[0].progress >= 0.25);

        // Replaying Done step returns false and preserves progress
        assert!(!center.begin_step(&id, "step1").await);
        assert!(progress_of(&center, &id).await >= 0.25);

        // Finish step 2
        center.finish_step(&id, "step2").await.unwrap();
        let tasks = center.list_tasks().await;
        assert_eq!(tasks[0].steps[1].state, StepState::Done);
        assert!(tasks[0].current_step.is_none());

        // Complete task marks all steps Done and progress 1.0
        center.complete_task(&id).await.unwrap();
        let tasks = center.list_tasks().await;
        assert_eq!(tasks[0].state, TaskState::Success);
        assert_eq!(tasks[0].progress, 1.0);
    }

    #[tokio::test]
    async fn has_active_tasks_tracks_only_live_work() {
        let center = TaskCenter::new();
        assert!(!center.has_active_tasks().await);

        let (id, _) = create_test_task(&center, "task.download").await;
        // Waiting counts: its worker has not touched the disk yet, but it will.
        assert!(center.has_active_tasks().await);

        center.start_task(&id).await.unwrap();
        assert!(center.has_active_tasks().await);

        center.pause_task(&id).await.unwrap();
        assert!(center.has_active_tasks().await);

        center.resume_task(&id).await.unwrap();
        center.complete_task(&id).await.unwrap();
        assert!(!center.has_active_tasks().await);

        let (failed, _) = create_test_task(&center, "task.download").await;
        center
            .fail_task(&failed, AppError::new("net.timeout"))
            .await
            .unwrap();
        assert!(!center.has_active_tasks().await);
    }

    #[tokio::test]
    async fn clear_finished_drops_all_terminal_tasks() {
        let center = TaskCenter::new();
        let (done, _) = create_test_task(&center, "task.download").await;
        center.start_task(&done).await.unwrap();
        center.complete_task(&done).await.unwrap();

        let (failed, _) = create_test_task(&center, "task.download").await;
        center.start_task(&failed).await.unwrap();
        center
            .fail_task(&failed, AppError::new("net.timeout"))
            .await
            .unwrap();

        let (running, _) = create_test_task(&center, "task.download").await;
        center.start_task(&running).await.unwrap();

        let removed = center.clear_finished().await;
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&done) && removed.contains(&failed));

        let left: Vec<String> = center
            .list_tasks()
            .await
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(left.len(), 1);
        assert!(left.contains(&running));
    }

    #[tokio::test]
    async fn pause_and_cancel_are_visible_to_workers() {
        let center = TaskCenter::new();
        let (id, control) = create_test_task(&center, "task.download").await;
        center.start_task(&id).await.unwrap();

        center.pause_task(&id).await.unwrap();
        assert!(control.is_paused());
        center.resume_task(&id).await.unwrap();
        assert!(!control.is_paused());
        assert!(!control.is_cancelled());
        assert!(control.checkpoint().await.is_ok());

        center.cancel_task(&id).await.unwrap();
        assert!(control.is_cancelled());
        assert!(control.checkpoint().await.is_err());
        center.complete_task(&id).await.unwrap();

        let tasks = center.list_tasks().await;
        assert_eq!(tasks[0].state, TaskState::Canceled);
    }
}
