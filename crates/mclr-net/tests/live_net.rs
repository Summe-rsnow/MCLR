use mclr_core::{NetSettings, SourcePriority};
use mclr_net::downloader::{DownloadSpec, Downloader};
use mclr_net::mirror::{SourceKind, race_get_bytes};
use tempfile::tempdir;
use url::Url;

#[tokio::test]
#[ignore]
async fn test_live_version_manifest() {
    let client = reqwest::Client::new();
    let url =
        Url::parse("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json").unwrap();

    let bytes = race_get_bytes(
        &client,
        &url,
        SourceKind::VersionMeta,
        SourcePriority::MirrorFirst,
    )
    .await
    .expect("fetch version manifest");

    let text = String::from_utf8(bytes).expect("valid utf8");
    assert!(text.contains("latest"));
    assert!(text.contains("versions"));
}

#[tokio::test]
#[ignore]
async fn test_live_chunked_download_large_file() {
    // Download lwjgl or client jar >= 8 MiB from BMCLAPI/Mojang
    let client = reqwest::Client::new();
    let downloader = Downloader::new(client, NetSettings::default());

    let dir = tempdir().unwrap();
    let dest = dir.path().join("client.jar");

    // 1.20.1 client jar is 23,028,853 bytes (~22 MiB):
    // https://piston-data.mojang.com/v1/objects/0c3ec587af28e5a785c0b4a7b8a30f9a8f78f838/client.jar
    // BMCLAPI mirror:
    // https://bmclapi2.bangbang93.com/version/1.20.1/client
    let url1 = Url::parse("https://bmclapi2.bangbang93.com/version/1.20.1/client").unwrap();
    let url2 = Url::parse("https://piston-data.mojang.com/v1/objects/0c3ec587af28e5a785c0b4a7b8a30f9a8f78f838/client.jar").unwrap();

    let spec = DownloadSpec {
        urls: vec![url1, url2],
        dest: dest.clone(),
        size: Some(23_028_853),
        sha1: Some("0c3ec587af28e5a785c0b4a7b8a30f9a8f78f838".to_string()),
        sha256: None,
        md5: None,
    };

    downloader
        .download(spec, None)
        .await
        .expect("download large live file");
    assert!(dest.exists());
    let metadata = std::fs::metadata(&dest).unwrap();
    assert_eq!(metadata.len(), 23_028_853);
}
