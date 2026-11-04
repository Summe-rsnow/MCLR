//! Behavioural tests for the content-addressed preset store.

use std::collections::HashSet;
use std::fs;
use std::io::Write;

use mclr_comp::preset::store::{Store, StoredObject};
use mclr_comp::{CompFile, CompFileHashes, CompPlatform, CompReleaseChannel, CompType, LinkMode};
use mclr_core::NetSettings;
use mclr_net::Downloader;
use sha1::{Digest, Sha1};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sha1_of(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn downloader() -> Downloader {
    Downloader::new(reqwest::Client::new(), NetSettings::default())
}

fn comp_file(url: String, file_name: &str, bytes: &[u8], with_sha1: bool) -> CompFile {
    CompFile {
        id: format!("file-{file_name}"),
        project_id: format!("proj-{file_name}"),
        platform: CompPlatform::Modrinth,
        name: "Probe".to_string(),
        file_name: file_name.to_string(),
        mc_versions: vec!["1.20.1".to_string()],
        loaders: vec!["fabric".to_string()],
        channel: CompReleaseChannel::Release,
        release_date: "2025-01-01T00:00:00Z".to_string(),
        size: bytes.len() as u64,
        hashes: CompFileHashes {
            sha1: with_sha1.then(|| sha1_of(bytes)),
            sha512: None,
            murmur2: None,
        },
        download_urls: if url.is_empty() {
            Vec::new()
        } else {
            vec![url]
        },
        distribution_disabled: false,
        dependencies: Vec::new(),
    }
}

async fn serve(server: &MockServer, name: &str, bytes: Vec<u8>) {
    Mock::given(method("GET"))
        .and(path(format!("/files/{name}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
        .mount(server)
        .await;
}

fn url(server: &MockServer, name: &str) -> String {
    format!("{}/files/{name}", server.uri())
}

async fn object_count(root: &std::path::Path) -> usize {
    let mut count = 0;
    for shard in fs::read_dir(root.join("objects")).into_iter().flatten() {
        let Ok(shard) = shard else { continue };
        for entry in fs::read_dir(shard.path()).into_iter().flatten() {
            if entry.map(|e| e.path().is_file()).unwrap_or(false) {
                count += 1;
            }
        }
    }
    count
}

/// Some CI filesystems cannot hard link; those runs still exercise the copy fallback.
fn hardlinks_supported(dir: &std::path::Path) -> bool {
    let a = dir.join(".link-probe-a");
    let b = dir.join(".link-probe-b");
    let supported = fs::write(&a, b"probe").is_ok() && fs::hard_link(&a, &b).is_ok();
    let _ = fs::remove_file(&a);
    let _ = fs::remove_file(&b);
    supported
}

struct Fixture {
    _tmp: TempDir,
    store: Store,
    server: MockServer,
}

async fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::new(tmp.path().join("cache-store"));
    let server = MockServer::start().await;
    Fixture {
        _tmp: tmp,
        store,
        server,
    }
}

impl Fixture {
    fn root(&self) -> std::path::PathBuf {
        self._tmp.path().to_path_buf()
    }

    fn game_dir(&self) -> std::path::PathBuf {
        self.root().join("instance")
    }

    async fn fetch(&self, name: &str, bytes: &[u8]) -> StoredObject {
        serve(&self.server, name, bytes.to_vec()).await;
        self.store
            .ensure(
                &downloader(),
                &comp_file(url(&self.server, name), name, bytes, true),
                CompType::Mod,
            )
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn store_place_hardlink_shares_bytes() {
    let fixture = fixture().await;
    let bytes = b"hardlink-content".repeat(64);
    let obj = fixture.fetch("a.jar", &bytes).await;

    let game = fixture.game_dir();
    let dest = game.join("mods/a.jar");
    let mode = fixture
        .store
        .place(&obj, &dest, true, &game, "mods/a.jar")
        .unwrap();

    if !hardlinks_supported(fixture._tmp.path()) {
        eprintln!("filesystem has no hard links; falling back to the copy assertion");
        assert_eq!(mode, LinkMode::Copy);
        return;
    }

    assert_eq!(mode, LinkMode::HardLink);
    // Writing through the instance path must be visible in the store object.
    let mut file = fs::OpenOptions::new().append(true).open(&dest).unwrap();
    file.write_all(b"-mutated").unwrap();
    drop(file);
    let stored = fs::read(&obj.path).unwrap();
    assert!(stored.ends_with(b"-mutated"));
    assert_eq!(stored.len(), bytes.len() + "-mutated".len());
}

#[tokio::test]
async fn store_place_copy_is_independent() {
    let fixture = fixture().await;
    let bytes = b"copy-content".to_vec();
    let obj = fixture.fetch("b.jar", &bytes).await;

    let game = fixture.game_dir();
    let dest = game.join("mods/b.jar");
    let mode = fixture
        .store
        .place(&obj, &dest, false, &game, "mods/b.jar")
        .unwrap();
    assert_eq!(mode, LinkMode::Copy);

    let mut file = fs::OpenOptions::new().append(true).open(&dest).unwrap();
    file.write_all(b"-mutated").unwrap();
    drop(file);

    assert_eq!(fs::read(&obj.path).unwrap(), bytes);
    assert_ne!(fs::read(&dest).unwrap(), bytes);
}

#[tokio::test]
async fn store_place_refuses_overwrite() {
    let fixture = fixture().await;
    let bytes = b"existing".to_vec();
    let obj = fixture.fetch("c.jar", &bytes).await;

    let game = fixture.game_dir();
    let dest = game.join("mods/c.jar");
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    fs::write(&dest, b"original").unwrap();

    let err = fixture
        .store
        .place(&obj, &dest, true, &game, "mods/c.jar")
        .unwrap_err();
    assert_eq!(err.code(), "preset.destExists");
    assert_eq!(fs::read(&dest).unwrap(), b"original");
}

#[tokio::test]
async fn store_ensure_cache_hit_skips_network() {
    let fixture = fixture().await;
    let bytes = b"cached-content".repeat(32);
    let obj = fixture.fetch("d.jar", &bytes).await;

    let requests_after_first = fixture
        .server
        .received_requests()
        .await
        .expect("request recording enabled")
        .len();
    assert!(requests_after_first > 0);

    serve(&fixture.server, "d.jar", bytes.clone()).await;
    let again = fixture
        .store
        .ensure(
            &downloader(),
            &comp_file(url(&fixture.server, "d.jar"), "d.jar", &bytes, true),
            CompType::Mod,
        )
        .await
        .unwrap();

    let requests_after_second = fixture
        .server
        .received_requests()
        .await
        .expect("request recording enabled")
        .len();
    assert_eq!(
        requests_after_first, requests_after_second,
        "a cache hit must not perform any HTTP request"
    );
    assert_eq!(again, obj);
}

#[tokio::test]
async fn store_ensure_computes_missing_hash() {
    let fixture = fixture().await;
    let bytes = b"no-hash-provided".to_vec();
    serve(&fixture.server, "e.jar", bytes.clone()).await;

    let obj = fixture
        .store
        .ensure(
            &downloader(),
            &comp_file(url(&fixture.server, "e.jar"), "e.jar", &bytes, false),
            CompType::Mod,
        )
        .await
        .unwrap();

    assert_eq!(obj.sha1, sha1_of(&bytes));
    assert!(obj.path.is_file());
    assert_eq!(fs::read(&obj.path).unwrap(), bytes);
}

#[tokio::test]
async fn store_ensure_rejects_hash_mismatch() {
    let fixture = fixture().await;
    let bytes = b"actual-bytes".to_vec();
    serve(&fixture.server, "f.jar", bytes.clone()).await;

    let mut file = comp_file(url(&fixture.server, "f.jar"), "f.jar", &bytes, true);
    file.hashes.sha1 = Some("0".repeat(40));

    let err = fixture
        .store
        .ensure(&downloader(), &file, CompType::Mod)
        .await
        .unwrap_err();
    assert!(!err.code().is_empty());
    assert_eq!(
        object_count(fixture._tmp.path().join("cache-store").as_path()).await,
        0
    );
    let tmp_entries = fs::read_dir(fixture._tmp.path().join("cache-store/tmp"))
        .unwrap()
        .count();
    assert_eq!(tmp_entries, 0);
}

#[tokio::test]
async fn store_prune_keeps_referenced_and_pinned() {
    let fixture = fixture().await;
    let referenced = fixture.fetch("g.jar", b"referenced").await;
    let pinned = fixture.fetch("h.jar", b"pinned").await;
    let orphan = fixture.fetch("i.jar", b"orphan").await;

    let game = fixture.game_dir();
    fixture
        .store
        .place(
            &referenced,
            &game.join("mods/g.jar"),
            false,
            &game,
            "mods/g.jar",
        )
        .unwrap();

    // The pinned object has no live reference: only the pin protects it.
    let mut pinned_ids = HashSet::new();
    pinned_ids.insert("file-h.jar".to_string());

    let info = fixture.store.info(&HashSet::new()).unwrap();
    assert_eq!(info.object_count, 3);
    assert_eq!(
        info.total_bytes,
        referenced.size + pinned.size + orphan.size
    );

    let result = fixture.store.prune(&pinned_ids).unwrap();
    assert_eq!(result.removed_count, 1);
    assert_eq!(result.freed_bytes, orphan.size);
    assert!(!orphan.path.exists());
    assert!(referenced.path.exists());
    assert!(pinned.path.exists());
    assert_eq!(
        object_count(fixture._tmp.path().join("cache-store").as_path()).await,
        2
    );

    let pinned_info = fixture.store.info(&pinned_ids).unwrap();
    assert_eq!(pinned_info.object_count, 2);
    assert!(
        pinned_info
            .entries
            .iter()
            .any(|entry| entry.sha1 == pinned.sha1 && entry.pinned)
    );
    assert!(
        pinned_info
            .entries
            .iter()
            .any(|entry| entry.sha1 == referenced.sha1 && entry.live_refs == 1)
    );
}

#[tokio::test]
async fn store_info_counts_live_refs() {
    let fixture = fixture().await;
    let bytes = b"ref-counted".to_vec();
    let obj = fixture.fetch("j.jar", &bytes).await;
    let game = fixture.game_dir();
    let dest = game.join("mods/j.jar");
    fixture
        .store
        .place(&obj, &dest, true, &game, "mods/j.jar")
        .unwrap();

    let info = fixture.store.info(&HashSet::new()).unwrap();
    assert_eq!(info.object_count, 1);
    assert_eq!(info.entries[0].live_refs, 1);
    assert_eq!(info.entries[0].size, bytes.len() as u64);
    assert_eq!(
        info.default_dir,
        mclr_core::paths::cache_dir()
            .join("store")
            .to_string_lossy()
            .into_owned()
    );

    // Deleting the instance file drops the reference so the object becomes prunable.
    fs::remove_file(&dest).unwrap();
    let result = fixture.store.prune(&HashSet::new()).unwrap();
    assert_eq!(result.removed_count, 1);
    assert!(!obj.path.exists());
}

#[tokio::test]
async fn store_migrate_moves_objects_and_index() {
    let fixture = fixture().await;
    let bytes = b"migrated-content".to_vec();
    let obj = fixture.fetch("k.jar", &bytes).await;
    let game = fixture.game_dir();
    let dest = game.join("mods/k.jar");
    fixture
        .store
        .place(&obj, &dest, true, &game, "mods/k.jar")
        .unwrap();

    let old_root = fixture.store.root().to_path_buf();
    let new_root = fixture.root().join("moved-store");
    fixture.store.migrate_to(&new_root).unwrap();

    assert!(
        new_root
            .join("objects")
            .join(&obj.sha1[..2])
            .join(&obj.sha1)
            .is_file()
    );
    assert!(new_root.join("index.json").is_file());
    assert!(!old_root.join("index.json").exists());
    assert!(!obj.path.exists());
    // The instance link survives (it is an independent directory entry).
    assert_eq!(fs::read(&dest).unwrap(), bytes);

    let moved = Store::new(new_root);
    let info = moved.info(&HashSet::new()).unwrap();
    assert_eq!(info.object_count, 1);
    assert_eq!(info.entries[0].sha1, obj.sha1);
    assert_eq!(info.entries[0].live_refs, 1);
}

#[tokio::test]
async fn store_migrate_refuses_inside_destination() {
    let fixture = fixture().await;
    let inside = fixture.store.root().join("nested");
    let err = fixture.store.migrate_to(&inside).unwrap_err();
    assert_eq!(err.code(), "preset.storeDirInvalid");

    let same = fixture.store.root().to_path_buf();
    let err = fixture.store.migrate_to(&same).unwrap_err();
    assert_eq!(err.code(), "preset.storeDirInvalid");
}

#[tokio::test]
async fn store_remove_deletes_object_and_refs() {
    let fixture = fixture().await;
    let bytes = b"removable".to_vec();
    let obj = fixture.fetch("l.jar", &bytes).await;
    let game = fixture.game_dir();
    fixture
        .store
        .place(&obj, &game.join("mods/l.jar"), false, &game, "mods/l.jar")
        .unwrap();

    fixture.store.remove(&obj.sha1).unwrap();
    assert!(!obj.path.exists());
    let info = fixture.store.info(&HashSet::new()).unwrap();
    assert_eq!(info.object_count, 0);
    assert_eq!(info.total_bytes, 0);
}
