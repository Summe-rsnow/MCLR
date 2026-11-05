//! Persistent storage for task records backed by redb.

use crate::tasks::model::TaskNode;
use redb::{
    Database, DatabaseError, ReadableDatabase, ReadableTable, ReadableTableMetadata,
    TableDefinition,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TASKS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tasks");
const META_TABLE: TableDefinition<&str, u32> = TableDefinition::new("meta");
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StoreMode {
    Persistent,
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskStoreStatus {
    pub mode: StoreMode,
    pub reset_notice: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub schema: u32,
    pub node: TaskNode,
    pub payload: serde_json::Value,
    pub artifacts: Vec<String>,
}

enum StoreCmd {
    Upsert {
        record: Box<TaskRecord>,
        urgent: bool,
    },
    Remove {
        ids: Vec<String>,
    },
}

pub struct TaskStore {
    tx: Mutex<Option<Sender<StoreCmd>>>,
    mode: Arc<Mutex<StoreMode>>,
    status_changed: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

impl Drop for TaskStore {
    fn drop(&mut self) {
        if let Ok(mut tx_guard) = self.tx.lock() {
            tx_guard.take();
        }
        if let Ok(mut worker_guard) = self.worker.lock() {
            if let Some(handle) = worker_guard.take() {
                let _ = handle.join();
            }
        }
    }
}

impl TaskStore {
    pub fn open(path: &Path) -> (Option<Arc<TaskStore>>, Vec<TaskRecord>, TaskStoreStatus) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut reset_notice = false;

        let db = match open_or_recover(path, &mut reset_notice) {
            Ok(db) => db,
            Err(e) => {
                tracing::warn!("Failed to open task database at {:?}: {:?}", path, e);
                return (
                    None,
                    Vec::new(),
                    TaskStoreStatus {
                        mode: StoreMode::Memory,
                        reset_notice,
                    },
                );
            }
        };

        let (records, broken_keys) = read_records(&db);
        if !broken_keys.is_empty() {
            if let Ok(write_txn) = db.begin_write() {
                if let Ok(mut table) = write_txn.open_table(TASKS_TABLE) {
                    for key in &broken_keys {
                        let _ = table.remove(key.as_str());
                    }
                }
                let _ = write_txn.commit();
            }
        }

        let (tx, rx) = channel::<StoreCmd>();
        let mode = Arc::new(Mutex::new(StoreMode::Persistent));
        let status_changed = Arc::new(AtomicBool::new(false));

        let worker_path = path.to_path_buf();
        let worker_mode = Arc::clone(&mode);
        let worker_status_changed = Arc::clone(&status_changed);

        let builder = thread::Builder::new().name("mclr-task-store".to_string());
        let worker_handle = builder
            .spawn(move || {
                run_writer(worker_path, db, rx, worker_mode, worker_status_changed);
            })
            .ok();

        let store = Arc::new(TaskStore {
            tx: Mutex::new(Some(tx)),
            mode,
            status_changed,
            worker: Mutex::new(worker_handle),
        });

        let status = TaskStoreStatus {
            mode: StoreMode::Persistent,
            reset_notice,
        };

        (Some(store), records, status)
    }

    pub fn upsert(&self, record: TaskRecord, urgent: bool) {
        if let Ok(guard) = self.tx.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(StoreCmd::Upsert {
                    record: Box::new(record),
                    urgent,
                });
            }
        }
    }

    pub fn remove(&self, ids: Vec<String>) {
        if !ids.is_empty() {
            if let Ok(guard) = self.tx.lock() {
                if let Some(tx) = guard.as_ref() {
                    let _ = tx.send(StoreCmd::Remove { ids });
                }
            }
        }
    }

    pub fn current_mode(&self) -> StoreMode {
        *self.mode.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn check_and_clear_status_changed(&self) -> bool {
        self.status_changed.swap(false, Ordering::AcqRel)
    }
}

fn open_or_recover(path: &Path, reset_notice: &mut bool) -> Result<Database, ()> {
    match redb::Builder::new().create(path) {
        Ok(db) => {
            if validate_and_init_db(&db) {
                return Ok(db);
            }
            drop(db);
            *reset_notice = true;
            quarantine_corrupt(path);
            open_fresh(path)
        }
        Err(DatabaseError::DatabaseAlreadyOpen) => {
            tracing::warn!(
                "Task database {:?} is already open by another process",
                path
            );
            Err(())
        }
        Err(e) => {
            tracing::warn!("Error opening task database {:?}: {:?}", path, e);
            *reset_notice = true;
            quarantine_corrupt(path);
            open_fresh(path)
        }
    }
}

fn open_fresh(path: &Path) -> Result<Database, ()> {
    let db = redb::Builder::new().create(path).map_err(|e| {
        tracing::error!("Failed to create fresh task database {:?}: {:?}", path, e);
    })?;
    if init_schema(&db) { Ok(db) } else { Err(()) }
}

fn validate_and_init_db(db: &Database) -> bool {
    let read_txn = match db.begin_read() {
        Ok(t) => t,
        Err(_) => return false,
    };
    let meta_table = match read_txn.open_table(META_TABLE) {
        Ok(m) => m,
        Err(_) => {
            drop(read_txn);
            let is_empty = match db.begin_read() {
                Ok(t) => match t.open_table(TASKS_TABLE) {
                    Ok(tbl) => tbl.is_empty().unwrap_or(true),
                    Err(_) => true,
                },
                Err(_) => true,
            };
            if is_empty {
                return init_schema(db);
            }
            return false;
        }
    };

    let schema_val = meta_table.get("schema").ok().flatten().map(|v| v.value());
    if schema_val == Some(SCHEMA_VERSION) {
        true
    } else {
        false
    }
}

fn init_schema(db: &Database) -> bool {
    let write_txn = match db.begin_write() {
        Ok(t) => t,
        Err(_) => return false,
    };
    {
        let mut meta = match write_txn.open_table(META_TABLE) {
            Ok(m) => m,
            Err(_) => return false,
        };
        if meta.insert("schema", SCHEMA_VERSION).is_err() {
            return false;
        }
    }
    {
        if write_txn.open_table(TASKS_TABLE).is_err() {
            return false;
        }
    }
    write_txn.commit().is_ok()
}

fn quarantine_corrupt(path: &Path) {
    if !path.exists() {
        return;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let corrupt_name = format!(
        "{}.corrupt-{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        now
    );
    let corrupt_path = path.with_file_name(&corrupt_name);
    let _ = std::fs::rename(path, &corrupt_path);

    prune_corrupt_files(path);
}

fn prune_corrupt_files(base_path: &Path) {
    let parent = match base_path.parent() {
        Some(p) => p,
        None => return,
    };
    let file_stem = base_path.file_name().unwrap_or_default().to_string_lossy();
    let prefix = format!("{}.corrupt-", file_stem);

    let mut corrupt_entries = Vec::new();
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) {
                if let Ok(meta) = entry.metadata() {
                    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    corrupt_entries.push((entry.path(), mtime));
                }
            }
        }
    }

    // Keep the 3 newest, delete older
    corrupt_entries.sort_by_key(|(_, mtime)| *mtime);
    if corrupt_entries.len() > 3 {
        let to_remove = corrupt_entries.len() - 3;
        for (path, _) in corrupt_entries.into_iter().take(to_remove) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn read_records(db: &Database) -> (Vec<TaskRecord>, Vec<String>) {
    let mut records = Vec::new();
    let mut broken = Vec::new();

    let read_txn = match db.begin_read() {
        Ok(t) => t,
        Err(_) => return (records, broken),
    };

    let table = match read_txn.open_table(TASKS_TABLE) {
        Ok(t) => t,
        Err(_) => return (records, broken),
    };

    let iter = match table.iter() {
        Ok(it) => it,
        Err(_) => return (records, broken),
    };

    for item in iter.flatten() {
        let key = item.0.value().to_string();
        let bytes = item.1.value();
        match serde_json::from_slice::<TaskRecord>(bytes) {
            Ok(rec) => records.push(rec),
            Err(e) => {
                tracing::warn!("Task record decode failed for {}: {:?}", key, e);
                broken.push(key);
            }
        }
    }

    (records, broken)
}

fn run_writer(
    db_path: PathBuf,
    mut db: Database,
    rx: std::sync::mpsc::Receiver<StoreCmd>,
    mode: Arc<Mutex<StoreMode>>,
    status_changed: Arc<AtomicBool>,
) {
    let mut pending_upserts: HashMap<String, TaskRecord> = HashMap::new();
    let mut pending_removes: Vec<String> = Vec::new();
    let mut last_flush = Instant::now();

    loop {
        let timeout = Duration::from_millis(500);
        let cmd = match rx.recv_timeout(timeout) {
            Ok(cmd) => Some(cmd),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };

        let mut urgent = false;
        if let Some(cmd) = cmd {
            match cmd {
                StoreCmd::Upsert { record, urgent: u } => {
                    urgent = u;
                    pending_upserts.insert(record.node.id.clone(), *record);
                }
                StoreCmd::Remove { ids } => {
                    urgent = true;
                    for id in ids {
                        pending_upserts.remove(&id);
                        pending_removes.push(id);
                    }
                }
            }
        }

        // Drain any other immediately available messages
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                StoreCmd::Upsert { record, urgent: u } => {
                    if u {
                        urgent = true;
                    }
                    pending_upserts.insert(record.node.id.clone(), *record);
                }
                StoreCmd::Remove { ids } => {
                    urgent = true;
                    for id in ids {
                        pending_upserts.remove(&id);
                        pending_removes.push(id);
                    }
                }
            }
        }

        let should_flush = urgent || last_flush.elapsed() >= Duration::from_millis(500);
        if should_flush && (!pending_upserts.is_empty() || !pending_removes.is_empty()) {
            let flush_res = flush_batch(&db_path, &mut db, &pending_upserts, &pending_removes);
            if flush_res.is_ok() {
                pending_upserts.clear();
                pending_removes.clear();
                last_flush = Instant::now();
            } else {
                tracing::warn!("Task store write failed permanently; falling back to Memory mode");
                if let Ok(mut m) = mode.lock() {
                    *m = StoreMode::Memory;
                }
                status_changed.store(true, Ordering::Release);
                break;
            }
        }
    }

    // Drain any remaining commands after channel closed
    while let Ok(cmd) = rx.try_recv() {
        match cmd {
            StoreCmd::Upsert { record, .. } => {
                pending_upserts.insert(record.node.id.clone(), *record);
            }
            StoreCmd::Remove { ids } => {
                for id in ids {
                    pending_upserts.remove(&id);
                    pending_removes.push(id);
                }
            }
        }
    }
    if !pending_upserts.is_empty() || !pending_removes.is_empty() {
        let _ = flush_batch(&db_path, &mut db, &pending_upserts, &pending_removes);
    }
}

fn flush_batch(
    db_path: &Path,
    db: &mut Database,
    upserts: &HashMap<String, TaskRecord>,
    removes: &[String],
) -> Result<(), ()> {
    for attempt in 0..3 {
        match write_once(db, upserts, removes) {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!("Task store write attempt {} failed: {:?}", attempt + 1, e);
                // On attempt 1, try reopening the DB handle once
                if attempt == 1 {
                    if let Ok(new_db) = redb::Builder::new().create(db_path) {
                        *db = new_db;
                    }
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Err(())
}

fn write_once(
    db: &Database,
    upserts: &HashMap<String, TaskRecord>,
    removes: &[String],
) -> Result<(), redb::Error> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(TASKS_TABLE)?;
        for (id, record) in upserts {
            if let Ok(bytes) = serde_json::to_vec(record) {
                table.insert(id.as_str(), bytes.as_slice())?;
            }
        }
        for id in removes {
            table.remove(id.as_str())?;
        }
    }
    write_txn.commit()?;
    Ok(())
}
