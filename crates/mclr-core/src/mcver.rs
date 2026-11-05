use std::cmp::Ordering;

pub const SEPARATORS: [char; 4] = ['.', '-', '+', '_'];

/// Total order comparison between two version strings.
///
/// Segments are split on `SEPARATORS`, empty segments dropped.
/// A missing segment is treated as `"0"`.
/// Numeric segments compare numerically (leading zeros stripped, then length, then lexicographic),
/// numeric segments rank above non-numeric segments, and two non-numeric segments compare lexicographically.
pub fn compare(a: &str, b: &str) -> Ordering {
    let mut it_a = a
        .trim()
        .split(SEPARATORS)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let mut it_b = b
        .trim()
        .split(SEPARATORS)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    loop {
        let next_a = it_a.next();
        let next_b = it_b.next();
        if next_a.is_none() && next_b.is_none() {
            return Ordering::Equal;
        }
        let a_seg = next_a.unwrap_or("0");
        let b_seg = next_b.unwrap_or("0");
        match cmp_segment(a_seg, b_seg) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
}

/// Compares two versions segment by segment, returning `None` if either side
/// has no segments or starts with a non-numeric segment (placeholder / unparsable).
pub fn compare_strict(a: &str, b: &str) -> Option<Ordering> {
    let first_a = a
        .trim()
        .split(SEPARATORS)
        .map(str::trim)
        .find(|s| !s.is_empty())?;
    let first_b = b
        .trim()
        .split(SEPARATORS)
        .map(str::trim)
        .find(|s| !s.is_empty())?;
    numeric(first_a)?;
    numeric(first_b)?;
    Some(compare(a, b))
}

fn cmp_segment(a: &str, b: &str) -> Ordering {
    match (numeric(a), numeric(b)) {
        (Some(x), Some(y)) => compare_numeric(x, y),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => a.cmp(b),
    }
}

fn numeric(segment: &str) -> Option<&str> {
    if segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let trimmed = segment.trim_start_matches('0');
    Some(if trimmed.is_empty() { "0" } else { trimmed })
}

fn compare_numeric(a: &str, b: &str) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

/// Returns true if `candidate` is newer than `current`.
///
/// Strips leading `'v'`, splits at the first `'-'` into numeric and pre-release parts.
/// The numeric part compares via [`compare`], stable release ranks above pre-release,
/// and two pre-releases compare lexicographically.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    fn parse_tuple(v: &str) -> (&str, Option<&str>) {
        let cleaned = v.trim().trim_start_matches('v');
        match cleaned.find('-') {
            Some(idx) => (&cleaned[..idx], Some(&cleaned[idx + 1..])),
            None => (cleaned, None),
        }
    }

    let (c_num, c_pre) = parse_tuple(candidate);
    let (cur_num, cur_pre) = parse_tuple(current);

    match compare(c_num, cur_num) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match (c_pre, cur_pre) {
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (Some(c_p), Some(cur_p)) => c_p > cur_p,
            (None, None) => false,
        },
    }
}

/// `true` for plain release ids (`1.20.1`, `1.7`, `26.1`), `false` for snapshots/pre-releases/old versions.
pub fn is_release(mc_version: &str) -> bool {
    let mut parts = mc_version.split('.');
    let (Some(a), Some(b)) = (parts.next(), parts.next()) else {
        return false;
    };
    let rest = parts.next();
    parts.next().is_none()
        && !a.is_empty()
        && a.bytes().all(|c| c.is_ascii_digit())
        && !b.is_empty()
        && b.bytes().all(|c| c.is_ascii_digit())
        && rest.is_none_or(|c| !c.is_empty() && c.bytes().all(|d| d.is_ascii_digit()))
}

/// Calculates drop score for Minecraft versions (e.g. 1.20.1 -> 200, 1.14 -> 140).
pub fn drop_of(mc_version: &str) -> u32 {
    let clean = mc_version.trim();
    if let Some(rest) = clean.strip_prefix("1.") {
        let minor = rest
            .split('.')
            .next()
            .and_then(|m| m.parse::<u32>().ok())
            .unwrap_or(0);
        return minor * 10;
    }

    // Modern 2-digit versions e.g. 26.1 -> 260
    if let Some(pos) = clean.find('.')
        && let Ok(first) = clean[..pos].parse::<u32>()
        && first >= 26
    {
        return first * 10;
    }

    // Snapshots: 24w14a -> 210
    if clean.len() >= 3 && clean.chars().take(2).all(|c| c.is_ascii_digit()) && clean.contains('w')
    {
        let yr = clean[..2].parse::<u32>().unwrap_or(0);
        return match yr {
            y if y >= 24 => 210,
            23 => 200,
            22 => 190,
            21 => 180,
            20 => 160,
            19 => 150,
            18 => 130,
            17 => 120,
            16 => 110,
            15 => 90,
            _ => 0,
        };
    }

    0
}

/// Major version key for release versions (e.g. `"1.20.1"` -> `"1.20"`, `"26.1.2"` -> `"26.1"`).
/// Returns `None` for non-release versions.
pub fn major_key(id: &str) -> Option<&str> {
    if !is_release(id) {
        return None;
    }
    let mut dot_count = 0;
    for (idx, b) in id.bytes().enumerate() {
        if b == b'.' {
            dot_count += 1;
            if dot_count == 2 {
                return Some(&id[..idx]);
            }
        }
    }
    Some(id)
}

/// Returns trimmed value, or `None` when empty or case-insensitively `"unknown"`.
pub fn known(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") {
        None
    } else {
        Some(trimmed)
    }
}
