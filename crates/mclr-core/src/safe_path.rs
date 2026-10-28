//! Containment helpers for paths that come from untrusted input
//! (modpack indexes, installer jars, IPC arguments).

use std::path::{Component, Path, PathBuf};

/// Joins an untrusted relative path onto `base`, refusing anything that could
/// escape it.
///
/// Rejects absolute paths, `..` segments, Windows drive/UNC prefixes and root
/// components, then double-checks the result still starts with `base`. Callers
/// that write files based on remote metadata (modpack `files[].path`, installer
/// `maven/` entries) must go through this.
pub fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let normalized = rel.replace('\\', "/");
    let candidate = Path::new(&normalized);
    if candidate.is_absolute() {
        return None;
    }
    if candidate.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    let joined = base.join(candidate);
    joined.starts_with(base).then_some(joined)
}

#[cfg(test)]
mod tests {
    use super::safe_join;
    use std::path::Path;

    #[test]
    fn rejects_escapes_and_keeps_normal_paths() {
        let base = Path::new("C:/mc/versions/inst");
        assert_eq!(
            safe_join(base, "mods/example.jar"),
            Some(base.join("mods").join("example.jar"))
        );
        assert_eq!(
            safe_join(base, "mods\\example.jar"),
            Some(base.join("mods").join("example.jar"))
        );

        assert_eq!(safe_join(base, "../../evil.jar"), None);
        assert_eq!(safe_join(base, "mods/../../evil.jar"), None);
        assert_eq!(safe_join(base, "/evil.jar"), None);
        assert_eq!(safe_join(base, "C:/evil.jar"), None);
        assert_eq!(safe_join(base, "\\\\server\\share\\evil.jar"), None);
    }
}
