//! Event batching: coalesces bursts of items into one emit per window.

use std::time::Duration;
use tokio::sync::mpsc;

pub const FLUSH_WINDOW: Duration = Duration::from_millis(50);
pub const FLUSH_MAX: usize = 64;

/// Handle for pushing items into a batcher. Cheap to clone; the flush task ends when all
/// senders are dropped.
#[derive(Clone)]
pub struct Batcher<T> {
    tx: mpsc::UnboundedSender<T>,
}

impl<T: Send + 'static> Batcher<T> {
    /// Spawn the flush task. `flush` receives every item pushed within `window` (or up to
    /// `max` items, whichever comes first), in push order. Idle batchers cost nothing.
    pub fn spawn(
        window: Duration,
        max: usize,
        flush: impl Fn(Vec<T>) + Send + 'static,
    ) -> Batcher<T> {
        let (tx, mut rx) = mpsc::unbounded_channel::<T>();
        tauri::async_runtime::spawn(async move {
            while let Some(first) = rx.recv().await {
                let mut batch = vec![first];
                let deadline = tokio::time::Instant::now() + window;
                while batch.len() < max {
                    match tokio::time::timeout_at(deadline, rx.recv()).await {
                        Ok(Some(item)) => batch.push(item),
                        Ok(None) | Err(_) => break,
                    }
                }
                flush(batch);
            }
        });
        Batcher { tx }
    }

    pub fn push(&self, item: T) {
        let _ = self.tx.send(item);
    }
}

/// `Batcher::spawn` with the default 50 ms / 64 item window.
pub fn spawn_default<T: Send + 'static>(flush: impl Fn(Vec<T>) + Send + 'static) -> Batcher<T> {
    Batcher::spawn(FLUSH_WINDOW, FLUSH_MAX, flush)
}
