//! Network client, mirror rewrite, source racing, and chunked downloader for MCLR.

pub mod client;
pub mod downloader;
pub mod error;
pub mod mirror;

pub use client::{DohResolver, build_http_client};
pub use downloader::{
    DownloadSpec, Downloader, MAX_CHUNK_RETRIES, SLOW_CHECK_DURATION, SLOW_SPEED_BYTES_PER_SEC,
    SLOW_SPLIT_THRESHOLD, SMALL_FILE_THRESHOLD, TARGET_CHUNK_SIZE,
};
pub use mirror::{
    SourceKind, SourceOrigin, race_candidates, race_get_bytes, racing_ladder, rewrite_url,
};
