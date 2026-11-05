//! Background task scheduler, concurrency queue, exponential backoff, and startup recovery.

use crate::AppError;
use crate::error::CoreError;
use crate::settings::TaskSettings;
use crate::tasks::center::{TaskCenter, now_ms};
use crate::tasks::handler::{NetProbe, TaskCtx, TaskHandler};
use crate::tasks::model::{StepState, TaskNode, TaskPlan, TaskState, is_terminal};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct TaskEngine {
    center: Arc<TaskCenter>,
    registry: RwLock<HashMap<&'static str, Arc<dyn TaskHandler>>>,
    settings: RwLock<TaskSettings>,
    probe: Arc<dyn NetProbe>,
    resumed_ids: RwLock<HashSet<String>>,
    tracked_ids: RwLock<HashSet<String>>,
}

impl TaskEngine {
    pub fn new(center: Arc<TaskCenter>, probe: Arc<dyn NetProbe>) -> Arc<Self> {
        Arc::new(Self {
            center,
            registry: RwLock::new(HashMap::new()),
            settings: RwLock::new(TaskSettings::default()),
            probe,
            resumed_ids: RwLock::new(HashSet::new()),
            tracked_ids: RwLock::new(HashSet::new()),
        })
    }

    pub fn center(&self) -> Arc<TaskCenter> {
        Arc::clone(&self.center)
    }

    pub async fn register(&self, handler: Arc<dyn TaskHandler>) {
        let mut reg = self.registry.write().await;
        reg.insert(handler.kind(), handler);
    }

    pub async fn apply_settings(self: &Arc<Self>, settings: TaskSettings) {
        let clamped = settings.clamped();
        {
            let mut s = self.settings.write().await;
            *s = clamped;
        }
        self.pump().await;
    }

    /// Startup recovery: marks in-flight tasks as Interrupted and validates handlers.
    pub async fn startup(self: &Arc<Self>) {
        let reg = self.registry.read().await;
        let mut diff = Vec::new();
        let mut to_upsert = Vec::new();

        {
            let mut lock = self.center.tasks.write().await;
            for task in lock.values_mut() {
                // Tasks with unknown kinds become Failed
                if !reg.contains_key(task.node.kind.as_str()) && !is_terminal(task.node.state) {
                    task.node.state = TaskState::Failed;
                    task.node.error = Some(AppError::new("task.unknownKind"));
                    task.node.speed_bps = 0;
                    task.node.finished_at_ms = Some(now_ms());
                    diff.push(task.node.clone());
                    to_upsert.push(task.to_record());
                    continue;
                }

                // In-flight tasks become Interrupted
                if matches!(
                    task.node.state,
                    TaskState::Waiting
                        | TaskState::Running
                        | TaskState::Paused
                        | TaskState::Retrying
                ) {
                    task.node.state = TaskState::Interrupted;
                    task.node.speed_bps = 0;
                    task.node.next_retry_at_ms = None;
                    task.control.set_paused(false);

                    for step in &mut task.node.steps {
                        if step.state == StepState::Running {
                            step.state = StepState::Pending;
                        }
                    }

                    diff.push(task.node.clone());
                    to_upsert.push(task.to_record());
                }
            }
        }

        for rec in to_upsert {
            if let Some(store) = &self.center.store {
                store.upsert(rec, true);
            }
        }

        if !diff.is_empty() {
            let _ = self.center.update_tx.send(diff);
        }

        self.prune_history().await;
    }

    /// Submits a new background task for execution.
    pub async fn submit(
        self: &Arc<Self>,
        kind: &str,
        payload: serde_json::Value,
    ) -> Result<String, AppError> {
        let handler = {
            let reg = self.registry.read().await;
            reg.get(kind).cloned()
        };

        let handler = match handler {
            Some(h) => h,
            None => return Err(AppError::new("task.unknownKind")),
        };

        let plan = handler.plan(&payload)?;
        let max_attempts = self.settings.read().await.auto_retry + 1;
        let resumable = handler.resumable();

        let (id, _) = self
            .center
            .create_task_with_payload(kind, plan, payload, resumable, max_attempts)
            .await;

        self.pump().await;
        Ok(id)
    }

    /// Begins a tracked task that bypasses queueing and concurrency limits (e.g. launch or auth).
    pub async fn begin_tracked(&self, kind: &'static str, plan: TaskPlan) -> TaskCtx {
        let (id, control) = self
            .center
            .create_task_with_payload(kind, plan, serde_json::Value::Null, false, 1)
            .await;

        let _ = self.center.start_task(&id).await;
        self.tracked_ids.write().await.insert(id.clone());

        TaskCtx::new(
            id,
            false,
            0,
            serde_json::Value::Null,
            Arc::clone(&self.center),
            control,
        )
    }

    /// Restores an Interrupted task to Waiting state.
    pub async fn restore(self: &Arc<Self>, id: &str) -> Result<(), AppError> {
        let reg = self.registry.read().await;
        let mut diff = Vec::new();
        let mut to_upsert = None;

        {
            let mut lock = self.center.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| AppError::from(CoreError::TaskNotFound(id.to_string())))?;

            if task.node.state != TaskState::Interrupted {
                return Err(AppError::from(CoreError::TaskInvalidState(
                    "task is not interrupted".to_string(),
                )));
            }

            if !task.node.resumable {
                return Err(AppError::new("task.notResumable"));
            }

            if !reg.contains_key(task.node.kind.as_str()) {
                return Err(AppError::new("task.unknownKind"));
            }

            task.node.state = TaskState::Waiting;
            task.node.attempt = 0;
            task.node.error = None;
            task.node.next_retry_at_ms = None;
            task.node.speed_bps = 0;
            task.node.queue_seq = self.center.next_seq();

            let mut done_weight = 0.0f32;
            for step in &mut task.node.steps {
                if step.state == StepState::Done {
                    done_weight += step.weight;
                } else {
                    step.state = StepState::Pending;
                    step.items_done = 0;
                    step.items_total = 0;
                    step.started_at_ms = None;
                    step.finished_at_ms = None;
                    step.error = None;
                }
            }

            task.node.progress = done_weight.clamp(0.0, 1.0);
            task.node.current_step = None;
            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        self.resumed_ids.write().await.insert(id.to_string());

        if let (Some(store), Some(rec)) = (&self.center.store, to_upsert) {
            store.upsert(rec, true);
        }

        if !diff.is_empty() {
            let _ = self.center.update_tx.send(diff);
        }

        self.pump().await;
        Ok(())
    }

    /// Restores all Interrupted and resumable tasks.
    pub async fn restore_all(self: &Arc<Self>) -> Vec<String> {
        let interrupted_ids: Vec<String> = {
            let lock = self.center.tasks.read().await;
            lock.values()
                .filter(|t| t.node.state == TaskState::Interrupted && t.node.resumable)
                .map(|t| t.node.id.clone())
                .collect()
        };

        let mut restored = Vec::new();
        for id in interrupted_ids {
            if self.restore(&id).await.is_ok() {
                restored.push(id);
            }
        }
        restored
    }

    /// Retries a Failed task if it is resumable.
    pub async fn retry(self: &Arc<Self>, id: &str) -> Result<(), AppError> {
        let reg = self.registry.read().await;
        let mut diff = Vec::new();
        let mut to_upsert = None;

        {
            let mut lock = self.center.tasks.write().await;
            let task = lock
                .get_mut(id)
                .ok_or_else(|| AppError::from(CoreError::TaskNotFound(id.to_string())))?;

            if task.node.state != TaskState::Failed {
                return Err(AppError::from(CoreError::TaskInvalidState(
                    "task is not failed".to_string(),
                )));
            }

            if !task.node.resumable {
                return Err(AppError::new("task.notResumable"));
            }

            if !reg.contains_key(task.node.kind.as_str()) {
                return Err(AppError::new("task.unknownKind"));
            }

            task.node.state = TaskState::Waiting;
            task.node.attempt = 0;
            task.node.error = None;
            task.node.next_retry_at_ms = None;
            task.node.speed_bps = 0;
            task.node.queue_seq = self.center.next_seq();

            let mut done_weight = 0.0f32;
            for step in &mut task.node.steps {
                if step.state == StepState::Done {
                    done_weight += step.weight;
                } else {
                    step.state = StepState::Pending;
                    step.items_done = 0;
                    step.items_total = 0;
                    step.started_at_ms = None;
                    step.finished_at_ms = None;
                    step.error = None;
                }
            }

            task.node.progress = done_weight.clamp(0.0, 1.0);
            task.node.current_step = None;
            diff.push(task.node.clone());
            to_upsert = Some(task.to_record());
        }

        self.resumed_ids.write().await.insert(id.to_string());

        if let (Some(store), Some(rec)) = (&self.center.store, to_upsert) {
            store.upsert(rec, true);
        }

        if !diff.is_empty() {
            let _ = self.center.update_tx.send(diff);
        }

        self.pump().await;
        Ok(())
    }

    /// Removes a task if it is terminal or Interrupted, deleting its stored artifacts.
    pub async fn remove(&self, id: &str) -> Result<(), AppError> {
        self.center.remove_task(id).await.map_err(AppError::from)
    }

    /// Reorders the queue sequence of Waiting tasks.
    pub async fn reorder(&self, ordered_ids: Vec<String>) {
        let mut diff = Vec::new();
        let mut to_upsert = Vec::new();

        {
            let mut lock = self.center.tasks.write().await;
            // Filter to only those that exist and are Waiting
            let valid_ids: Vec<String> = ordered_ids
                .into_iter()
                .filter(|id| {
                    lock.get(id)
                        .map(|t| t.node.state == TaskState::Waiting)
                        .unwrap_or(false)
                })
                .collect();

            if valid_ids.is_empty() {
                return;
            }

            let mut seqs: Vec<u64> = valid_ids
                .iter()
                .filter_map(|id| lock.get(id).map(|t| t.node.queue_seq))
                .collect();
            seqs.sort_unstable();

            for (id, seq) in valid_ids.into_iter().zip(seqs) {
                if let Some(task) = lock.get_mut(&id) {
                    task.node.queue_seq = seq;
                    diff.push(task.node.clone());
                    to_upsert.push(task.to_record());
                }
            }
        }

        for rec in to_upsert {
            if let Some(store) = &self.center.store {
                store.upsert(rec, true);
            }
        }

        if !diff.is_empty() {
            let _ = self.center.update_tx.send(diff);
        }
    }
    pub fn spawn_pump(self: &Arc<Self>) {
        let engine = Arc::clone(self);
        tokio::spawn(async move {
            engine.pump().await;
        });
    }

    /// Pumps the scheduler queue, starting waiting tasks up to `max_concurrent`.
    pub async fn pump(self: &Arc<Self>) {
        let max_concurrent = self.settings.read().await.max_concurrent;
        let tracked = self.tracked_ids.read().await.clone();
        loop {
            let next_waiting = {
                let mut lock = self.center.tasks.write().await;
                let running = lock
                    .values()
                    .filter(|t| t.node.state == TaskState::Running && !tracked.contains(&t.node.id))
                    .count();

                if running >= max_concurrent as usize {
                    None
                } else {
                    let mut waiting: Vec<&mut crate::tasks::center::InternalTask> = lock
                        .values_mut()
                        .filter(|t| {
                            t.node.state == TaskState::Waiting && !tracked.contains(&t.node.id)
                        })
                        .collect();

                    waiting.sort_by_key(|t| t.node.queue_seq);
                    if let Some(task) = waiting.into_iter().next() {
                        task.node.state = TaskState::Running;
                        if task.node.started_at_ms.is_none() {
                            task.node.started_at_ms = Some(now_ms());
                        }
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
                        let id = task.node.id.clone();
                        let rec = task.to_record();
                        let node = task.node.clone();
                        drop(lock);
                        if let Some(store) = &self.center.store {
                            store.upsert(rec, true);
                        }
                        let _ = self.center.update_tx.send(vec![node]);
                        Some(id)
                    } else {
                        None
                    }
                }
            };
            match next_waiting {
                Some(id) => {
                    let engine = Arc::clone(self);
                    tokio::spawn(async move {
                        engine.run_task(id).await;
                    });
                }
                None => break,
            }
        }
    }

    async fn run_task(self: Arc<Self>, id: String) {
        let task_info = {
            let lock = self.center.tasks.read().await;
            lock.get(&id).map(|t| {
                (
                    t.node.kind.clone(),
                    t.payload.clone(),
                    t.control.clone(),
                    t.node.attempt,
                    t.node.max_attempts,
                )
            })
        };

        let (kind, payload, control, is_resumed, max_attempts) = match task_info {
            Some(info) => info,
            None => return,
        };

        let handler = {
            let reg = self.registry.read().await;
            reg.get(kind.as_str()).cloned()
        };

        let handler = match handler {
            Some(h) => h,
            None => {
                let _ = self
                    .center
                    .fail_task(&id, AppError::new("task.unknownKind"))
                    .await;
                return;
            }
        };

        let _ = self.center.start_task(&id).await;

        let resumed = self.resumed_ids.write().await.remove(&id);
        let mut attempt = is_resumed;

        loop {
            let ctx = TaskCtx::new(
                id.clone(),
                resumed,
                attempt,
                payload.clone(),
                Arc::clone(&self.center),
                control.clone(),
            );

            let run_res = handler.run(&ctx).await;
            match run_res {
                Ok(()) => {
                    let _ = self.center.complete_task(&id).await;
                    break;
                }
                Err(e) => {
                    if control.is_cancelled() {
                        let _ = self.center.cancel_task(&id).await;
                        break;
                    }

                    if self.is_retryable(&e) && attempt + 1 < max_attempts {
                        attempt += 1;
                        let delay_ms = 1000u64 << attempt;
                        let next_retry_at_ms = now_ms() + delay_ms;

                        {
                            let mut diff = Vec::new();
                            let mut to_upsert = None;
                            {
                                let mut lock = self.center.tasks.write().await;
                                if let Some(t) = lock.get_mut(&id) {
                                    t.node.state = TaskState::Retrying;
                                    t.node.attempt = attempt;
                                    t.node.error = Some(e.clone());
                                    t.node.next_retry_at_ms = Some(next_retry_at_ms);
                                    t.node.speed_bps = 0;
                                    diff.push(t.node.clone());
                                    to_upsert = Some(t.to_record());
                                }
                            }
                            if let (Some(store), Some(rec)) = (&self.center.store, to_upsert) {
                                store.upsert(rec, true);
                            }
                            if !diff.is_empty() {
                                let _ = self.center.update_tx.send(diff);
                            }
                        }

                        // Sleep racing cancellation token
                        let cancel_token = control.cancellation_token();
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {}
                            _ = cancel_token.cancelled() => {
                                let _ = self.center.cancel_task(&id).await;
                                break;
                            }
                        }
                        if control.is_cancelled() {
                            let _ = self.center.cancel_task(&id).await;
                            break;
                        }

                        // Clear next_retry_at_ms -> indicates waiting for network
                        {
                            let mut diff = Vec::new();
                            let mut to_upsert = None;
                            {
                                let mut lock = self.center.tasks.write().await;
                                if let Some(t) = lock.get_mut(&id) {
                                    t.node.next_retry_at_ms = None;
                                    diff.push(t.node.clone());
                                    to_upsert = Some(t.to_record());
                                }
                            }
                            if let (Some(store), Some(rec)) = (&self.center.store, to_upsert) {
                                store.upsert(rec, true);
                            }
                            if !diff.is_empty() {
                                let _ = self.center.update_tx.send(diff);
                            }
                        }

                        // Poll probe until online or cancelled
                        while !self.probe.online().await {
                            let cancel_token = control.cancellation_token();
                            tokio::select! {
                                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                                _ = cancel_token.cancelled() => {
                                    break;
                                }
                            }
                            if control.is_cancelled() {
                                break;
                            }
                        }

                        if control.is_cancelled() {
                            let _ = self.center.cancel_task(&id).await;
                            break;
                        }

                        // Retrying: loop again to execute run()
                        let _ = self.center.start_task(&id).await;
                        continue;
                    } else {
                        let _ = self.center.fail_task(&id, e).await;
                        break;
                    }
                }
            }
        }
        self.spawn_pump();
        self.prune_history().await;
    }

    pub fn is_retryable(&self, error: &AppError) -> bool {
        matches!(
            error.code.as_str(),
            "net.http" | "net.timeout" | "net.io" | "net.sourcesExhausted" | "net.racingFailed"
        )
    }

    pub async fn prune_history(&self) {
        let (history_max, history_days) = {
            let s = self.settings.read().await;
            (s.history_max as usize, s.history_days as u64)
        };

        let now = now_ms();
        let max_age_ms = history_days.saturating_mul(86_400_000);
        let mut to_remove = Vec::new();

        {
            let lock = self.center.tasks.read().await;
            let mut terminal: Vec<&TaskNode> = lock
                .values()
                .filter(|t| is_terminal(t.node.state))
                .map(|t| &t.node)
                .collect();

            // Prune tasks older than history_days
            for t in &terminal {
                if let Some(finished_at) = t.finished_at_ms {
                    if now.saturating_sub(finished_at) > max_age_ms {
                        to_remove.push(t.id.clone());
                    }
                }
            }

            // Prune extras exceeding history_max (oldest finished first)
            terminal.retain(|t| !to_remove.contains(&t.id));
            if terminal.len() > history_max {
                terminal.sort_by_key(|t| t.finished_at_ms.unwrap_or(0));
                let extra = terminal.len() - history_max;
                for t in terminal.into_iter().take(extra) {
                    to_remove.push(t.id.clone());
                }
            }
        }

        for id in to_remove {
            let _ = self.center.remove_task(&id).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::handler::TaskHandler;
    use crate::tasks::model::StepPlan;
    use crate::tasks::store::TaskStore;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use tempfile::tempdir;
    use tokio::sync::Notify;

    struct AlwaysOnline;
    #[async_trait]
    impl NetProbe for AlwaysOnline {
        async fn online(&self) -> bool {
            true
        }
    }

    struct MockProbe(Arc<AtomicBool>);
    #[async_trait]
    impl NetProbe for MockProbe {
        async fn online(&self) -> bool {
            self.0.load(Ordering::SeqCst)
        }
    }

    struct BlockHandler {
        start_tx: Arc<Notify>,
        release_rx: Arc<Notify>,
    }
    #[async_trait]
    impl TaskHandler for BlockHandler {
        fn kind(&self) -> &'static str {
            "test.block"
        }
        fn plan(&self, _payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
            Ok(TaskPlan {
                title_key: "test.block".into(),
                title_params: HashMap::new(),
                steps: vec![StepPlan {
                    key: "run",
                    weight: 1.0,
                }],
            })
        }
        async fn run(&self, _ctx: &TaskCtx) -> Result<(), AppError> {
            self.start_tx.notify_one();
            self.release_rx.notified().await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_concurrency_queue_and_reorder() {
        let center = Arc::new(TaskCenter::new());
        let engine = TaskEngine::new(center, Arc::new(AlwaysOnline));

        engine
            .apply_settings(TaskSettings {
                max_concurrent: 1,
                auto_retry: 0,
                notify: false,
                history_max: 50,
                history_days: 1,
            })
            .await;

        let a_started = Arc::new(Notify::new());
        let a_release = Arc::new(Notify::new());
        engine
            .register(Arc::new(BlockHandler {
                start_tx: Arc::clone(&a_started),
                release_rx: Arc::clone(&a_release),
            }))
            .await;

        let _id_a = engine
            .submit("test.block", serde_json::Value::Null)
            .await
            .unwrap();
        a_started.notified().await;

        let id_b = engine
            .submit("test.block", serde_json::Value::Null)
            .await
            .unwrap();
        let id_c = engine
            .submit("test.block", serde_json::Value::Null)
            .await
            .unwrap();

        let tasks = engine.center().list_tasks().await;
        let state_b = tasks.iter().find(|t| t.id == id_b).unwrap().state;
        let state_c = tasks.iter().find(|t| t.id == id_c).unwrap().state;
        assert_eq!(state_b, TaskState::Waiting);
        assert_eq!(state_c, TaskState::Waiting);

        // Reorder [C, B] so C runs first after A completes
        engine.reorder(vec![id_c.clone(), id_b.clone()]).await;

        let tasks_after = engine.center().list_tasks().await;
        let seq_b = tasks_after.iter().find(|t| t.id == id_b).unwrap().queue_seq;
        let seq_c = tasks_after.iter().find(|t| t.id == id_c).unwrap().queue_seq;
        assert!(seq_c < seq_b);

        // Release A
        a_release.notify_waiters();
    }

    struct FlakyHandler {
        failures: Arc<AtomicU32>,
    }
    #[async_trait]
    impl TaskHandler for FlakyHandler {
        fn kind(&self) -> &'static str {
            "test.flaky"
        }
        fn plan(&self, _payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
            Ok(TaskPlan {
                title_key: "test.flaky".into(),
                title_params: HashMap::new(),
                steps: vec![StepPlan {
                    key: "run",
                    weight: 1.0,
                }],
            })
        }
        async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
            let count = self.failures.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(AppError::new("net.timeout"))
            } else {
                ctx.begin_step("run").await;
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_auto_retry_and_net_probe() {
        let center = Arc::new(TaskCenter::new());
        let probe_online = Arc::new(AtomicBool::new(false));
        let engine = TaskEngine::new(center, Arc::new(MockProbe(Arc::clone(&probe_online))));

        engine
            .apply_settings(TaskSettings {
                max_concurrent: 1,
                auto_retry: 3,
                notify: false,
                history_max: 50,
                history_days: 1,
            })
            .await;

        let failures = Arc::new(AtomicU32::new(0));
        engine
            .register(Arc::new(FlakyHandler {
                failures: Arc::clone(&failures),
            }))
            .await;

        let id = engine
            .submit("test.flaky", serde_json::Value::Null)
            .await
            .unwrap();

        // Wait until task reaches Retrying
        let mut retried = false;
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let tasks = engine.center().list_tasks().await;
            if let Some(t) = tasks.iter().find(|t| t.id == id)
                && t.state == TaskState::Retrying
            {
                retried = true;
                break;
            }
        }
        assert!(retried, "Task should enter Retrying state");

        // Set probe to online so retry proceeds
        probe_online.store(true, Ordering::SeqCst);

        // Wait for final success
        let mut success = false;
        for _ in 0..100 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let tasks = engine.center().list_tasks().await;
            if let Some(t) = tasks.iter().find(|t| t.id == id)
                && t.state == TaskState::Success
            {
                assert_eq!(t.attempt, 2);
                success = true;
                break;
            }
        }
        assert!(success, "Task should eventually succeed after retries");
    }

    struct ResumableTestHandler;
    #[async_trait]
    impl TaskHandler for ResumableTestHandler {
        fn kind(&self) -> &'static str {
            "test.resumable"
        }
        fn plan(&self, _payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
            Ok(TaskPlan {
                title_key: "test.resumable".into(),
                title_params: HashMap::new(),
                steps: vec![StepPlan {
                    key: "run",
                    weight: 1.0,
                }],
            })
        }
        async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
            assert!(ctx.resumed);
            Ok(())
        }
    }

    struct NonResumableHandler;
    #[async_trait]
    impl TaskHandler for NonResumableHandler {
        fn kind(&self) -> &'static str {
            "test.nonresumable"
        }
        fn resumable(&self) -> bool {
            false
        }
        fn plan(&self, _payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
            Ok(TaskPlan {
                title_key: "test.nonresumable".into(),
                title_params: HashMap::new(),
                steps: vec![StepPlan {
                    key: "run",
                    weight: 1.0,
                }],
            })
        }
        async fn run(&self, _ctx: &TaskCtx) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_persistence_restart_and_restore() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.redb");

        let (store_a, records, status) = TaskStore::open(&db_path);
        assert_eq!(records.len(), 0);
        let center_a = Arc::new(TaskCenter::with_store(store_a, status));
        let engine_a = TaskEngine::new(center_a, Arc::new(AlwaysOnline));
        engine_a
            .apply_settings(TaskSettings {
                max_concurrent: 1,
                auto_retry: 0,
                notify: false,
                history_max: 50,
                history_days: 1,
            })
            .await;

        let block_started = Arc::new(Notify::new());
        let block_release = Arc::new(Notify::new());
        let blocker = Arc::new(BlockHandler {
            start_tx: Arc::clone(&block_started),
            release_rx: Arc::clone(&block_release),
        });
        engine_a.register(blocker).await;
        engine_a.register(Arc::new(ResumableTestHandler)).await;
        engine_a.register(Arc::new(NonResumableHandler)).await;

        let _id_block = engine_a
            .submit("test.block", serde_json::Value::Null)
            .await
            .unwrap();
        block_started.notified().await;

        let id_resumable = engine_a
            .submit("test.resumable", serde_json::Value::Null)
            .await
            .unwrap();
        let id_nonresumable = engine_a
            .submit("test.nonresumable", serde_json::Value::Null)
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(600)).await;
        block_release.notify_waiters();
        tokio::time::sleep(Duration::from_millis(150)).await;
        drop(engine_a);

        // Reopen DB on second engine
        let (store_b, records_b, status_b) = TaskStore::open(&db_path);
        assert!(records_b.len() >= 2);
        let center_b = Arc::new(TaskCenter::with_store(store_b, status_b));
        center_b.load_records(records_b).await;

        let engine_b = TaskEngine::new(center_b, Arc::new(AlwaysOnline));
        engine_b.register(Arc::new(ResumableTestHandler)).await;
        engine_b.register(Arc::new(NonResumableHandler)).await;
        engine_b.startup().await;

        let tasks = engine_b.center().list_tasks().await;
        let t_res = tasks.iter().find(|t| t.id == id_resumable).unwrap();
        let t_non = tasks.iter().find(|t| t.id == id_nonresumable).unwrap();
        assert_eq!(t_res.state, TaskState::Interrupted);
        assert_eq!(t_non.state, TaskState::Interrupted);

        // Non-resumable restore returns task.notResumable
        let err = engine_b.restore(&id_nonresumable).await.unwrap_err();
        assert_eq!(err.code, "task.notResumable");

        // Resumable restore succeeds
        assert!(engine_b.restore(&id_resumable).await.is_ok());
    }

    #[tokio::test]
    async fn test_corrupt_db_quarantined_and_fresh_db_created() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.redb");

        // Write garbage bytes
        std::fs::write(&db_path, b"not a valid redb file!").unwrap();

        let (store, records, status) = TaskStore::open(&db_path);
        assert!(
            status.reset_notice,
            "reset_notice should be true on corrupt DB"
        );
        assert_eq!(records.len(), 0);
        assert!(store.is_some(), "Fresh store should be created");

        // Verify quarantine file exists
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            entries
                .iter()
                .any(|name| name.starts_with("tasks.redb.corrupt-")),
            "Quarantine corrupt file should exist"
        );
    }

    #[tokio::test]
    async fn test_step_progress_cumulative_weights_and_no_regression() {
        let center = Arc::new(TaskCenter::new());
        let (id, _) = center
            .create_task(
                "steps.test",
                TaskPlan {
                    title_key: "test.steps".into(),
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
        assert!(center.begin_step(&id, "step1").await);
        center.update_progress(&id, 1.0, 100).await.unwrap();
        center.finish_step(&id, "step1").await.unwrap();

        // Progress at completion of step 1 is 0.25
        let tasks = center.list_tasks().await;
        assert_eq!(tasks[0].steps[0].state, StepState::Done);
        assert!((tasks[0].progress - 0.25).abs() < 1e-5);

        // Replaying step1 returns false and never lowers progress
        assert!(!center.begin_step(&id, "step1").await);
        center.update_progress(&id, 0.0, 100).await.unwrap();
        let tasks = center.list_tasks().await;
        assert!((tasks[0].progress - 0.25).abs() < 1e-5);

        // Step 2 starts with progress >= 0.25
        assert!(center.begin_step(&id, "step2").await);
        let tasks = center.list_tasks().await;
        assert!(tasks[0].progress >= 0.25);
    }
}
