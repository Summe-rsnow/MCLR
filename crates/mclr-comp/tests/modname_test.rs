//! Integration tests for the Chinese-name dictionary and downloaded file naming.

use mclr_comp::model::CompType;
use mclr_comp::modname::{self, lookup_by_slug};
use mclr_comp::naming::{format_download_name, sanitize_file_name};

#[test]
fn dictionary_contains_jei() {
    let entry = lookup_by_slug("jei").expect("jei must be in mcmod.buf");
    assert!(
        entry.zh_name.contains("物品管理器"),
        "unexpected zh name: {}",
        entry.zh_name
    );
    assert_eq!(entry.wiki_id, 459);
    assert_eq!(modname::display_zh(entry), "JEI物品管理器");
    assert_eq!(
        modname::wiki_url(entry.wiki_id),
        "https://www.mcmod.cn/class/459.html"
    );
}

#[test]
fn lookup_is_case_insensitive_and_covers_both_platforms() {
    for slug in ["jei", "JEI", "Jei", " jei "] {
        assert_eq!(
            lookup_by_slug(slug).map(|e| e.wiki_id),
            Some(459),
            "slug {slug}"
        );
    }
    // Entry 20 carries different CurseForge and Modrinth slugs; both must resolve.
    assert_eq!(
        lookup_by_slug("iron-chests-for-fabric").map(|e| e.wiki_id),
        Some(20)
    );
    assert_eq!(
        lookup_by_slug("cyberanner-ironchest").map(|e| e.wiki_id),
        Some(20)
    );
    assert_eq!(
        lookup_by_slug("sodium").map(modname::display_zh),
        Some("钠")
    );
    assert!(lookup_by_slug("").is_none());
    assert!(lookup_by_slug("   ").is_none());
    assert!(lookup_by_slug("this-slug-does-not-exist-xyz").is_none());
}

#[test]
fn display_zh_trims_english_suffixes() {
    let entry = modname::ModNameEntry {
        wiki_id: 7,
        zh_name: "等价交换 (ProjectE)".to_string(),
        cf_slug: String::new(),
        mr_slug: String::new(),
    };
    assert_eq!(modname::display_zh(&entry), "等价交换");

    let entry = modname::ModNameEntry {
        wiki_id: 8,
        zh_name: "工业时代2 - Industrial Craft 2".to_string(),
        cf_slug: String::new(),
        mr_slug: String::new(),
    };
    assert_eq!(modname::display_zh(&entry), "工业时代2");
}

#[test]
fn format_table() {
    let file = "sodium-0.5.jar";
    let zh = Some("钠");
    assert_eq!(
        format_download_name(file, zh, CompType::Mod, 0),
        "【钠】sodium-0.5.jar"
    );
    assert_eq!(
        format_download_name(file, zh, CompType::Mod, 1),
        "[钠] sodium-0.5.jar"
    );
    assert_eq!(
        format_download_name(file, zh, CompType::Mod, 2),
        "钠-sodium-0.5.jar"
    );
    assert_eq!(
        format_download_name(file, zh, CompType::Mod, 3),
        "sodium-0.5-钠.jar"
    );
    // Any other value keeps the raw file name.
    assert_eq!(
        format_download_name(file, zh, CompType::Mod, 4),
        "sodium-0.5.jar"
    );
    assert_eq!(format_download_name(file, zh, CompType::Mod, 255), file);
}

#[test]
fn missing_chinese_name_keeps_file_name() {
    assert_eq!(
        format_download_name("a/b.jar", None, CompType::Resourcepack, 1),
        "a／b.jar"
    );
    assert_eq!(
        format_download_name("a/b.jar", Some("   "), CompType::Resourcepack, 1),
        "a／b.jar"
    );
}

#[test]
fn mixin_tilde_becomes_dash_for_mods() {
    assert_eq!(
        format_download_name("~mixin-patch.jar", None, CompType::Mod, 4),
        "-mixin-patch.jar"
    );
    // Other kinds keep the tilde.
    assert_eq!(
        format_download_name("~pack.zip", None, CompType::Resourcepack, 4),
        "~pack.zip"
    );
}

#[test]
fn illegal_characters_become_fullwidth() {
    assert_eq!(
        sanitize_file_name(r#"a/b:c*d?e"f<g>h|i\j.jar"#),
        "a／b：c＊d？e＂f＜g＞h｜i＼j.jar"
    );
    assert_eq!(sanitize_file_name("line\nbreak.jar"), "line_break.jar");
    assert_eq!(sanitize_file_name("  padded.jar  "), "padded.jar");
}

#[test]
fn empty_names_fall_back_to_download() {
    for raw in ["", "   ", ".", "..", "..."] {
        assert_eq!(sanitize_file_name(raw), "download", "raw {raw:?}");
    }
}

#[test]
fn chinese_part_is_sanitized() {
    assert_eq!(
        format_download_name("pack.zip", Some("a/b:c"), CompType::Resourcepack, 1),
        "[a／b：c] pack.zip"
    );
}

#[test]
fn sanitize_is_idempotent() {
    let samples = [
        "sodium-0.5.jar",
        r#"a/b:c*d?e"f<g>h|i\j.jar"#,
        "",
        "   ",
        "..",
        "tail.",
        "line\nbreak.jar",
        "【钠】sodium-0.5.jar",
    ];
    for sample in samples {
        let once = sanitize_file_name(sample);
        let twice = sanitize_file_name(&once);
        assert_eq!(once, twice, "sample {sample:?}");
    }
}

#[test]
fn formatted_name_is_a_stable_single_sanitize_pass() {
    let formatted = format_download_name("a/b:c.jar", Some("钠/钠"), CompType::Mod, 1);
    assert_eq!(formatted, "[钠／钠] a／b：c.jar");
    assert_eq!(sanitize_file_name(&formatted), formatted);
}
