//! File name formatting for downloads according to user preferences.
//!
//! Mirrors PCL's "saved file name" setting: a downloaded file can be prefixed,
//! suffixed or wrapped with its Chinese name, while keeping the extension and
//! staying a legal Windows file name.

use crate::model::CompType;

/// Replaces every character Windows rejects and blanks control characters.
fn map_illegal_chars(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '\\' => out.push('＼'),
            '/' => out.push('／'),
            '|' => out.push('｜'),
            ':' => out.push('：'),
            '<' => out.push('＜'),
            '>' => out.push('＞'),
            '*' => out.push('＊'),
            '?' => out.push('？'),
            '"' => out.push('＂'),
            c if c.is_control() => out.push('_'),
            c => out.push(c),
        }
    }
    out
}

/// Replaces characters Windows rejects in file names with their fullwidth
/// counterparts, blanks control characters, and trims. The result is always a
/// usable file name and sanitizing it a second time is a no-op.
pub fn sanitize_file_name(name: &str) -> String {
    let mut trimmed = map_illegal_chars(name).trim().to_string();
    // Windows strips trailing dots and spaces; do it ourselves so the result is
    // stable instead of depending on the filesystem.
    while trimmed.ends_with('.') || trimmed.ends_with(' ') {
        trimmed.pop();
    }

    if trimmed.is_empty() {
        return "download".to_string();
    }
    trimmed
}

/// Splits `name` into `(stem, extension)`. `extension` keeps its leading dot.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(idx) if idx > 0 => (&name[..idx], &name[idx..]),
        _ => (name, ""),
    }
}

fn sanitize_zh(zh: &str) -> String {
    map_illegal_chars(zh).trim().to_string()
}

/// Formats the saved file name of a downloaded resource.
///
/// `format`:
/// - `0` `【中文名】file.ext`
/// - `1` `[中文名] file.ext`
/// - `2` `中文名-stem.ext`
/// - `3` `stem-中文名.ext`
/// - any other value keeps the raw (sanitized) file name.
///
/// `file.ext` is the original file name including its extension. When
/// `zh_name` is missing or blank only the sanitized file name is returned.
/// For [`CompType::Mod`] a `~` (Mixin marker) becomes `-`.
pub fn format_download_name(
    file_name: &str,
    zh_name: Option<&str>,
    kind: CompType,
    format: u8,
) -> String {
    let mut file = sanitize_file_name(file_name);
    if kind == CompType::Mod {
        file = file.replace('~', "-");
    }

    let zh = zh_name.map(sanitize_zh).filter(|zh| !zh.is_empty());
    let Some(zh) = zh else {
        return file;
    };

    let formatted = match format {
        0 => format!("【{zh}】{file}"),
        1 => format!("[{zh}] {file}"),
        2 => {
            let (stem, ext) = split_extension(&file);
            format!("{zh}-{stem}{ext}")
        }
        3 => {
            let (stem, ext) = split_extension(&file);
            format!("{stem}-{zh}{ext}")
        }
        _ => return file,
    };

    sanitize_file_name(&formatted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_is_idempotent() {
        let samples = [
            "sodium-0.5.jar",
            "a/b:c*d?e\"f<g>h|i\\j.jar",
            "   ",
            "..",
            "tail.",
            "line\nbreak.jar",
            "【钠】sodium-0.5.jar",
        ];
        for sample in samples {
            let once = sanitize_file_name(sample);
            assert_eq!(once, sanitize_file_name(&once), "sample: {sample:?}");
        }
    }
}
