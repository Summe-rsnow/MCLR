//! Community platforms (Modrinth, CurseForge), dependency resolution, local mods, JarInJar, and modpack import/export.

pub mod curseforge;
pub mod error;
pub mod export;
pub mod jarinjar;
pub mod local;
pub mod model;
pub mod modpack;
pub mod modrinth;
pub mod resolver;
pub mod resources;

pub use curseforge::CurseForgeClient;
pub use error::CompError;
pub use export::{
    ExportCandidateEntry, ModpackExportFormat, ModpackExportOptions, ModpackExportParams,
    export_modpack, scan_export_candidates,
};
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
    ModpackFileDownload, ModpackFormat, ModpackImportContext, ModpackInspectResult,
    ModpackInstallPlan, extract_modpack_overrides, import_modpack, inspect_modpack,
};
pub use modrinth::ModrinthClient;
pub use resolver::{ResolvedDependencyItem, resolve_dependencies, select_best_file};
pub use resources::{
    ImportResourcesResult, OverwritePolicy, ResourceEntry, ResourceKind, ResourceState,
    SchematicInfo, delete_resources, export_resource_list, import_resources, scan_resources,
    schematic_info, toggle_resource,
};
