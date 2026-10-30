use mclr_core::{NetSettings, SourcePriority};
use mclr_net::downloader::{DownloadSpec, Downloader};
use mclr_net::mirror::{SourceKind, SourceOrigin, racing_ladder, rewrite_url};
use sha1::{Digest as Sha1Digest, Sha1};
use std::time::Duration;
use tempfile::tempdir;
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_rewrite_url() {
    let url = Url::parse("https://resources.download.minecraft.net/ab/abcdef123456").unwrap();
    let variants = rewrite_url(&url, SourceKind::Assets);
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0].0, SourceOrigin::Mirror);
    assert_eq!(
        variants[0].1.as_str(),
        "https://bmclapi2.bangbang93.com/assets/ab/abcdef123456"
    );
    assert_eq!(variants[1].0, SourceOrigin::Official);
    assert_eq!(
        variants[1].1.as_str(),
        "https://resources.download.minecraft.net/ab/abcdef123456"
    );
    // piston-data.mojang.com should NOT be rewritten under SourceKind::Assets
    let url_piston =
        Url::parse("https://piston-data.mojang.com/v1/objects/123/client-1.21.2.xml").unwrap();
    let piston_variants = rewrite_url(&url_piston, SourceKind::Assets);
    assert_eq!(piston_variants.len(), 1);
    assert_eq!(piston_variants[0].0, SourceOrigin::Official);

    // But under SourceKind::VersionMeta, it should be rewritten to bmclapi2
    let meta_variants = rewrite_url(&url_piston, SourceKind::VersionMeta);
    assert_eq!(meta_variants.len(), 2);
    assert_eq!(meta_variants[0].0, SourceOrigin::Mirror);
    assert_eq!(
        meta_variants[0].1.as_str(),
        "https://bmclapi2.bangbang93.com/v1/objects/123/client-1.21.2.xml"
    );

    let url_lib =
        Url::parse("https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar")
            .unwrap();
    let lib_variants = rewrite_url(&url_lib, SourceKind::Libraries);
    assert_eq!(lib_variants.len(), 2);
    assert_eq!(lib_variants[0].0, SourceOrigin::Mirror);
    assert_eq!(
        lib_variants[0].1.as_str(),
        "https://bmclapi2.bangbang93.com/maven/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"
    );

    let url_mod = Url::parse("https://cdn.modrinth.com/data/abc/version/123/mod.jar").unwrap();
    let mod_variants = rewrite_url(&url_mod, SourceKind::ModCdn);
    assert_eq!(mod_variants.len(), 2);
    assert_eq!(mod_variants[0].0, SourceOrigin::Mirror);
    assert_eq!(
        mod_variants[0].1.as_str(),
        "https://mod.mcimirror.top/data/abc/version/123/mod.jar"
    );
}

#[test]
fn test_racing_ladder() {
    let mirror_first = racing_ladder(SourcePriority::MirrorFirst);
    assert_eq!(
        mirror_first[0],
        (SourceOrigin::Mirror, Duration::from_secs(0))
    );
    assert_eq!(
        mirror_first[1],
        (SourceOrigin::Official, Duration::from_secs(30))
    );

    let smart = racing_ladder(SourcePriority::Smart);
    assert_eq!(smart[0], (SourceOrigin::Official, Duration::from_secs(0)));
    assert_eq!(smart[1], (SourceOrigin::Mirror, Duration::from_secs(5)));

    let official_first = racing_ladder(SourcePriority::OfficialFirst);
    assert_eq!(
        official_first[0],
        (SourceOrigin::Official, Duration::from_secs(0))
    );
    assert_eq!(
        official_first[1],
        (SourceOrigin::Mirror, Duration::from_secs(60))
    );
}

#[tokio::test]
async fn test_single_small_file_download() {
    let server = MockServer::start().await;
    let payload = b"Hello, small Minecraft world!";
    let mut hasher = Sha1::new();
    hasher.update(payload);
    let expected_sha1 = hex::encode(hasher.finalize());

    Mock::given(method("GET"))
        .and(path("/small.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.to_vec()))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let dest = dir.path().join("small.txt");
    let url = Url::parse(&format!("{}/small.txt", server.uri())).unwrap();

    let client = reqwest::Client::new();
    let downloader = Downloader::new(client, NetSettings::default());

    let spec = DownloadSpec {
        urls: vec![url],
        dest: dest.clone(),
        size: Some(payload.len() as u64),
        sha1: Some(expected_sha1),
        sha256: None,
        md5: None,
    };

    downloader
        .download(spec, None)
        .await
        .expect("download small file");
    let downloaded_bytes = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(downloaded_bytes, payload);
}

#[tokio::test]
async fn test_range_206_multichunk_download() {
    let server = MockServer::start().await;
    // 9 MiB total file
    let total_size: usize = 9 * 1024 * 1024;
    let mut full_data = vec![0u8; total_size];
    for (i, byte) in full_data.iter_mut().enumerate() {
        *byte = (i % 251) as u8;
    }

    let mut hasher = Sha1::new();
    hasher.update(&full_data);
    let expected_sha1 = hex::encode(hasher.finalize());

    // Probe range responder: Range: bytes=0-0 -> 206 with Content-Range: bytes 0-0/9437184
    Mock::given(method("GET"))
        .and(path("/large.bin"))
        .and(header("range", "bytes=0-0"))
        .respond_with(
            ResponseTemplate::new(206)
                .insert_header("content-range", format!("bytes 0-0/{total_size}"))
                .set_body_bytes(vec![full_data[0]]),
        )
        .mount(&server)
        .await;

    // Range handler for subsequent chunks
    let full_data_clone = full_data.clone();
    Mock::given(method("GET"))
        .and(path("/large.bin"))
        .respond_with(move |req: &wiremock::Request| {
            if let Some(range_header) = req.headers.get("range")
                && let Ok(s) = range_header.to_str()
                && let Some(stripped) = s.strip_prefix("bytes=")
            {
                let parts: Vec<&str> = stripped.split('-').collect();
                if parts.len() == 2 {
                    let start: usize = parts[0].parse().unwrap();
                    let end: usize = parts[1].parse().unwrap();
                    let slice = full_data_clone[start..=end].to_vec();
                    return ResponseTemplate::new(206)
                        .insert_header("content-range", format!("bytes {start}-{end}/{total_size}"))
                        .set_body_bytes(slice);
                }
            }
            ResponseTemplate::new(200).set_body_bytes(full_data_clone.clone())
        })
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let dest = dir.path().join("large.bin");
    let url = Url::parse(&format!("{}/large.bin", server.uri())).unwrap();

    let client = reqwest::Client::new();
    let downloader = Downloader::new(client, NetSettings::default());

    let spec = DownloadSpec {
        urls: vec![url],
        dest: dest.clone(),
        size: Some(total_size as u64),
        sha1: Some(expected_sha1),
        sha256: None,
        md5: None,
    };

    downloader
        .download(spec, None)
        .await
        .expect("download 9 MiB file with chunks");
    let downloaded_bytes = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(downloaded_bytes.len(), total_size);
    assert_eq!(downloaded_bytes, full_data);
}

#[tokio::test]
async fn test_range_fallback_on_200() {
    let server = MockServer::start().await;
    let total_size: usize = 5 * 1024 * 1024;
    let full_data = vec![42u8; total_size];

    // Server always returns 200 OK without partial content support even when range requested
    Mock::given(method("GET"))
        .and(path("/fallback.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(full_data.clone()))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let dest = dir.path().join("fallback.bin");
    let url = Url::parse(&format!("{}/fallback.bin", server.uri())).unwrap();

    let client = reqwest::Client::new();
    let downloader = Downloader::new(client, NetSettings::default());

    let spec = DownloadSpec {
        urls: vec![url],
        dest: dest.clone(),
        size: Some(total_size as u64),
        sha1: None,
        sha256: None,
        md5: None,
    };

    downloader
        .download(spec, None)
        .await
        .expect("download fallback on 200 OK");
    let downloaded_bytes = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(downloaded_bytes.len(), total_size);
    assert_eq!(downloaded_bytes, full_data);
}

#[tokio::test]
async fn test_hash_mismatch_retry() {
    let server = MockServer::start().await;
    let corrupted = b"Corrupted content";
    let valid = b"Valid authentic content";

    let mut hasher = Sha1::new();
    hasher.update(valid);
    let expected_sha1 = hex::encode(hasher.finalize());

    // First URL serves corrupted content
    Mock::given(method("GET"))
        .and(path("/corrupt"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(corrupted.to_vec()))
        .mount(&server)
        .await;

    // Second URL serves valid content
    Mock::given(method("GET"))
        .and(path("/valid"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(valid.to_vec()))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let dest = dir.path().join("retry.txt");
    let url1 = Url::parse(&format!("{}/corrupt", server.uri())).unwrap();
    let url2 = Url::parse(&format!("{}/valid", server.uri())).unwrap();

    let client = reqwest::Client::new();
    let downloader = Downloader::new(client, NetSettings::default());

    let spec = DownloadSpec {
        urls: vec![url1, url2],
        dest: dest.clone(),
        size: Some(valid.len() as u64),
        sha1: Some(expected_sha1),
        sha256: None,
        md5: None,
    };

    downloader
        .download(spec, None)
        .await
        .expect("retry on hash mismatch");
    let downloaded_bytes = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(downloaded_bytes, valid);
}

#[tokio::test]
async fn test_source_racing_order() {
    let server_slow = MockServer::start().await;
    let server_fast = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(1500))
                .set_body_bytes(b"Slow data".to_vec()),
        )
        .mount(&server_slow)
        .await;

    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"Fast data".to_vec()))
        .mount(&server_fast)
        .await;

    let slow_url = Url::parse(&format!("{}/data", server_slow.uri())).unwrap();
    let fast_url = Url::parse(&format!("{}/data", server_fast.uri())).unwrap();

    let client = reqwest::Client::new();
    let candidates = vec![
        (slow_url, Duration::from_millis(0)),
        (fast_url, Duration::from_millis(50)),
    ];

    let winner = mclr_net::mirror::race_candidates(&client, candidates)
        .await
        .expect("race get bytes");
    assert_eq!(winner, b"Fast data");
}

#[tokio::test]
async fn test_slow_chunk_splitting() {
    let server = MockServer::start().await;
    let total_size: usize = 6 * 1024 * 1024; // 6 MiB (> SLOW_SPLIT_THRESHOLD 2 MiB)
    let full_data = vec![7u8; total_size];

    // Range probe
    Mock::given(method("GET"))
        .and(path("/split.bin"))
        .and(header("range", "bytes=0-0"))
        .respond_with(
            ResponseTemplate::new(206)
                .insert_header("content-range", format!("bytes 0-0/{total_size}"))
                .set_body_bytes(vec![full_data[0]]),
        )
        .mount(&server)
        .await;

    let full_data_clone = full_data.clone();
    Mock::given(method("GET"))
        .and(path("/split.bin"))
        .respond_with(move |req: &wiremock::Request| {
            if let Some(range_header) = req.headers.get("range")
                && let Ok(s) = range_header.to_str()
                && let Some(stripped) = s.strip_prefix("bytes=")
            {
                let parts: Vec<&str> = stripped.split('-').collect();
                if parts.len() == 2 {
                    let start: usize = parts[0].parse().unwrap();
                    let end: usize = parts[1].parse().unwrap();
                    let slice = full_data_clone[start..=end].to_vec();
                    return ResponseTemplate::new(206)
                        .insert_header("content-range", format!("bytes {start}-{end}/{total_size}"))
                        .set_body_bytes(slice);
                }
            }
            ResponseTemplate::new(200).set_body_bytes(full_data_clone.clone())
        })
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let dest = dir.path().join("split.bin");
    let url = Url::parse(&format!("{}/split.bin", server.uri())).unwrap();

    let client = reqwest::Client::new();
    // Configure downloader to check slow speed after 1ms and split if speed < 1 GB/s
    let downloader = Downloader::new(client, NetSettings::default())
        .with_slow_check(Duration::from_millis(1), 1_000_000_000);

    let spec = DownloadSpec {
        urls: vec![url],
        dest: dest.clone(),
        size: Some(total_size as u64),
        sha1: None,
        sha256: None,
        md5: None,
    };

    downloader
        .download(spec, None)
        .await
        .expect("download with split");
    let downloaded_bytes = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(downloaded_bytes.len(), total_size);
    assert_eq!(downloaded_bytes, full_data);
}

#[tokio::test]
async fn test_concurrency_bounded() {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    const TOTAL: usize = 200;
    const MAX_FILES: u32 = 4;
    const HOLD: Duration = Duration::from_millis(25);

    let server = MockServer::start().await;

    // Every request is answered after `HOLD`, so a request stays in flight for at
    // least that long. Recording arrival instants lets us derive the peak number of
    // overlapping requests without racing against timer wake-ups.
    let arrivals: Arc<Mutex<Vec<Instant>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&arrivals);
    Mock::given(method("GET"))
        .respond_with(move |req: &wiremock::Request| {
            recorder.lock().unwrap().push(Instant::now());

            let index = req
                .url
                .path()
                .trim_start_matches("/files/")
                .trim_end_matches(".txt");
            ResponseTemplate::new(200)
                .set_delay(HOLD)
                .set_body_string(format!("payload-{index}"))
        })
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let settings = NetSettings {
        max_concurrent_files: MAX_FILES,
        ..NetSettings::default()
    };
    let downloader = Downloader::new(reqwest::Client::new(), settings);

    let mut specs = Vec::new();
    let mut expected = Vec::new();
    for i in 0..TOTAL {
        let payload = format!("payload-{i}");
        let mut hasher = Sha1::new();
        hasher.update(payload.as_bytes());
        let expected_sha1 = hex::encode(hasher.finalize());

        let dest = dir.path().join(format!("file-{i}.txt"));
        let url = Url::parse(&format!("{}/files/{i}.txt", server.uri())).unwrap();
        let size = payload.len() as u64;
        expected.push((dest.clone(), payload));
        specs.push(DownloadSpec {
            urls: vec![url],
            dest,
            size: Some(size),
            sha1: Some(expected_sha1),
            sha256: None,
            md5: None,
        });
    }

    downloader
        .download_many(specs, None)
        .await
        .expect("batch download 200 files");

    let mut starts = arrivals.lock().unwrap().clone();
    starts.sort();
    assert_eq!(starts.len(), TOTAL, "every spec must reach the server");

    // Peak overlap of the in-flight windows [arrival, arrival + HOLD).
    let peak = starts
        .iter()
        .enumerate()
        .map(|(i, start)| {
            starts[i..]
                .iter()
                .filter(|other| **other - *start < HOLD)
                .count()
        })
        .max()
        .unwrap_or(0);

    assert!(
        peak <= MAX_FILES as usize,
        "peak concurrent requests {peak} exceeded limit {MAX_FILES}"
    );
    assert!(
        peak >= 2,
        "requests never overlapped, the concurrency harness is not measuring anything"
    );

    for (dest, payload) in expected {
        let bytes = tokio::fs::read(&dest).await.expect("file landed on disk");
        assert_eq!(String::from_utf8(bytes).unwrap(), payload);
    }
}

/// A paused task must stop the transfer instead of streaming the file to completion.
#[tokio::test]
async fn paused_task_blocks_before_any_request_and_resumes() {
    use mclr_core::{TaskCenter, TaskState};
    use std::sync::Arc;

    let server = MockServer::start().await;
    let payload = vec![7u8; 1024];
    let mut hasher = Sha1::new();
    hasher.update(&payload);
    let expected_sha1 = hex::encode(hasher.finalize());

    Mock::given(method("GET"))
        .and(path("/paused.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.clone()))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let dest = dir.path().join("paused.bin");
    let url = Url::parse(&format!("{}/paused.bin", server.uri())).unwrap();

    let center = Arc::new(TaskCenter::new());
    let (task_id, _control) = center
        .create_task(None, "task.test", Default::default())
        .await;
    center.start_task(&task_id).await.unwrap();
    center.pause_task(&task_id).await.unwrap();

    let downloader = Downloader::new(reqwest::Client::new(), NetSettings::default());
    let spec = DownloadSpec {
        urls: vec![url],
        dest: dest.clone(),
        size: Some(payload.len() as u64),
        sha1: Some(expected_sha1.clone()),
        sha256: None,
        md5: None,
    };

    // Paused: the download future must stay pending (no request, no file).
    let paused = tokio::time::timeout(
        Duration::from_millis(400),
        downloader.download_many(vec![spec], Some((&center, &task_id))),
    )
    .await;
    assert!(paused.is_err(), "paused download must not finish");
    assert!(
        !dest.exists(),
        "paused download must not write the file yet"
    );
    assert_eq!(
        center.list_tasks().await[0].state,
        TaskState::Paused,
        "pause must survive the wait"
    );

    // Resume: the same task must complete and produce the exact bytes.
    center.resume_task(&task_id).await.unwrap();
    let spec = DownloadSpec {
        urls: vec![Url::parse(&format!("{}/paused.bin", server.uri())).unwrap()],
        dest: dest.clone(),
        size: Some(payload.len() as u64),
        sha1: Some(expected_sha1),
        sha256: None,
        md5: None,
    };
    downloader
        .download_many(vec![spec], Some((&center, &task_id)))
        .await
        .expect("resumed download completes");
    assert_eq!(tokio::fs::read(&dest).await.unwrap(), payload);
}
