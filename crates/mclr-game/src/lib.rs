//! Minecraft version model, instances, Java discovery, installation, and launch pipeline.
#![allow(
    clippy::collapsible_if,
    clippy::double_ended_iterator_last,
    clippy::field_reassign_with_default,
    clippy::map_entry
)]

pub mod error;
pub mod install;
pub mod instance;
pub mod java;
pub mod version;

pub use error::GameError;
pub use install::{
    VersionManifestEntry, VersionManifestList, collect_version_download_specs,
    fetch_version_manifest, install_vanilla, load_resolved_version_json, maven_coordinate_to_path,
    repair_instance,
};
pub use instance::{
    InstanceConfig, InstanceInfo, InstanceKind, InstanceState, classify_instance, delete_instance,
    duplicate_instance, rename_instance, save_instance_config, scan_instances,
    validate_instance_name,
};
pub use java::{
    JavaConstraint, JavaInstallation, RuntimeFileSpec, parse_major_version, parse_release_content,
    resolve_mojang_runtime_downloads, scan_java, select_java,
};
pub use version::{
    ArgumentEntry, ArgumentValue, Arguments, Artifact, AssetIndexInfo, ConditionalArgument,
    DownloadEntry, ExtractRules, JavaVersionInfo, Library, LibraryDownloads, LoggingClient,
    LoggingFile, LoggingInfo, OsRule, Rule, RuleContext, VersionDownloads, VersionJson,
    evaluate_rules, library_dedup_key, merge_version_json,
};
