//! Version range satisfaction checks for Maven and Fabric version specs.
//!
//! Mod metadata expresses dependency constraints in three dialects: Maven intervals
//! (`[1.0,2.0)`, `(,1.5]`), Fabric/npm predicates (`>=1.2.3`, `~1.2`, `^1.2`, `1.2.x`,
//! space-separated conjunctions, `||` alternatives) and plain versions. Only one thing matters
//! to the launcher here: never claim a mismatch it cannot prove. Anything that does not parse —
//! placeholders such as `${file.jarVersion}`, Forge's `$libraries` strings, garbage from a
//! hand-edited jar — satisfies the range, so the worst case is a warning that could have been
//! shown rather than a mod reported broken when it is not.

use mclr_core::mcver::{SEPARATORS, compare_strict};
use std::cmp::Ordering;

/// True when `version` is allowed by `range`.
///
/// Empty input, an empty range, `*` or any unparsable expression returns `true`.
pub fn satisfies(version: &str, range: &str) -> bool {
    let version = version.trim();
    let range = range.trim();
    if version.is_empty() || range.is_empty() {
        return true;
    }
    // Unresolved build placeholders (`${file.jarVersion}`) and Forge library specs are not
    // versions at all; guessing would only produce false VersionMismatch rows.
    if version.contains("${") || range.contains("${") {
        return true;
    }
    satisfies_parsed(version, range).unwrap_or(true)
}

/// `None` means "could not parse", which callers turn into `true`.
fn satisfies_parsed(version: &str, range: &str) -> Option<bool> {
    let mut saw_unparsable = false;
    let mut any_alternative = false;

    for alternative in range.split("||") {
        let alternative = alternative.trim();
        if alternative.is_empty() {
            continue;
        }
        any_alternative = true;
        match alternative_satisfied(version, alternative) {
            Some(true) => return Some(true),
            Some(false) => {}
            None => saw_unparsable = true,
        }
    }

    if !any_alternative || saw_unparsable {
        None
    } else {
        Some(false)
    }
}

fn alternative_satisfied(version: &str, alternative: &str) -> Option<bool> {
    if alternative.starts_with('[') || alternative.starts_with('(') {
        return maven_satisfied(version, alternative);
    }

    let mut saw_token = false;
    for token in alternative.split_whitespace() {
        if token.is_empty() {
            continue;
        }
        saw_token = true;
        if !token_satisfied(version, token)? {
            return Some(false);
        }
    }
    if saw_token { Some(true) } else { None }
}

/// One Maven interval or a comma-separated union of intervals, starting with `[` or `(`.
fn maven_satisfied(version: &str, spec: &str) -> Option<bool> {
    let mut rest = spec.trim();
    let mut saw_interval = false;

    while !rest.is_empty() {
        let first = rest.chars().next()?;
        if first != '[' && first != '(' {
            return None;
        }
        let close = rest.find([']', ')'])?;
        if interval_satisfied(version, &rest[..=close])? {
            return Some(true);
        }
        saw_interval = true;
        rest = rest[close + 1..].trim_start_matches([',', ' ']);
    }

    if saw_interval { Some(false) } else { None }
}

/// A single Maven interval such as `[1.0,2.0)`, `(,1.5]` or `[1.2]`.
fn interval_satisfied(version: &str, interval: &str) -> Option<bool> {
    let lower_inclusive = interval.starts_with('[');
    let upper_inclusive = interval.ends_with(']');
    let inner = interval
        .strip_prefix(['[', '('])
        .and_then(|s| s.strip_suffix([']', ')']))
        .unwrap_or("");

    if inner.trim().is_empty() {
        return Some(true);
    }

    let parts: Vec<&str> = inner.split(',').collect();
    match parts.len() {
        0 => Some(true),
        1 => {
            // `[1.2]` is an exact pin; a bare `1.2` without brackets never reaches here.
            let only = parts[0].trim();
            if only.is_empty() {
                return Some(true);
            }
            Some(compare_strict(version, only)? == Ordering::Equal)
        }
        2 => {
            let lower = parts[0].trim();
            let upper = parts[1].trim();
            if !lower.is_empty() {
                let ord = compare_strict(version, lower)?;
                if ord == Ordering::Less || (ord == Ordering::Equal && !lower_inclusive) {
                    return Some(false);
                }
            }
            if !upper.is_empty() {
                let ord = compare_strict(version, upper)?;
                if ord == Ordering::Greater || (ord == Ordering::Equal && !upper_inclusive) {
                    return Some(false);
                }
            }
            Some(true)
        }
        _ => None,
    }
}

/// Evaluates one predicate token (`>=1.2`, `~1.2`, `1.2.x`, `*`, `1.2.3`).
fn token_satisfied(version: &str, token: &str) -> Option<bool> {
    let token = token.trim();
    if token.is_empty() {
        return Some(true);
    }
    if token == "*" || token == "x" || token == "X" {
        return Some(true);
    }

    for (op, rest) in [
        (">=", token.strip_prefix(">=")),
        ("<=", token.strip_prefix("<=")),
        (">", token.strip_prefix('>')),
        ("<", token.strip_prefix('<')),
        ("=", token.strip_prefix('=')),
    ] {
        if let Some(rest) = rest {
            let rest = rest.trim();
            if rest.is_empty() || rest == "*" {
                return Some(true);
            }
            let ord = compare_strict(version, rest)?;
            return Some(match op {
                ">=" => ord != Ordering::Less,
                "<=" => ord != Ordering::Greater,
                ">" => ord == Ordering::Greater,
                "<" => ord == Ordering::Less,
                _ => ord == Ordering::Equal,
            });
        }
    }

    if let Some(rest) = token.strip_prefix('~') {
        return tilde_satisfied(version, rest.trim());
    }
    if let Some(rest) = token.strip_prefix('^') {
        return caret_satisfied(version, rest.trim());
    }

    if has_wildcard(token) {
        return prefix_satisfied(version, token);
    }

    Some(compare_strict(version, token)? == Ordering::Equal)
}

/// `~1.2` → `>=1.2.0 <1.3.0`; `~1.2.3` → `>=1.2.3 <1.3.0`; `~1` → `>=1 <2`.
fn tilde_satisfied(version: &str, pattern: &str) -> Option<bool> {
    let lower: Vec<String> = pattern_segments(pattern)?;
    let mut upper = lower.clone();
    let bump = if upper.len() > 1 { upper.len() - 1 } else { 0 };
    bump_numeric(&mut upper, bump)?;

    let lower_ok = compare_strict(version, &lower.join("."))? != Ordering::Less;
    let upper_ok = compare_strict(version, &upper.join("."))? == Ordering::Less;
    Some(lower_ok && upper_ok)
}

/// `^1.2.3` → `>=1.2.3 <2.0.0`; `^0.2.3` → `<0.3.0`; all-zero specs bump their last segment.
fn caret_satisfied(version: &str, pattern: &str) -> Option<bool> {
    let lower: Vec<String> = pattern_segments(pattern)?;
    let numeric: Vec<Option<u64>> = lower.iter().map(|s| s.parse::<u64>().ok()).collect();
    let first_non_zero = numeric
        .iter()
        .position(|n| n.is_some_and(|v| v != 0))
        .unwrap_or(lower.len() - 1);

    let mut upper = lower[..=first_non_zero].to_vec();
    bump_numeric(&mut upper, first_non_zero)?;

    let lower_ok = compare_strict(version, &lower.join("."))? != Ordering::Less;
    let upper_ok = compare_strict(version, &upper.join("."))? == Ordering::Less;
    Some(lower_ok && upper_ok)
}

/// `1.2.x` / `1.*` → every version starting with the fixed prefix.
fn prefix_satisfied(version: &str, pattern: &str) -> Option<bool> {
    let version_segments: Vec<String> = pattern_segments(version)?;
    let pattern_segments: Vec<String> = pattern_segments(pattern)?;
    let fixed: Vec<&String> = pattern_segments
        .iter()
        .take_while(|s| !is_wildcard(s))
        .collect();

    if fixed.is_empty() {
        return Some(true);
    }
    if version_segments.len() < fixed.len() {
        return Some(false);
    }
    for (a, b) in version_segments.iter().zip(fixed) {
        if mclr_core::mcver::compare(a, b) != Ordering::Equal {
            return Some(false);
        }
    }
    Some(true)
}

fn bump_numeric(segments: &mut [String], index: usize) -> Option<()> {
    if index >= segments.len() {
        return None;
    }
    let value: u64 = segments[index].parse().ok()?;
    segments[index] = (value + 1).to_string();
    Some(())
}

fn is_wildcard(segment: &str) -> bool {
    segment == "*" || segment == "x" || segment == "X"
}

fn has_wildcard(value: &str) -> bool {
    value
        .split(SEPARATORS)
        .any(|segment| is_wildcard(segment.trim()))
}

/// Splits a version on separators; empty input yields `None`.
fn pattern_segments(value: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = value
        .trim()
        .split(SEPARATORS)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() { None } else { Some(parts) }
}
