use crate::error::NetError;
use futures_util::StreamExt;
use mclr_core::{NetSettings, TaskCenter};
use md5::Md5;
use reqwest::header::{CONTENT_RANGE, RANGE};
use reqwest::{Client, StatusCode};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::collections::VecDeque;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::warn;
use url::Url;

pub const SMALL_FILE_THRESHOLD: u64 = 4 * 1024 * 1024; // 4 MiB
pub const TARGET_CHUNK_SIZE: u64 = 8 * 1024 * 1024; // 8 MiB
pub const STREAM_BUFFER_SIZE: usize = 64 * 1024; // 64 KiB
pub const SLOW_CHECK_DURATION: Duration = Duration::from_secs(8);
pub const SLOW_SPEED_BYTES_PER_SEC: u64 = 50 * 1024; // 50 KiB/s
pub const SLOW_SPLIT_THRESHOLD: u64 = 2 * 1024 * 1024; // 2 MiB
pub const MAX_CHUNK_RETRIES: u32 = 2;

#[derive(Debug, Clone)]
pub struct DownloadSpec {
    pub urls: Vec<Url>,
    pub dest: PathBuf,
    pub size: Option<u64>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub md5: Option<String>,
}

/// Digests computed while a single-stream transfer writes the body to disk.
///
/// Only the algorithms required by the [`DownloadSpec`] are populated, so the
/// caller can compare them against the expected values without re-reading the
/// finished file.
#[derive(Debug, Clone, Default)]
pub struct StreamHashes {
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub md5: Option<String>,
}

/// How the body of a single file reached the part file.
enum FileOutcome {
    /// Written by the single-stream path; digests were computed while streaming.
    Streamed(StreamHashes),
    /// Written by the multi-connection path; the caller must verify from disk.
    Chunked,
}

/// Failure of the chunked path — either fatal, or "this server ignores `Range`"
/// which the caller answers by falling back to a single stream.
enum ChunkedError {
    Fatal(NetError),
    RangeUnsupported,
}

/// Expected hash value when the spec actually requires that algorithm.
fn required_hash(expected: &Option<String>) -> Option<&str> {
    expected.as_deref().filter(|value| !value.is_empty())
}

fn ensure_hash(expected: &str, actual: &str) -> Result<(), NetError> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(NetError::HashMismatch {
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
struct Chunk {
    _id: usize,
    start: u64,
    end: u64, // inclusive
    downloaded: u64,
    retries: u32,
}

impl Chunk {
    fn remaining(&self) -> u64 {
        if self.end < self.start + self.downloaded {
            0
        } else {
            self.end - (self.start + self.downloaded) + 1
        }
    }

    fn current_offset(&self) -> u64 {
        self.start + self.downloaded
    }
}

pub struct Downloader {
    client: Client,
    settings: NetSettings,
    /// Configurable slow-check duration (for testing dynamic chunk split without 8s real sleep)
    pub slow_check_duration: Duration,
    /// Configurable slow speed threshold
    pub slow_speed_threshold: u64,
}

impl Downloader {
    pub fn new(client: Client, settings: NetSettings) -> Self {
        Self {
            client,
            settings,
            slow_check_duration: SLOW_CHECK_DURATION,
            slow_speed_threshold: SLOW_SPEED_BYTES_PER_SEC,
        }
    }

    pub fn with_slow_check(mut self, duration: Duration, speed_threshold: u64) -> Self {
        self.slow_check_duration = duration;
        self.slow_speed_threshold = speed_threshold;
        self
    }

    pub async fn download(
        &self,
        spec: DownloadSpec,
        task_center: Option<(&TaskCenter, &str)>,
    ) -> Result<(), NetError> {
        let part_path = PathBuf::from(format!("{}.mclr-part", spec.dest.display()));
        if let Some(parent) = spec.dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let max_whole_retries = 3;
        let mut last_err = NetError::SourcesExhausted("No URLs provided".into());

        if spec.urls.is_empty() {
            return Err(last_err);
        }

        let total_attempts = spec.urls.len().max(max_whole_retries);
        for attempt in 0..total_attempts {
            if attempt > 0 {
                let delay = Duration::from_millis(300 + (attempt as u64) * 300);
                tokio::time::sleep(delay).await;
            }

            // Pick URL across attempts sequentially through all candidate URLs
            let url = &spec.urls[attempt % spec.urls.len()];

            // Clean up any stale part file before attempting
            let _ = tokio::fs::remove_file(&part_path).await;

            let download_result = self
                .download_single_file(url, &spec, &part_path, task_center)
                .await;
            match download_result {
                Ok(outcome) => {
                    // The stream path already hashed the bytes it wrote; only the
                    // chunked path needs the extra full-file read.
                    let verified = match &outcome {
                        FileOutcome::Streamed(hashes) => {
                            self.verify_streamed(&part_path, &spec, hashes).await
                        }
                        FileOutcome::Chunked => self.verify_file(&part_path, &spec).await,
                    };
                    match verified {
                        Ok(()) => {
                            // Atomic rename
                            tokio::fs::rename(&part_path, &spec.dest).await?;
                            return Ok(());
                        }
                        Err(verify_err) => {
                            warn!(
                                "Verification failed for {} (attempt {}): {}",
                                spec.dest.display(),
                                attempt + 1,
                                verify_err
                            );
                            let _ = tokio::fs::remove_file(&part_path).await;
                            last_err = verify_err;
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        "Download attempt {} from {} failed: {}",
                        attempt + 1,
                        url,
                        err
                    );
                    let _ = tokio::fs::remove_file(&part_path).await;
                    last_err = err;
                }
            }
        }

        Err(last_err)
    }

    async fn download_single_file(
        &self,
        url: &Url,
        spec: &DownloadSpec,
        part_path: &Path,
        task_center: Option<(&TaskCenter, &str)>,
    ) -> Result<FileOutcome, NetError> {
        // A known size makes the range probe pointless: pick the path directly.
        if let Some(size) = spec.size {
            if size < SMALL_FILE_THRESHOLD {
                return self
                    .download_stream(url, part_path, Some(size), task_center, spec)
                    .await
                    .map(FileOutcome::Streamed);
            }
            return self
                .download_chunked_or_stream(url, part_path, size, task_center, spec)
                .await;
        }

        // Only an unknown size needs to probe range support
        match self.probe_range(url).await {
            Ok(Some(total_size)) if total_size < SMALL_FILE_THRESHOLD => self
                .download_stream(url, part_path, Some(total_size), task_center, spec)
                .await
                .map(FileOutcome::Streamed),
            Ok(Some(total_size)) => {
                self.download_chunked_or_stream(url, part_path, total_size, task_center, spec)
                    .await
            }
            // Server returned 200 OK without partial content, or the probe failed
            Ok(None) | Err(_) => self
                .download_stream(url, part_path, spec.size, task_center, spec)
                .await
                .map(FileOutcome::Streamed),
        }
    }

    /// Chunked transfer that falls back to a single stream when the server
    /// refuses `Range` requests.
    async fn download_chunked_or_stream(
        &self,
        url: &Url,
        part_path: &Path,
        total_size: u64,
        task_center: Option<(&TaskCenter, &str)>,
        spec: &DownloadSpec,
    ) -> Result<FileOutcome, NetError> {
        match self
            .download_chunked(url, part_path, total_size, task_center)
            .await
        {
            Ok(()) => Ok(FileOutcome::Chunked),
            Err(ChunkedError::Fatal(err)) => Err(err),
            Err(ChunkedError::RangeUnsupported) => {
                warn!(
                    "Range requests unsupported by {}, falling back to a single stream",
                    url
                );
                let _ = tokio::fs::remove_file(part_path).await;
                self.download_stream(url, part_path, Some(total_size), task_center, spec)
                    .await
                    .map(FileOutcome::Streamed)
            }
        }
    }

    /// Range probe with Range: bytes=0-0
    async fn probe_range(&self, url: &Url) -> Result<Option<u64>, NetError> {
        let resp = self
            .client
            .get(url.clone())
            .header(RANGE, "bytes=0-0")
            .send()
            .await?;

        if resp.status() == StatusCode::PARTIAL_CONTENT
            && let Some(range_header) = resp.headers().get(CONTENT_RANGE)
            && let Ok(range_str) = range_header.to_str()
            && let Some(pos) = range_str.rfind('/')
        {
            let total_part = &range_str[pos + 1..];
            if let Ok(total) = total_part.trim().parse::<u64>() {
                return Ok(Some(total));
            }
        }

        Ok(None)
    }

    /// Fallback single-stream download (64 KiB buffer).
    ///
    /// Digests the spec requires are computed while the bytes are written, so the
    /// caller never has to read the finished file back.
    async fn download_stream(
        &self,
        url: &Url,
        part_path: &Path,
        total_size: Option<u64>,
        task_center: Option<(&TaskCenter, &str)>,
        spec: &DownloadSpec,
    ) -> Result<StreamHashes, NetError> {
        let resp = self.client.get(url.clone()).send().await?;
        if !resp.status().is_success() {
            return Err(NetError::Http(resp.error_for_status().unwrap_err()));
        }

        let mut sha1_hasher = required_hash(&spec.sha1).map(|_| Sha1::new());
        let mut sha256_hasher = required_hash(&spec.sha256).map(|_| Sha256::new());
        let mut md5_hasher = required_hash(&spec.md5).map(|_| Md5::new());

        let mut file = File::create(part_path).await?;
        let mut stream = resp.bytes_stream();

        let mut downloaded: u64 = 0;
        let start_time = Instant::now();
        let mut last_report = Instant::now();

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res.map_err(NetError::Http)?;
            file.write_all(&chunk).await?;
            if let Some(hasher) = sha1_hasher.as_mut() {
                hasher.update(&chunk);
            }
            if let Some(hasher) = sha256_hasher.as_mut() {
                hasher.update(&chunk);
            }
            if let Some(hasher) = md5_hasher.as_mut() {
                hasher.update(&chunk);
            }
            downloaded += chunk.len() as u64;

            if let Some((tc, tid)) = task_center
                && last_report.elapsed() >= Duration::from_millis(200)
            {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.001 {
                    (downloaded as f64 / elapsed) as u64
                } else {
                    0
                };
                let prog = if let Some(tot) = total_size {
                    (downloaded as f32 / tot as f32).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                let _ = tc.update_progress(tid, prog, speed).await;
                last_report = Instant::now();
            }
        }

        file.flush().await?;
        Ok(StreamHashes {
            sha1: sha1_hasher.map(|hasher| hex::encode(hasher.finalize())),
            sha256: sha256_hasher.map(|hasher| hex::encode(hasher.finalize())),
            md5: md5_hasher.map(|hasher| hex::encode(hasher.finalize())),
        })
    }

    /// Multi-connection chunked download for files >= 4 MiB
    async fn download_chunked(
        &self,
        url: &Url,
        part_path: &Path,
        total_size: u64,
        task_center: Option<(&TaskCenter, &str)>,
    ) -> Result<(), ChunkedError> {
        // 1. Pre-allocate part file
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(part_path)
            .await
            .map_err(|e| ChunkedError::Fatal(NetError::Io(e)))?;
        file.set_len(total_size)
            .await
            .map_err(|e| ChunkedError::Fatal(NetError::Io(e)))?;
        file.flush()
            .await
            .map_err(|e| ChunkedError::Fatal(NetError::Io(e)))?;
        drop(file);

        // 2. Initial chunks
        let raw_count = total_size.div_ceil(TARGET_CHUNK_SIZE).clamp(1, 1024);
        let chunk_size = total_size.div_ceil(raw_count);

        let mut chunks_queue = VecDeque::new();
        let mut start = 0;
        let mut chunk_id = 0;

        while start < total_size {
            let end = (start + chunk_size - 1).min(total_size - 1);
            chunks_queue.push_back(Chunk {
                _id: chunk_id,
                start,
                end,
                downloaded: 0,
                retries: 0,
            });
            chunk_id += 1;
            start = end + 1;
        }

        let total_downloaded = Arc::new(AtomicU64::new(0));
        let chunks_queue = Arc::new(Mutex::new(chunks_queue));
        let next_id = Arc::new(AtomicU64::new(chunk_id as u64));

        let conn_limit = (self.settings.connections_per_file as usize).clamp(1, 64);
        let worker_count = conn_limit.min(chunks_queue.lock().await.len());

        let mut worker_handles = Vec::new();

        for _ in 0..worker_count {
            let queue_clone = Arc::clone(&chunks_queue);
            let next_id_clone = Arc::clone(&next_id);
            let downloaded_clone = Arc::clone(&total_downloaded);
            let client = self.client.clone();
            let url_clone = url.clone();
            let part_path = part_path.to_path_buf();
            let slow_check_duration = self.slow_check_duration;
            let slow_speed_threshold = self.slow_speed_threshold;

            let handle = tokio::spawn(async move {
                loop {
                    // Pop next chunk
                    let mut chunk = {
                        let mut lock = queue_clone.lock().await;
                        match lock.pop_front() {
                            Some(c) => c,
                            None => break,
                        }
                    };

                    let chunk_res = download_single_chunk(
                        &client,
                        &url_clone,
                        &part_path,
                        &mut chunk,
                        &downloaded_clone,
                        slow_check_duration,
                        slow_speed_threshold,
                    )
                    .await;

                    match chunk_res {
                        Ok(()) => {
                            // Finished this chunk
                        }
                        Err(ChunkError::SlowChunk) => {
                            // Dynamic split if remaining >= SLOW_SPLIT_THRESHOLD
                            let rem = chunk.remaining();
                            let mut lock = queue_clone.lock().await;
                            if rem >= SLOW_SPLIT_THRESHOLD && chunk.retries < MAX_CHUNK_RETRIES {
                                chunk.retries += 1;
                                let half = rem / 2;
                                let split_start = chunk.current_offset() + half;
                                let old_end = chunk.end;
                                chunk.end = split_start - 1;

                                let new_id = next_id_clone.fetch_add(1, Ordering::SeqCst) as usize;
                                let new_chunk = Chunk {
                                    _id: new_id,
                                    start: split_start,
                                    end: old_end,
                                    downloaded: 0,
                                    retries: chunk.retries,
                                };

                                lock.push_back(chunk);
                                lock.push_back(new_chunk);
                            } else if chunk.retries < MAX_CHUNK_RETRIES {
                                chunk.retries += 1;
                                lock.push_back(chunk);
                            } else {
                                return Err(ChunkedError::Fatal(NetError::Timeout(
                                    "Chunk retries exceeded".into(),
                                )));
                            }
                        }
                        Err(ChunkError::Io(e)) => {
                            return Err(ChunkedError::Fatal(NetError::Io(e)));
                        }
                        Err(ChunkError::Http(e)) => {
                            let mut lock = queue_clone.lock().await;
                            if chunk.retries < MAX_CHUNK_RETRIES {
                                chunk.retries += 1;
                                lock.push_back(chunk);
                            } else {
                                return Err(ChunkedError::Fatal(NetError::Http(e)));
                            }
                        }
                        Err(ChunkError::RangeUnsupported) => {
                            return Err(ChunkedError::RangeUnsupported);
                        }
                        Err(ChunkError::PrematureEof) => {
                            let mut lock = queue_clone.lock().await;
                            if chunk.retries < MAX_CHUNK_RETRIES {
                                chunk.retries += 1;
                                lock.push_back(chunk);
                            } else {
                                return Err(ChunkedError::Fatal(NetError::Timeout(
                                    "Premature EOF in chunk".into(),
                                )));
                            }
                        }
                    }
                }
                Ok::<(), ChunkedError>(())
            });
            worker_handles.push(handle);
        }
        // Progress reporter loop
        let start_time = Instant::now();
        let mut last_report = Instant::now();

        // Wait for workers while periodically reporting progress
        let mut all_workers_done = false;
        while !all_workers_done {
            all_workers_done = true;
            for h in &mut worker_handles {
                if !h.is_finished() {
                    all_workers_done = false;
                    break;
                }
            }

            if let Some((tc, tid)) = task_center
                && last_report.elapsed() >= Duration::from_millis(250)
            {
                let cur = total_downloaded.load(Ordering::Relaxed);
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.001 {
                    (cur as f64 / elapsed) as u64
                } else {
                    0
                };
                let prog = (cur as f32 / total_size as f32).clamp(0.0, 1.0);
                let _ = tc.update_progress(tid, prog, speed).await;
                last_report = Instant::now();
            }

            if !all_workers_done {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        let mut first_error: Option<NetError> = None;
        let mut range_unsupported = false;
        for h in worker_handles {
            match h.await {
                Ok(Ok(())) => {}
                Ok(Err(ChunkedError::RangeUnsupported)) => range_unsupported = true,
                Ok(Err(ChunkedError::Fatal(err))) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
                Err(join_err) => {
                    if first_error.is_none() {
                        first_error =
                            Some(NetError::Io(std::io::Error::other(join_err.to_string())));
                    }
                }
            }
        }

        if range_unsupported {
            return Err(ChunkedError::RangeUnsupported);
        }

        if let Some(err) = first_error {
            return Err(ChunkedError::Fatal(err));
        }

        let downloaded_final = total_downloaded.load(Ordering::SeqCst);
        if downloaded_final != total_size {
            return Err(ChunkedError::Fatal(NetError::SizeMismatch {
                expected: total_size,
                actual: downloaded_final,
            }));
        }

        if let Some((tc, tid)) = task_center {
            let _ = tc.update_progress(tid, 1.0, 0).await;
        }

        Ok(())
    }

    async fn verify_file(&self, path: &Path, spec: &DownloadSpec) -> Result<(), NetError> {
        let mut file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let file_len = metadata.len();

        if let Some(expected_size) = spec.size
            && file_len != expected_size
        {
            return Err(NetError::SizeMismatch {
                expected: expected_size,
                actual: file_len,
            });
        }

        if spec.sha1.is_none() && spec.sha256.is_none() && spec.md5.is_none() {
            return Ok(());
        }

        let mut sha1_hasher = Sha1::new();
        let mut sha256_hasher = Sha256::new();
        let mut md5_hasher = Md5::new();

        let mut buf = vec![0u8; STREAM_BUFFER_SIZE];
        loop {
            let n = file.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            if spec.sha1.is_some() {
                sha1_hasher.update(&buf[..n]);
            }
            if spec.sha256.is_some() {
                sha256_hasher.update(&buf[..n]);
            }
            if spec.md5.is_some() {
                md5_hasher.update(&buf[..n]);
            }
        }

        if let Some(expected) = &spec.sha1 {
            let actual = hex::encode(sha1_hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(NetError::HashMismatch {
                    expected: expected.clone(),
                    actual,
                });
            }
        }

        if let Some(expected) = &spec.sha256 {
            let actual = hex::encode(sha256_hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(NetError::HashMismatch {
                    expected: expected.clone(),
                    actual,
                });
            }
        }

        if let Some(expected) = &spec.md5 {
            let actual = hex::encode(md5_hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(NetError::HashMismatch {
                    expected: expected.clone(),
                    actual,
                });
            }
        }

        Ok(())
    }

    /// Verify a part file that was written by the single-stream path, reusing the
    /// digests computed while streaming. No second full-file read happens here.
    async fn verify_streamed(
        &self,
        path: &Path,
        spec: &DownloadSpec,
        hashes: &StreamHashes,
    ) -> Result<(), NetError> {
        let file_len = tokio::fs::metadata(path).await?.len();
        if let Some(expected_size) = spec.size
            && file_len != expected_size
        {
            return Err(NetError::SizeMismatch {
                expected: expected_size,
                actual: file_len,
            });
        }

        if let Some(expected) = required_hash(&spec.sha1) {
            match hashes.sha1.as_deref() {
                Some(actual) => ensure_hash(expected, actual)?,
                // Defensive: the stream skipped this algorithm, so read the file.
                None => return self.verify_file(path, spec).await,
            }
        }

        if let Some(expected) = required_hash(&spec.sha256) {
            match hashes.sha256.as_deref() {
                Some(actual) => ensure_hash(expected, actual)?,
                None => return self.verify_file(path, spec).await,
            }
        }

        if let Some(expected) = required_hash(&spec.md5) {
            match hashes.md5.as_deref() {
                Some(actual) => ensure_hash(expected, actual)?,
                None => return self.verify_file(path, spec).await,
            }
        }

        Ok(())
    }

    /// Batch download multiple files concurrently with global limit `max_concurrent_files`.
    pub async fn download_many(
        &self,
        specs: Vec<DownloadSpec>,
        task_center: Option<(&TaskCenter, &str)>,
    ) -> Result<(), NetError> {
        if specs.is_empty() {
            return Ok(());
        }

        if let Some((tc, tid)) = task_center {
            let mut count_map = std::collections::HashMap::new();
            count_map.insert("count".to_string(), specs.len().to_string());
            let _ = tc.update_title_params(tid, count_map).await;
        }
        let max_files = (self.settings.max_concurrent_files as usize).clamp(1, 128);

        let total_bytes: u64 = specs.iter().map(|s| s.size.unwrap_or(0)).sum();
        let completed_bytes = Arc::new(AtomicU64::new(0));

        // Fixed-size worker pool over a shared queue: the number of live tasks never
        // exceeds `max_files`, no per-file `tokio::spawn` and no `is_finished` sweep.
        let worker_count = max_files.min(specs.len());
        let queue = Arc::new(Mutex::new(VecDeque::from_iter(
            specs.into_iter().enumerate(),
        )));

        let mut workers = JoinSet::new();
        for _ in 0..worker_count {
            let queue = Arc::clone(&queue);
            let completed_bytes = Arc::clone(&completed_bytes);
            let client = self.client.clone();
            let settings = self.settings.clone();
            let slow_check_duration = self.slow_check_duration;
            let slow_speed_threshold = self.slow_speed_threshold;

            workers.spawn(async move {
                let dl = Downloader::new(client, settings)
                    .with_slow_check(slow_check_duration, slow_speed_threshold);
                let mut results: Vec<(usize, Result<(), NetError>)> = Vec::new();

                loop {
                    let next = {
                        let mut queue = queue.lock().await;
                        queue.pop_front()
                    };
                    let Some((index, spec)) = next else {
                        break;
                    };

                    let spec_size = spec.size.unwrap_or(0);
                    let result = dl.download(spec, None).await;
                    if result.is_ok() {
                        completed_bytes.fetch_add(spec_size, Ordering::Relaxed);
                    }
                    results.push((index, result));
                }

                results
            });
        }

        // Aggregate progress monitor: report on the same 250ms cadence, driven by
        // worker completions and a 50ms ticker instead of an O(n) sweep.
        let start_time = Instant::now();
        let mut last_report = Instant::now();
        let mut ticker = tokio::time::interval(Duration::from_millis(50));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let mut results: Vec<(usize, Result<(), NetError>)> = Vec::new();
        let mut join_error: Option<NetError> = None;

        loop {
            tokio::select! {
                joined = workers.join_next() => {
                    match joined {
                        Some(Ok(mut batch)) => results.append(&mut batch),
                        Some(Err(err)) => {
                            join_error = Some(NetError::Io(std::io::Error::other(err.to_string())));
                            break;
                        }
                        None => break,
                    }
                }
                _ = ticker.tick() => {}
            }

            if let Some((tc, tid)) = task_center
                && last_report.elapsed() >= Duration::from_millis(250)
            {
                let done = completed_bytes.load(Ordering::Relaxed);
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.001 {
                    (done as f64 / elapsed) as u64
                } else {
                    0
                };
                let prog = if total_bytes > 0 {
                    (done as f32 / total_bytes as f32).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                let _ = tc.update_progress(tid, prog, speed).await;
                last_report = Instant::now();
            }
        }

        if let Some(err) = join_error {
            return Err(err);
        }

        // Report the first failure in spec order, matching the old join order.
        results.sort_by_key(|(index, _)| *index);
        for (_, result) in results {
            result?;
        }

        if let Some((tc, tid)) = task_center {
            let _ = tc.update_progress(tid, 1.0, 0).await;
        }

        Ok(())
    }
}

enum ChunkError {
    SlowChunk,
    Io(std::io::Error),
    Http(reqwest::Error),
    PrematureEof,
    /// Server answered with a full-body success instead of `206 Partial Content`.
    RangeUnsupported,
}

async fn download_single_chunk(
    client: &Client,
    url: &Url,
    part_path: &Path,
    chunk: &mut Chunk,
    total_downloaded: &AtomicU64,
    slow_check_duration: Duration,
    slow_speed_threshold: u64,
) -> Result<(), ChunkError> {
    let mut file = OpenOptions::new()
        .write(true)
        .open(part_path)
        .await
        .map_err(ChunkError::Io)?;

    let range_val = format!("bytes={}-{}", chunk.current_offset(), chunk.end);
    let resp = client
        .get(url.clone())
        .header(RANGE, range_val)
        .send()
        .await
        .map_err(ChunkError::Http)?;

    if resp.status() != StatusCode::PARTIAL_CONTENT {
        // A 2xx here means the server ignored `Range` and started the whole body:
        // report it so the caller can fall back to a single stream.
        if resp.status().is_success() {
            return Err(ChunkError::RangeUnsupported);
        }
        return Err(ChunkError::Http(resp.error_for_status().unwrap_err()));
    }

    let mut stream = resp.bytes_stream();
    let started_at = Instant::now();
    let mut attempt_bytes = 0u64;

    while chunk.remaining() > 0 {
        match stream.next().await {
            Some(Ok(bytes)) => {
                let to_write = (bytes.len() as u64).min(chunk.remaining()) as usize;
                let offset = chunk.current_offset();

                file.seek(SeekFrom::Start(offset))
                    .await
                    .map_err(ChunkError::Io)?;
                file.write_all(&bytes[..to_write])
                    .await
                    .map_err(ChunkError::Io)?;

                chunk.downloaded += to_write as u64;
                attempt_bytes += to_write as u64;
                total_downloaded.fetch_add(to_write as u64, Ordering::Relaxed);

                // Speed check after elapsed check duration
                let elapsed = started_at.elapsed();
                if elapsed >= slow_check_duration {
                    let speed = (attempt_bytes as f64 / elapsed.as_secs_f64()) as u64;
                    if speed < slow_speed_threshold && chunk.remaining() >= SLOW_SPLIT_THRESHOLD {
                        return Err(ChunkError::SlowChunk);
                    }
                }
            }
            Some(Err(e)) => return Err(ChunkError::Http(e)),
            None => {
                if chunk.remaining() > 0 {
                    return Err(ChunkError::PrematureEof);
                }
                break;
            }
        }
    }

    file.flush().await.map_err(ChunkError::Io)?;
    Ok(())
}
