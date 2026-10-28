//! Log cleaning, token sanitization, and normalization utilities.

use regex::Regex;
use std::sync::LazyLock;

static ANSI_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap());

static ACCESS_TOKEN_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(--accessToken\s+)[^\s]+").unwrap());

// Covers camelCase (`accessToken`) and snake_case (`access_token`,
// `client_token`, `refresh_token`) JSON keys — crash reports are published.
static AUTH_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)("(?:access[_-]?token|client[_-]?token|refresh[_-]?token|id[_-]?token|token)"\s*:\s*")[^"]+(")"#,
    )
    .unwrap()
});

// `access_token=...&refresh_token=...` style form bodies / query strings.
static AUTH_FORM_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)((?:access[_-]?token|client[_-]?token|refresh[_-]?token|id[_-]?token)=)[^&\s]+",
    )
    .unwrap()
});

static BEARER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(Bearer\s+)[A-Za-z0-9\-_=.]+").unwrap());

static USERNAME_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)([A-Z]:\\Users\\)([^\\/:\*\?<>\|\r\n\s]+)").unwrap());

/// Strips ANSI terminal escape codes from log text.
pub fn strip_ansi(text: &str) -> String {
    ANSI_REGEX.replace_all(text, "").to_string()
}

/// Normalizes all line endings to LF (`\n`).
pub fn normalize_crlf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Masks sensitive authentication tokens and personal file paths.
pub fn mask_sensitive(text: &str) -> String {
    let s1 = ACCESS_TOKEN_REGEX.replace_all(text, "${1}***");
    let s2 = AUTH_TOKEN_REGEX.replace_all(&s1, "${1}***${2}");
    let s2b = AUTH_FORM_REGEX.replace_all(&s2, "${1}***");
    let s3 = BEARER_REGEX.replace_all(&s2b, "${1}***");

    // Mask Windows username if not system/public
    USERNAME_PATH_REGEX
        .replace_all(&s3, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let user = &caps[2];
            let user_lower = user.to_ascii_lowercase();
            if user_lower == "public" || user_lower == "default" || user_lower == "all users" {
                format!("{prefix}{user}")
            } else {
                format!("{prefix}***")
            }
        })
        .to_string()
}

/// Fully cleans and prepares raw log text for analysis and export.
pub fn prepare_text(text: &str) -> String {
    let no_ansi = strip_ansi(text);
    let normalized = normalize_crlf(&no_ansi);
    mask_sensitive(&normalized)
}

/// Extracts head and tail lines from a collection of raw lines, keeping up to `head + tail` lines.
pub fn get_head_tail_lines(lines: &[String], head_count: usize, tail_count: usize) -> Vec<String> {
    if lines.len() <= head_count + tail_count {
        return lines.to_vec();
    }
    let mut result = Vec::with_capacity(head_count + tail_count);
    result.extend_from_slice(&lines[..head_count]);
    result.extend_from_slice(&lines[lines.len() - tail_count..]);
    result
}
