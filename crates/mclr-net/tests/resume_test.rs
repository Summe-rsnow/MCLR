use mclr_core::{NetSettings, TaskCenter};
use mclr_net::downloader::{DownloadSpec, Downloader};
use sha1::{Digest, Sha1};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

fn compute_sha1(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

struct TestServer {
    url_base: String,
    range_headers: Arc<Mutex<Vec<String>>>,
    request_count: Arc<AtomicUsize>,
}

async fn spawn_test_server(payload: Vec<u8>, is_slow: bool) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let range_headers = Arc::new(Mutex::new(Vec::new()));
    let request_count = Arc::new(AtomicUsize::new(0));

    let range_headers_clone = Arc::clone(&range_headers);
    let request_count_clone = Arc::clone(&request_count);

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let body = payload.clone();
            let ranges = Arc::clone(&range_headers_clone);
            let req_count = Arc::clone(&request_count_clone);

            tokio::spawn(async move {
                req_count.fetch_add(1, Ordering::SeqCst);
                let mut buf = Vec::new();
                let mut byte = [0u8; 1];
                while socket.read_exact(&mut byte).await.is_ok() {
                    buf.push(byte[0]);
                    if buf.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }

                let request = String::from_utf8_lossy(&buf).to_string();
                let range_line = request
                    .lines()
                    .find(|line| line.to_lowercase().starts_with("range: bytes="))
                    .map(|line| line.to_string());

                let total = body.len();

                if let Some(ref r_line) = range_line {
                    ranges.lock().unwrap().push(r_line.clone());
                    let val = r_line.split('=').nth(1).unwrap_or("").trim();
                    let (start_str, end_str) = val.split_once('-').unwrap_or((val, ""));
                    let start: usize = start_str.trim().parse().unwrap_or(0);
                    let end: usize = if end_str.trim().is_empty() {
                        total - 1
                    } else {
                        end_str.trim().parse().unwrap_or(total - 1).min(total - 1)
                    };

                    let slice = if start < total && start <= end {
                        body[start..=end].to_vec()
                    } else {
                        Vec::new()
                    };

                    let head = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{total}\r\nETag: \"etag-test\"\r\nConnection: close\r\n\r\n",
                        slice.len()
                    );
                    if socket.write_all(head.as_bytes()).await.is_err() {
                        return;
                    }

                    if is_slow {
                        for piece in slice.chunks(64 * 1024) {
                            if socket.write_all(piece).await.is_err() {
                                return;
                            }
                            tokio::time::sleep(Duration::from_millis(15)).await;
                        }
                    } else {
                        let _ = socket.write_all(&slice).await;
                    }
                } else {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nETag: \"etag-test\"\r\nConnection: close\r\n\r\n"
                    );
                    if socket.write_all(head.as_bytes()).await.is_err() {
                        return;
                    }

                    if is_slow {
                        for piece in body.chunks(64 * 1024) {
                            if socket.write_all(piece).await.is_err() {
                                return;
                            }
                            tokio::time::sleep(Duration::from_millis(15)).await;
                        }
                    } else {
                        let _ = socket.write_all(&body).await;
                    }
                }
                let _ = socket.shutdown().await;
            });
        }
    });

    TestServer {
        url_base: format!("http://{addr}"),
        range_headers,
        request_count,
    }
}

#[tokio::test]
async fn test_stream_resume_and_tamper_and_skip() {
    // 2 MiB payload (< 4 MiB -> stream path)
    let total_size = 2 * 1024 * 1024;
    let mut payload = vec![0u8; total_size];
    for (i, byte) in payload.iter_mut().enumerate() {
        *byte = ((i * 17) % 256) as u8;
    }
    let expected_sha1 = compute_sha1(&payload);

    let server = spawn_test_server(payload, true).await;
    let url = Url::parse(&format!("{}/stream.bin", server.url_base)).unwrap();

    let dir = tempdir().unwrap();
    let dest = dir.path().join("stream.bin");
    let part_path = dir.path().join("stream.bin.mclr-part");
    let meta_path = dir.path().join("stream.bin.mclr-part.meta");

    let downloader = Downloader::new(reqwest::Client::new(), NetSettings::default());
    let center = Arc::new(TaskCenter::new());
    let (tid, control) = center
        .create_task(
            "task.stream",
            mclr_core::tasks::TaskPlan {
                title_key: "task.stream".into(),
                title_params: Default::default(),
                steps: vec![mclr_core::tasks::StepPlan {
                    key: "run",
                    weight: 1.0,
                }],
            },
        )
        .await;
    center.start_task(&tid).await.unwrap();

    // 1. First run: cancel task as soon as >= 512 KiB is written to part
    let cancel_watcher = {
        let control_clone = control.clone();
        let part_clone = part_path.clone();
        tokio::spawn(async move {
            for _ in 0..200 {
                tokio::time::sleep(Duration::from_millis(10)).await;
                if let Ok(meta) = tokio::fs::metadata(&part_clone).await
                    && meta.len() >= 512 * 1024
                {
                    control_clone.cancel();
                    break;
                }
            }
        })
    };

    let spec = DownloadSpec {
        urls: vec![url.clone()],
        dest: dest.clone(),
        size: Some(total_size as u64),
        sha1: Some(expected_sha1.clone()),
        sha256: None,
        md5: None,
    };

    let res1 = downloader
        .download(spec.clone(), Some((&center, &tid)))
        .await;
    let _ = cancel_watcher.await;
    assert!(
        res1.is_err(),
        "First download should be canceled: {:?}",
        res1
    );

    // Verify part and meta files exist after cancellation
    assert!(part_path.exists(), "part file must survive cancellation");
    assert!(meta_path.exists(), "meta file must survive cancellation");

    let part_len_before_resume = tokio::fs::metadata(&part_path).await.unwrap().len();
    assert!(part_len_before_resume >= 512 * 1024);

    // 2. Second run: resume the stream download
    let (tid2, _) = center
        .create_task(
            "task.stream.resume",
            mclr_core::tasks::TaskPlan {
                title_key: "task.stream.resume".into(),
                title_params: Default::default(),
                steps: vec![mclr_core::tasks::StepPlan {
                    key: "run",
                    weight: 1.0,
                }],
            },
        )
        .await;
    center.start_task(&tid2).await.unwrap();

    let res2 = downloader
        .download(spec.clone(), Some((&center, &tid2)))
        .await;
    assert!(
        res2.is_ok(),
        "Second download must resume and succeed: {:?}",
        res2
    );

    // Check Range header was sent with bytes=L-
    let recorded = server.range_headers.lock().unwrap().clone();
    let expected_range_prefix = format!("bytes={part_len_before_resume}-");
    assert!(
        recorded.iter().any(|h| h
            .to_lowercase()
            .contains(&expected_range_prefix.to_lowercase())),
        "Expected range header containing {expected_range_prefix}, got: {:?}",
        recorded
    );

    // Verify final destination file matches
    let written = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(compute_sha1(&written), expected_sha1);
    assert!(!part_path.exists(), "part file should be renamed");
    assert!(
        !meta_path.exists(),
        "meta file should be removed on success"
    );

    // 3. Skip-if-verified: calling download when dest already matches sha1 makes zero network requests
    let req_count_before = server.request_count.load(Ordering::SeqCst);
    let res_skip = downloader.download(spec.clone(), None).await;
    assert!(res_skip.is_ok());
    let req_count_after = server.request_count.load(Ordering::SeqCst);
    assert_eq!(
        req_count_before, req_count_after,
        "Skip-if-verified should make zero requests"
    );
}

#[tokio::test]
async fn test_tampering_forces_clean_restart() {
    let total_size = 1024 * 1024;
    let payload = vec![42u8; total_size];
    let expected_sha1 = compute_sha1(&payload);

    let server = spawn_test_server(payload, false).await;
    let url = Url::parse(&format!("{}/tamper.bin", server.url_base)).unwrap();

    let dir = tempdir().unwrap();
    let dest = dir.path().join("tamper.bin");
    let part_path = dir.path().join("tamper.bin.mclr-part");
    let meta_path = dir.path().join("tamper.bin.mclr-part.meta");

    let downloader = Downloader::new(reqwest::Client::new(), NetSettings::default());

    // Create a tampered meta file with wrong URL
    std::fs::write(
        &meta_path,
        br#"{"v":1,"url":"https://fake-url.com/wrong","total":1048576,"etag":null,"lastModified":null,"chunks":[]}"#,
    )
    .unwrap();
    std::fs::write(&part_path, vec![0u8; 100]).unwrap();

    let spec = DownloadSpec {
        urls: vec![url],
        dest: dest.clone(),
        size: Some(total_size as u64),
        sha1: Some(expected_sha1.clone()),
        sha256: None,
        md5: None,
    };

    // Download should detect tampered meta, delete it, and download cleanly from scratch
    let res = downloader.download(spec, None).await;
    assert!(
        res.is_ok(),
        "Tampered meta should cleanly restart and succeed: {:?}",
        res
    );

    let written = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(compute_sha1(&written), expected_sha1);
}

#[tokio::test]
async fn test_chunked_resume_12mib() {
    // 12 MiB payload (>= 4 MiB -> chunked path)
    let total_size = 12 * 1024 * 1024;
    let mut payload = vec![0u8; total_size];
    for (i, byte) in payload.iter_mut().enumerate() {
        *byte = ((i * 31) % 256) as u8;
    }
    let expected_sha1 = compute_sha1(&payload);

    let server = spawn_test_server(payload, true).await;
    let url = Url::parse(&format!("{}/chunked.bin", server.url_base)).unwrap();

    let dir = tempdir().unwrap();
    let dest = dir.path().join("chunked.bin");
    let part_path = dir.path().join("chunked.bin.mclr-part");
    let meta_path = dir.path().join("chunked.bin.mclr-part.meta");

    let downloader = Downloader::new(reqwest::Client::new(), NetSettings::default());
    let center = Arc::new(TaskCenter::new());
    let (tid, control) = center
        .create_task(
            "task.chunked",
            mclr_core::tasks::TaskPlan {
                title_key: "task.chunked".into(),
                title_params: Default::default(),
                steps: vec![mclr_core::tasks::StepPlan {
                    key: "run",
                    weight: 1.0,
                }],
            },
        )
        .await;
    center.start_task(&tid).await.unwrap();

    // Abort after >= 3 MiB written
    let cancel_watcher = {
        let control_clone = control.clone();
        let meta_clone = meta_path.clone();
        tokio::spawn(async move {
            for _ in 0..200 {
                tokio::time::sleep(Duration::from_millis(10)).await;
                if let Ok(content) = tokio::fs::read(&meta_clone).await
                    && let Ok(meta) =
                        serde_json::from_slice::<mclr_net::downloader::PartMeta>(&content)
                {
                    let downloaded: u64 = meta.chunks.iter().map(|c| c.downloaded).sum();
                    if downloaded >= 3 * 1024 * 1024 {
                        control_clone.cancel();
                        break;
                    }
                }
            }
        })
    };

    let spec = DownloadSpec {
        urls: vec![url.clone()],
        dest: dest.clone(),
        size: Some(total_size as u64),
        sha1: Some(expected_sha1.clone()),
        sha256: None,
        md5: None,
    };

    let res1 = downloader
        .download(spec.clone(), Some((&center, &tid)))
        .await;
    let _ = cancel_watcher.await;
    assert!(
        res1.is_err(),
        "First chunked download should be canceled: {:?}",
        res1
    );

    assert!(part_path.exists(), "part file must exist");
    assert!(meta_path.exists(), "meta file must exist");

    // Second run: resume chunked download
    let (tid2, _) = center
        .create_task(
            "task.chunked.resume",
            mclr_core::tasks::TaskPlan {
                title_key: "task.chunked.resume".into(),
                title_params: Default::default(),
                steps: vec![mclr_core::tasks::StepPlan {
                    key: "run",
                    weight: 1.0,
                }],
            },
        )
        .await;
    center.start_task(&tid2).await.unwrap();

    let res2 = downloader
        .download(spec.clone(), Some((&center, &tid2)))
        .await;
    assert!(
        res2.is_ok(),
        "Second chunked download must resume and succeed: {:?}",
        res2
    );

    let written = tokio::fs::read(&dest).await.unwrap();
    assert_eq!(compute_sha1(&written), expected_sha1);
    assert!(!part_path.exists());
    assert!(!meta_path.exists());
}
