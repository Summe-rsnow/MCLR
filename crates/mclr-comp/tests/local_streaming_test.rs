//! Streaming-local-mod-path tests: the reader-based MurmurHash2 and the path-based jar parser
//! must reproduce the byte-slice results exactly. Expected values were pinned from the
//! pre-streaming implementation before it was replaced.

use std::fs;
use std::io::{Cursor, Write};

use mclr_comp::local::{
    compute_curseforge_murmur2, compute_curseforge_murmur2_reader, mods_needing_project_lookup,
    parse_mod_metadata, parse_mod_metadata_path, record_project_ids, scan_local_mods,
};
use tempfile::tempdir;
use zip::write::SimpleFileOptions;

const JAR_NAME: &str = "probe-mod-1.2.3.jar";
const JAR_SHA1: &str = "1c61a43751d5beda8418c64475f7b9cf3444fc41";
const JAR_MURMUR2: u32 = 3_519_676_804;
const JAR_SIZE: u64 = 352;
const FABRIC_JSON: &str = r#"{"id":"probe_mod","name":"Probe Mod","version":"1.2.3","description":"Probe description","authors":["Alice",{"name":"Bob"}]}"#;

/// Fixed entry timestamps keep the fixture jar byte-identical between runs.
fn entry_options() -> SimpleFileOptions {
    SimpleFileOptions::default().last_modified_time(zip::DateTime::default())
}

fn write_fixture_jar(dir: &std::path::Path) -> std::path::PathBuf {
    let jar_path = dir.join(JAR_NAME);
    let file = fs::File::create(&jar_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("fabric.mod.json", entry_options()).unwrap();
    zip.write_all(FABRIC_JSON.as_bytes()).unwrap();
    zip.start_file("probe/asset.txt", entry_options()).unwrap();
    zip.write_all(b"line one\r\nline two\t end ").unwrap();
    zip.finish().unwrap();
    jar_path
}

/// Deterministic byte soup that mixes long runs, whitespace and chunk-boundary offsets.
fn fixture_bytes(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut state: u32 = 0x9e37_79b9;
    for i in 0..len {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let byte = (state >> 16) as u8;
        out.push(match i % 7 {
            0 => b' ',
            1 => b'\n',
            2 => b'\t',
            3 => b'\r',
            _ => byte,
        });
    }
    out
}

#[test]
fn test_murmur2_reader_matches_slice_for_fixtures() {
    let cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"abc".to_vec(),
        b"abcd".to_vec(),
        b"\t\n\r ".to_vec(),
        b"\x0a hello\tworld \x0d\x0a foo bar baz  1234567890\r\n".to_vec(),
        // Pinned from the pre-streaming implementation.
        b"aklerfdhvkore;fhjbgoiwrfgbuio34htgb889rguiyufgvueirefvrvu9vhgg9wygf94u8fgw249fyhuwygf293ghf8h"
            .to_vec(),
        fixture_bytes(64 * 1024 - 3),
        fixture_bytes(64 * 1024),
        fixture_bytes(64 * 1024 + 5),
        fixture_bytes(200_000),
    ];

    for bytes in &cases {
        let expected = compute_curseforge_murmur2(bytes);
        let streamed = compute_curseforge_murmur2_reader(&mut Cursor::new(bytes.clone()))
            .expect("streamed murmur2");
        assert_eq!(
            streamed,
            expected,
            "reader mismatch for fixture of {} bytes",
            bytes.len()
        );
    }
}

#[test]
fn test_murmur2_reader_matches_pinned_vectors() {
    let with_ws = b"\x0a hello\tworld \x0d\x0a foo bar baz  1234567890\r\n";
    assert_eq!(
        compute_curseforge_murmur2_reader(&mut Cursor::new(with_ws.to_vec())).unwrap(),
        2_638_458_804
    );
    assert_eq!(
        compute_curseforge_murmur2_reader(&mut Cursor::new(Vec::new())).unwrap(),
        compute_curseforge_murmur2(&[])
    );
}

#[test]
fn test_parse_mod_metadata_path_matches_bytes() {
    let dir = tempdir().unwrap();
    let jar_path = write_fixture_jar(dir.path());
    let bytes = fs::read(&jar_path).unwrap();

    let from_bytes = parse_mod_metadata(&bytes, "probe-mod-1.2.3");
    let from_path = parse_mod_metadata_path(&jar_path, "probe-mod-1.2.3").expect("path parse");
    assert_eq!(from_path.mod_id, from_bytes.mod_id);
    assert_eq!(from_path.name, from_bytes.name);
    assert_eq!(from_path.version, from_bytes.version);
    assert_eq!(from_path.description, from_bytes.description);
    assert_eq!(from_path.authors, from_bytes.authors);
    assert_eq!(from_path.loader, from_bytes.loader);

    // A file that is not a zip falls back exactly like the byte-slice parser.
    let junk = dir.path().join("not-a-jar.jar");
    fs::write(&junk, b"plain text, definitely not a zip").unwrap();
    assert!(parse_mod_metadata_path(&junk, "fallback").is_none());
    let fallback = parse_mod_metadata(b"plain text, definitely not a zip", "fallback");
    assert_eq!(fallback.mod_id, "fallback");
    assert_eq!(fallback.loader, "Unknown");
}

#[test]
fn test_scan_local_mods_streamed_hashes_match_pinned_values() {
    let dir = tempdir().unwrap();
    let mods_dir = dir.path().join("mods");
    fs::create_dir_all(&mods_dir).unwrap();
    let jar_path = write_fixture_jar(&mods_dir);

    let bytes = fs::read(&jar_path).unwrap();
    assert_eq!(bytes.len() as u64, JAR_SIZE);
    // The pre-streaming scan hashed exactly these two ways: digest over the whole file and
    // `compute_curseforge_murmur2(&bytes)`. The streamed scan must agree with both, and with
    // the values pinned above.
    assert_eq!(
        hex::encode(<sha1::Sha1 as sha1::Digest>::digest(&bytes)),
        JAR_SHA1
    );
    assert_eq!(compute_curseforge_murmur2(&bytes), JAR_MURMUR2);

    let scanned = scan_local_mods(&mods_dir).unwrap();
    assert_eq!(scanned.len(), 1);
    let m = &scanned[0];
    assert_eq!(m.file_name, JAR_NAME);
    assert!(m.enabled);
    assert_eq!(m.sha1, JAR_SHA1);
    assert_eq!(m.murmur2, JAR_MURMUR2);
    assert_eq!(m.size, JAR_SIZE);
    assert_eq!(m.mod_id, "probe_mod");
    assert_eq!(m.name, "Probe Mod");
    assert_eq!(m.version, "1.2.3");
    assert_eq!(m.description.as_deref(), Some("Probe description"));
    assert_eq!(m.authors, vec!["Alice", "Bob"]);
    assert_eq!(m.loader, "Fabric");
    assert_eq!(m.path, jar_path.to_string_lossy());

    // Second scan is a cache hit and must return the same metadata.
    let rescanned = scan_local_mods(&mods_dir).unwrap();
    assert_eq!(rescanned, scanned);

    // Project-id backfill lands in the single cache and suppresses further lookups.
    assert_eq!(scanned[0].project_id, None);
    assert!(mods_needing_project_lookup(&scanned).contains(&scanned[0].sha1));

    let mut lookups = std::collections::HashMap::new();
    lookups.insert(
        scanned[0].sha1.clone(),
        Some("modrinth_project".to_string()),
    );
    record_project_ids(&lookups);

    let backfilled = scan_local_mods(&mods_dir).unwrap();
    assert_eq!(
        backfilled[0].project_id.as_deref(),
        Some("modrinth_project")
    );
    assert_eq!(backfilled[0].sha1, JAR_SHA1);
    assert!(mods_needing_project_lookup(&backfilled).is_empty());
}
