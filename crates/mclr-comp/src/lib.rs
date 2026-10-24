//! Community platforms (Modrinth, CurseForge), dependency resolution, local mods, JarInJar, and modpack import/export.

pub mod curseforge;
pub mod error;
pub mod jarinjar;
pub mod local;
pub mod model;
pub mod modpack;
pub mod modrinth;
pub mod resolver;

pub use curseforge::CurseForgeClient;
pub use error::CompError;
pub use jarinjar::{JarInJarNode, scan_jar_in_jar};
pub use local::{
    LocalModInfo, check_local_mods_updates, compute_curseforge_murmur2, delete_local_mod,
    scan_local_mods, toggle_local_mod,
};
pub use model::{
    CompDependency, CompDependencyKind, CompFile, CompFileHashes, CompPlatform, CompProject,
    CompReleaseChannel, CompSearchQuery, CompSearchResult, CompSort, CompType,
};
pub use modpack::{
    ModpackFileDownload, ModpackFormat, ModpackInspectResult, ModpackInstallPlan,
    extract_modpack_overrides, import_modpack, inspect_modpack,
};
pub use modrinth::ModrinthClient;
pub use resolver::{ResolvedDependencyItem, resolve_dependencies, select_best_file};
