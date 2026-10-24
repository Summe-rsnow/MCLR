//! Community platforms (Modrinth, CurseForge), dependency resolution, local mods, JarInJar, and modpack import/export.

pub mod curseforge;
pub mod error;
pub mod model;
pub mod modrinth;

pub use curseforge::CurseForgeClient;
pub use error::CompError;
pub use model::{
    CompDependency, CompDependencyKind, CompFile, CompFileHashes, CompPlatform, CompProject,
    CompReleaseChannel, CompSearchQuery, CompSearchResult, CompSort, CompType,
};
pub use modrinth::ModrinthClient;
