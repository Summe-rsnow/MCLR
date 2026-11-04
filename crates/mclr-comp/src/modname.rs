//! Chinese mod name dictionary lookup (`mcmod.buf`).
//!
//! `data/mcmod.buf` is a gzip-compressed protobuf-net serialized `List<entry>`:
//! the root is a repeated field `1` whose elements are messages carrying
//!
//! | field | type   | content   |
//! |-------|--------|-----------|
//! | 1     | varint | `wiki_id` |
//! | 2     | string | `zh_name` |
//! | 3     | string | `cf_slug` |
//! | 4     | string | `mr_slug` |
//!
//! The table is decoded once, lazily, into a slug -> entry index. Unknown fields
//! and wire types are skipped so a data revision cannot break decoding.

use std::collections::HashMap;
use std::io::Read;
use std::sync::LazyLock;

use flate2::read::GzDecoder;

const RAW_DATA: &[u8] = include_bytes!("../data/mcmod.buf");

/// One Chinese-name dictionary entry, keyed by CurseForge / Modrinth slug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModNameEntry {
    /// mcmod.cn class id, used to build `https://www.mcmod.cn/class/{id}.html`.
    pub wiki_id: u32,
    /// Raw Chinese name as stored in the dictionary; may carry a trailing
    /// `(English Name)` or `- Alias` suffix. Use [`display_zh`] for display.
    pub zh_name: String,
    /// CurseForge slug (may be empty).
    pub cf_slug: String,
    /// Modrinth slug (may be empty).
    pub mr_slug: String,
}

struct Index {
    entries: Vec<ModNameEntry>,
    /// Lowercase slug -> index into `entries`. Both slug columns are indexed.
    by_slug: HashMap<String, usize>,
}

impl Index {
    fn build() -> Self {
        let entries = decode_entries(RAW_DATA);
        let mut by_slug = HashMap::with_capacity(entries.len() * 2);
        for (idx, entry) in entries.iter().enumerate() {
            for slug in [entry.cf_slug.as_str(), entry.mr_slug.as_str()] {
                let key = slug.trim().to_ascii_lowercase();
                if !key.is_empty() {
                    by_slug.entry(key).or_insert(idx);
                }
            }
        }
        Self { entries, by_slug }
    }
}

static INDEX: LazyLock<Index> = LazyLock::new(Index::build);

/// Looks up a dictionary entry by CurseForge or Modrinth slug (case-insensitive).
///
/// Returns `None` for empty slugs and for slugs absent from the dictionary.
pub fn lookup_by_slug(slug: &str) -> Option<&'static ModNameEntry> {
    let key = slug.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }
    INDEX.by_slug.get(&key).map(|&idx| &INDEX.entries[idx])
}

/// Builds the mcmod.cn wiki page URL for a class id.
pub fn wiki_url(wiki_id: u32) -> String {
    format!("https://www.mcmod.cn/class/{wiki_id}.html")
}

/// Trims the English-name / alias suffixes from a dictionary Chinese name.
///
/// `"JEI物品管理器 (Just Enough Items)"` -> `"JEI物品管理器"`,
/// `"等价交换 - ProjectE"` -> `"等价交换"`.
pub fn display_zh(entry: &ModNameEntry) -> &str {
    entry
        .zh_name
        .split(" (")
        .next()
        .unwrap_or(&entry.zh_name)
        .split(" - ")
        .next()
        .unwrap_or(&entry.zh_name)
        .trim()
}

const WIRE_VARINT: u64 = 0;
const WIRE_64BIT: u64 = 1;
const WIRE_LEN: u64 = 2;
const WIRE_32BIT: u64 = 5;

/// Reads a base-128 varint. Returns the value and the position after it.
fn read_varint(buf: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    while pos < buf.len() {
        let byte = buf[pos];
        pos += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((value, pos));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

/// One decoded protobuf field value.
enum Field<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
    Other,
}

/// Reads the field starting at `pos`. Returns the field number, its value and
/// the position just past it. `None` means malformed input or end of buffer.
fn next_field(buf: &[u8], pos: usize) -> Option<(u64, Field<'_>, usize)> {
    let (key, cur) = read_varint(buf, pos)?;
    let field = key >> 3;
    match key & 7 {
        WIRE_VARINT => {
            let (value, next) = read_varint(buf, cur)?;
            Some((field, Field::Varint(value), next))
        }
        WIRE_LEN => {
            let (len, start) = read_varint(buf, cur)?;
            let end = start.checked_add(len as usize)?;
            if end > buf.len() {
                return None;
            }
            Some((field, Field::Bytes(&buf[start..end]), end))
        }
        WIRE_64BIT => {
            let end = cur.checked_add(8)?;
            (end <= buf.len()).then_some((field, Field::Other, end))
        }
        WIRE_32BIT => {
            let end = cur.checked_add(4)?;
            (end <= buf.len()).then_some((field, Field::Other, end))
        }
        _ => None,
    }
}

fn decode_entry(buf: &[u8]) -> ModNameEntry {
    let mut entry = ModNameEntry {
        wiki_id: 0,
        zh_name: String::new(),
        cf_slug: String::new(),
        mr_slug: String::new(),
    };
    let mut pos = 0usize;
    while pos < buf.len() {
        let Some((field, value, next)) = next_field(buf, pos) else {
            break;
        };
        pos = next;
        match (field, value) {
            (1, Field::Varint(v)) => entry.wiki_id = v as u32,
            (2, Field::Bytes(b)) => entry.zh_name = String::from_utf8_lossy(b).into_owned(),
            (3, Field::Bytes(b)) => entry.cf_slug = String::from_utf8_lossy(b).into_owned(),
            (4, Field::Bytes(b)) => entry.mr_slug = String::from_utf8_lossy(b).into_owned(),
            _ => {}
        }
    }
    entry
}

fn decode_entries(data: &[u8]) -> Vec<ModNameEntry> {
    let mut raw = Vec::new();
    if GzDecoder::new(data).read_to_end(&mut raw).is_err() {
        return Vec::new();
    }

    // Root: `repeated entry` in field 1; every other field is skipped.
    let mut entries = Vec::new();
    let mut pos = 0usize;
    while pos < raw.len() {
        let Some((field, value, next)) = next_field(&raw, pos) else {
            break;
        };
        pos = next;
        if let (1, Field::Bytes(payload)) = (field, value) {
            entries.push(decode_entry(payload));
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_bundled_dictionary() {
        let entries = decode_entries(RAW_DATA);
        assert!(entries.len() > 10_000, "got {} entries", entries.len());
        assert!(entries.iter().any(|e| e.mr_slug == "jei"));
    }
}
