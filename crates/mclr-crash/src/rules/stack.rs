//! Stack trace extraction and package/keyword analysis (Phase 3).

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::causes::CrashCause;
use crate::rules::modindex::CrashModIndex;
use crate::types::{CrashConfidence, CrashLogSet, InternalCrashFinding};

static IGNORED_PREFIXES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("java");
    s.insert("sun");
    s.insert("javax");
    s.insert("jdk");
    s.insert("com.sun");
    s.insert("com.mojang");
    s.insert("net.minecraft");
    s.insert("MojangTricksIntelDriversForPerformance_javaw");
    s.insert("net.minecraftforge");
    s.insert("cpw.mods");
    s.insert("net.fabricmc");
    s.insert("org.quiltmc");
    s.insert("org.spongepowered");
    s.insert("com.mumfrey");
    s.insert("org.lwjgl");
    s.insert("paulscode.sound");
    s.insert("com.google");
    s.insert("org.apache");
    s.insert("com.electronwill.nightconfig");
    s.insert("it.unimi.dsi");
    s.insert("oolloo");
    s
});

static IGNORED_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let words = [
        "com",
        "org",
        "net",
        "top",
        "dev",
        "gitlab",
        "github",
        "sun",
        "lib",
        "nio",
        "api",
        "asm",
        "reflect",
        "internal",
        "microsoft",
        "mcp",
        "fml",
        "forge",
        "fabricmc",
        "neoforge",
        "neoforged",
        "minecraftforge",
        "minecraft",
        "mojang",
        "mod",
        "mods",
        "modapi",
        "jar",
        "loader",
        "launch",
        "preinit",
        "preload",
        "init",
        "setup",
        "plugin",
        "mixin",
        "mixins",
        "injection",
        "transformer",
        "transformers",
        "spongepowered",
        "game",
        "world",
        "server",
        "client",
        "common",
        "entity",
        "block",
        "item",
        "tile",
        "blockentity",
        "gui",
        "model",
        "render",
        "shader",
        "optifine",
        "config",
        "data",
        "file",
        "read",
        "recipe",
        "content",
        "general",
        "event",
        "events",
        "handler",
        "listeners",
        "assist",
        "override",
        "netty",
        "packet",
        "channel",
        "task",
        "pool",
        "scheduler",
        "systems",
        "system",
        "modules",
        "service",
        "platform",
        "core",
        "main",
        "base",
        "util",
        "impl",
        "done",
        "map",
        "load",
        "machine",
        "dsi",
        "unimi",
        "fastutil",
        "lwjgl",
        "oshi",
        "electronwill",
        "compat",
        "universal",
        "multipart",
    ];
    let mut s = HashSet::new();
    for w in words {
        s.insert(w);
    }
    s
});

static PACKAGE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)(?:at\s+|^\s*)([a-zA-Z_]\w+(?:\.[a-zA-Z_]\w+)+)\.[\w\.$]+\(").unwrap()
});

static MIXIN_STACK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)at\s+[^(]+?\.\w+\$\w+\$([\w\$]+?)\$\w+\(").unwrap());

/// Performs stack analysis on crash report or game output.
pub fn analyze_stack(
    logs: &CrashLogSet,
    mod_index: &CrashModIndex,
) -> Option<InternalCrashFinding> {
    if !should_analyze_stack(&logs.all) {
        return None;
    }

    let stack_text = collect_stack_text(logs);
    let packages = extract_packages(&stack_text);
    let keywords = extract_keywords(&packages);

    if keywords.is_empty() || keywords.len() > 10 {
        return None;
    }

    let matched_mods = mod_index.resolve_many(&keywords);
    if !matched_mods.is_empty() {
        let mod_names: Vec<String> = matched_mods
            .into_iter()
            .map(|m| m.display_name_or_file_name())
            .filter(|s| !s.is_empty())
            .collect();

        let mut finding =
            InternalCrashFinding::new(CrashCause::StackModNameFound, CrashConfidence::Medium)
                .with_stop(true);
        if !mod_names.is_empty() {
            finding = finding.with_param("mod", mod_names.join(", "));
            finding = finding.with_param("mods", mod_names.join("\n - "));
            for name in &mod_names {
                finding = finding.with_evidence(name);
            }
        }
        return Some(finding);
    }

    let mut finding =
        InternalCrashFinding::new(CrashCause::StackKeywordFound, CrashConfidence::Low)
            .with_stop(true);
    finding = finding.with_param("keyword", keywords.join(", "));
    finding = finding.with_param("keywords", keywords.join("\n - "));
    for kw in &keywords {
        finding = finding.with_evidence(kw);
    }
    Some(finding)
}

fn should_analyze_stack(all_logs: &str) -> bool {
    let lower = all_logs.to_ascii_lowercase();
    lower.contains("forge")
        || lower.contains("fabric")
        || lower.contains("quilt")
        || lower.contains("liteloader")
        || lower.contains("neoforge")
}

fn collect_stack_text(logs: &CrashLogSet) -> String {
    let mut parts = Vec::new();

    if let Some(cr) = &logs.crash_report {
        if let Some(pos) = cr.text.find("System Details") {
            parts.push(&cr.text[..pos]);
        } else {
            parts.push(&cr.text);
        }
    }

    if let Some(game) = &logs.game {
        parts.push(&game.text);
    }

    if let Some(hs) = &logs.hs_err
        && let Some(start) = hs.text.find("T H R E A D")
        && let Some(end) = hs.text.find("Registers:")
        && end > start
    {
        parts.push(&hs.text[start..end]);
    }

    parts.join("\n")
}

fn extract_packages(stack_text: &str) -> Vec<String> {
    let mut packages = Vec::new();

    for caps in PACKAGE_REGEX.captures_iter(stack_text) {
        if let Some(m) = caps.get(1) {
            packages.push(m.as_str().trim().to_string());
        }
    }

    for caps in MIXIN_STACK_REGEX.captures_iter(stack_text) {
        if let Some(m) = caps.get(1) {
            packages.push(m.as_str().replace('$', ".").trim().to_string());
        }
    }

    let mut filtered = Vec::new();
    for pkg in packages {
        if pkg.is_empty() {
            continue;
        }
        let ignored = IGNORED_PREFIXES.iter().any(|prefix| {
            pkg.starts_with(prefix)
                && (pkg.len() == prefix.len() || pkg.as_bytes()[prefix.len()] == b'.')
        });
        if !ignored && !filtered.contains(&pkg) {
            filtered.push(pkg);
        }
    }
    filtered
}

fn extract_keywords(packages: &[String]) -> Vec<String> {
    let mut words = Vec::new();

    for pkg in packages {
        let parts: Vec<&str> = pkg.split('.').collect();
        for &part in parts.iter().take(4) {
            let part_trim = part.trim();
            if part_trim.len() <= 2 || part_trim.starts_with("func_") {
                continue;
            }
            let lower = part_trim.to_ascii_lowercase();
            if IGNORED_WORDS.contains(lower.as_str()) {
                continue;
            }
            if !words.contains(&part_trim.to_string()) {
                words.push(part_trim.to_string());
            }
        }
    }

    words
}
