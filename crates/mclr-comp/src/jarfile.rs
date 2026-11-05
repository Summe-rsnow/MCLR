use std::io::{Read, Seek};
use zip::ZipArchive;

pub fn read_entry_text<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

/// Reads a small archive entry fully. Icons are capped at 2 MiB so a crafted jar cannot
/// force a large allocation.
pub fn read_entry_bytes<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Option<Vec<u8>> {
    const MAX_ICON_BYTES: u64 = 2 * 1024 * 1024;
    let mut entry = archive.by_name(name).ok()?;
    if entry.size() > MAX_ICON_BYTES {
        return None;
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

pub fn is_jar(name: &str) -> bool {
    name.ends_with(".jar")
}

pub fn is_disabled_jar(name: &str) -> bool {
    name.ends_with(".jar.disabled")
}

pub fn jar_stem(name: &str) -> &str {
    if let Some(stem) = name.strip_suffix(".jar.disabled") {
        stem
    } else if let Some(stem) = name.strip_suffix(".jar") {
        stem
    } else {
        name
    }
}

pub fn toggled_name(name: &str) -> Option<String> {
    if let Some(stem) = name.strip_suffix(".jar.disabled") {
        Some(format!("{stem}.jar"))
    } else if name.ends_with(".jar") {
        Some(format!("{name}.disabled"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jarfile_helpers() {
        assert!(is_jar("foo.jar"));
        assert!(!is_jar("foo.jar.disabled"));
        assert!(!is_jar("foo.zip"));

        assert!(is_disabled_jar("foo.jar.disabled"));
        assert!(!is_disabled_jar("foo.jar"));

        assert_eq!(jar_stem("foo.jar"), "foo");
        assert_eq!(jar_stem("foo.jar.disabled"), "foo");
        assert_eq!(jar_stem("foo.txt"), "foo.txt");

        assert_eq!(
            toggled_name("foo.jar"),
            Some("foo.jar.disabled".to_string())
        );
        assert_eq!(
            toggled_name("foo.jar.disabled"),
            Some("foo.jar".to_string())
        );
        assert_eq!(toggled_name("foo.txt"), None);
    }
}
