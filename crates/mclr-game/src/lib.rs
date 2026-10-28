//! Minecraft version model, instances, Java discovery, installation, and launch pipeline.
#![allow(
    clippy::collapsible_if,
    clippy::double_ended_iterator_last,
    clippy::field_reassign_with_default,
    clippy::map_entry
)]

pub mod compat;
pub mod error;
pub mod gamedir;
pub mod install;
pub mod instance;
pub mod java;
pub mod launch;
pub mod naming;
pub mod version;
pub use compat::{VersionCompatInfo, compute_version_compat};
pub use error::GameError;
pub use install::{
    RepairMode, VersionManifestEntry, VersionManifestList, collect_version_download_specs,
    fetch_version_manifest, install_vanilla, load_resolved_version_json, maven_coordinate_to_path,
    repair_instance,
};
pub use instance::{
    InstanceConfig, InstanceInfo, InstanceKind, InstanceState, classify_instance, delete_instance,
    duplicate_instance, rename_instance, save_instance_config, scan_instances,
    validate_instance_name,
};
pub use java::{
    JavaConstraint, JavaInstallation, parse_major_version, parse_release_content, scan_java,
    select_java,
};
pub use launch::{
    LaunchCommandInput, LaunchCommandSpec, LaunchExitPayload, LaunchPipelineParams,
    LaunchProcessInfo, LaunchStartResult, LaunchWatcher, LogLevel, LogLine, WatcherEvent,
    build_launch_command, check_launch_paths, check_login_requirement, classify_log_level,
    compute_ram_mb, compute_ram_mb_auto, ensure_authlib_injector, execute_pre_launch_command,
    extract_natives, find_window_for_pid, get_free_physical_memory_mb, launch,
    resolve_java_installation, resolve_java_installation_with_component, sanitize_log_line,
    set_window_title_for_pid, version_java_component, version_java_constraint,
};
pub use version::{
    ArgumentEntry, ArgumentValue, Arguments, Artifact, AssetIndexInfo, ConditionalArgument,
    DownloadEntry, ExtractRules, JavaVersionInfo, Library, LibraryDownloads, LoggingClient,
    LoggingFile, LoggingInfo, OsRule, Rule, RuleContext, VersionDownloads, VersionJson,
    evaluate_rules, library_dedup_key, merge_version_json,
};
