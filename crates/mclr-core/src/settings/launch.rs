//! Global game launch settings (`settings.launch`).

use serde::{Deserialize, Serialize};
use specta::Type;

/// Recommended JVM arguments, mirrored by `DEFAULT_JVM_ARGS` in
/// `src/features/settings-game/SettingsGamePage.vue`; both must stay identical.
pub const DEFAULT_JVM_ARGS: &str = "-XX:+UseG1GC -XX:+UseAdaptiveSizePolicy \
-XX:-OmitStackTraceInFastThrow -Djdk.lang.Process.allowAmbiguousCommands=true \
-Dfml.ignoreInvalidMinecraftCertificates=True -Dfml.ignorePatchDiscrepancies=True \
-Dlog4j2.formatMsgNoLookups=true";

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct LaunchSettings {
    pub ram_mode: String, // "auto" | "custom"
    pub ram_custom_mb: u32,
    pub jvm_args: String,
    pub game_args: String,
    pub window_title: String,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
    pub preferred_ip_stack: String,      // "auto" | "ipv4" | "ipv6"
    pub process_priority: String,        // "normal" | "aboveNormal" | "high"
    pub close_launcher_on_start: String, // "keep" | "hide" | "exit"
    pub disable_crash_analysis: bool,
    /// Software renderer bridge: `"default" | "software" | "d3d12" | "vulkan"`.
    /// Only applied when the bundled `mesa-loader` agent is available.
    pub renderer: String,
    /// Global default pre-launch command (instance setting wins when set).
    pub pre_launch_command: String,
    /// Launch `java.exe` instead of `javaw.exe` so a console window stays attached.
    pub use_java_exe: bool,
    /// Register the Java executable as high-performance GPU in Windows settings.
    pub require_gpu_high_performance: bool,
    /// Force `-Xms` to equal `-Xmx` (fixed heap, fewer GC pauses).
    pub lock_memory: bool,
}

impl Default for LaunchSettings {
    fn default() -> Self {
        Self {
            ram_mode: "auto".to_string(),
            ram_custom_mb: 4096,
            jvm_args: DEFAULT_JVM_ARGS.to_string(),
            game_args: String::new(),
            window_title: String::new(),
            fullscreen: false,
            width: 854,
            height: 480,
            preferred_ip_stack: "auto".to_string(),
            process_priority: "normal".to_string(),
            close_launcher_on_start: "keep".to_string(),
            disable_crash_analysis: false,
            renderer: "default".to_string(),
            pre_launch_command: String::new(),
            use_java_exe: false,
            require_gpu_high_performance: true,
            lock_memory: false,
        }
    }
}
