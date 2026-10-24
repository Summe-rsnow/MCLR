//! Crash detection pipeline implementing 4-phase evaluation.

use regex::Regex;
use std::sync::LazyLock;

use crate::causes::CrashCause;
use crate::rules::modindex::CrashModIndex;
use crate::rules::stack::analyze_stack;
use crate::types::{CrashConfidence, CrashLogSet, InternalCrashFinding};

static BLOCK_LOCATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)\tBlock location: World: (\([^\)]+\))").unwrap());
static BLOCK_DESC_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)\tBlock: Block\{([^\}]+)\}").unwrap());
static ENTITY_LOCATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)\tEntity's Exact location: ([^\r\n]+)").unwrap());
static ENTITY_TYPE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)\tEntity Type: ([^\r\n]+?)(?:\s+\(|$)").unwrap());
static MOD_CONFIG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Failed loading config file\s+([^\r\n]+?)(?:\s+of type\s+[^\r\n]+?)?\s+for modid\s+([a-zA-Z0-9_\-]+)").unwrap()
});
static FABRIC_SOLUTION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s+-\s+([^\r\n]+)").unwrap());
static DUPLICATE_JAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)([a-zA-Z0-9_\-\.]+\.jar)").unwrap());

/// Runs the 4-phase detection pipeline.
pub fn run_detection(logs: &CrashLogSet, mod_index: &CrashModIndex) -> Vec<InternalCrashFinding> {
    let mut findings = Vec::new();

    // Check if there are any analyzable logs at all
    if !logs.has_analyzable_log() {
        findings.push(
            InternalCrashFinding::new(CrashCause::NoAnalyzableFile, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("No analyzable log files or output was found for this instance."),
        );
        return findings;
    }

    // Phase 1: Fatal environment / JVM / driver / memory crashes
    let fatal_findings = detect_phase1_fatal(logs, mod_index);
    let should_stop_fatal = fatal_findings.iter().any(|f| f.should_stop);
    findings.extend(fatal_findings);
    if should_stop_fatal {
        return findings;
    }

    // Phase 2: Primary game and loader exceptions
    let primary_findings = detect_phase2_primary(logs, mod_index);
    let should_stop_primary = primary_findings.iter().any(|f| f.should_stop);
    findings.extend(primary_findings);
    if should_stop_primary {
        return findings;
    }

    // Phase 3: Stack analysis and mod indexing
    if let Some(stack_finding) = analyze_stack(logs, mod_index) {
        let should_stop = stack_finding.should_stop;
        findings.push(stack_finding);
        if should_stop {
            return findings;
        }
    }

    // Suspected mod from crash report
    if let Some(suspected) = detect_suspected_mods(logs, mod_index) {
        let should_stop = suspected.should_stop;
        findings.push(suspected);
        if should_stop {
            return findings;
        }
    }

    // Phase 4: Secondary and fallback
    let secondary_findings = detect_phase4_secondary(logs);
    findings.extend(secondary_findings);

    findings
}

fn detect_phase1_fatal(
    logs: &CrashLogSet,
    _mod_index: &CrashModIndex,
) -> Vec<InternalCrashFinding> {
    let mut res = Vec::new();
    let all = &logs.all;

    // 1. OutOfMemory & X86JavaMemoryLimit
    if all.contains("Could not reserve enough space") {
        if all.contains("for 1048576KB object heap") || all.contains("Invalid maximum heap size") {
            res.push(
                InternalCrashFinding::new(CrashCause::X86JavaMemoryLimit, CrashConfidence::High)
                    .with_stop(true)
                    .with_evidence("Could not reserve enough space for 1048576KB object heap (32-bit Java limit)"),
            );
        } else {
            res.push(
                InternalCrashFinding::new(CrashCause::OutOfMemory, CrashConfidence::High)
                    .with_stop(true)
                    .with_evidence("Could not reserve enough space for object heap"),
            );
        }
    } else if all.contains("Invalid maximum heap size") {
        res.push(
            InternalCrashFinding::new(CrashCause::X86JavaMemoryLimit, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Invalid maximum heap size (32-bit Java limit)"),
        );
    } else if all.contains("java.lang.OutOfMemoryError")
        || all.contains("an out of memory error")
        || all.contains("The system is out of physical RAM or swap space")
        || all.contains("Out of Memory Error")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::OutOfMemory, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("java.lang.OutOfMemoryError"),
        );
    }

    // 2. GPU Driver Access Violations
    let is_access_violation = all.contains("EXCEPTION_ACCESS_VIOLATION");
    if (is_access_violation && all.contains("# C  [ig"))
        || all.contains("ig7icd64.dll")
        || all.contains("ig9icd64.dll")
        || all.contains("ig4icd64.dll")
    {
        res.push(
            InternalCrashFinding::new(
                CrashCause::IntelDriverAccessViolation,
                CrashConfidence::High,
            )
            .with_stop(true)
            .with_evidence("Intel HD Graphics driver crash (ig*.dll)"),
        );
    }
    if (is_access_violation && all.contains("# C  [atio"))
        || all.contains("atio6axx.dll")
        || all.contains("atig6pxx.dll")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::AmdDriverAccessViolation, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("AMD Radeon graphics driver crash (atio*.dll)"),
        );
    }
    if (is_access_violation && all.contains("# C  [nvoglv"))
        || all.contains("nvoglv64.dll")
        || all.contains("nvd3dum.dll")
    {
        res.push(
            InternalCrashFinding::new(
                CrashCause::NvidiaDriverAccessViolation,
                CrashConfidence::High,
            )
            .with_stop(true)
            .with_evidence("NVIDIA graphics driver crash (nvoglv*.dll)"),
        );
    }

    // 3. PixelFormatNotSupported
    if all.contains("Couldn't set pixel format") || all.contains("Pixel format not accelerated") {
        res.push(
            InternalCrashFinding::new(CrashCause::PixelFormatNotSupported, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Pixel format not supported or accelerated"),
        );
    }

    // 4. UnsupportedOpenGl
    if all.contains("The driver does not appear to support OpenGL") {
        res.push(
            InternalCrashFinding::new(CrashCause::UnsupportedOpenGl, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("The driver does not appear to support OpenGL"),
        );
    }

    // 5. UsingOpenJ9
    if all.contains("Open J9 is not supported")
        || all.contains("OpenJ9 is incompatible")
        || all.contains(".J9VMInternals.")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::UsingOpenJ9, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("OpenJ9 VM detected which is incompatible with Minecraft"),
        );
    }

    // 6. JavaTooNew
    if all.contains("Unable to make protected final java.lang.Class java.lang.ClassLoader.defineClass")
        || all.contains("java.lang.NoSuchFieldException: ucp")
        || all.contains("because module java.base does not export")
        || all.contains("java.lang.ClassNotFoundException: jdk.nashorn.api.scripting.NashornScriptEngineFactory")
        || all.contains("java.lang.ClassNotFoundException: java.lang.invoke.LambdaMetafactory")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::JavaTooNew, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Incompatible new Java version detected for this Minecraft/loader version"),
        );
    }

    // 7. JavaIncompatible
    if all.contains("Unsupported class file major version")
        || all.contains("Unsupported major.minor version")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::JavaIncompatible, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Java version incompatible (Unsupported class file major version)"),
        );
    }

    // 8. UsingJdk
    if all.contains("java.lang.ClassCastException: java.base/jdk")
        || all.contains("java.lang.ClassCastException: class jdk.")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::UsingJdk, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("JDK internal class cast exception (UsingJdk)"),
        );
    }

    // 9. ExtractedModFile
    if all.contains(
        "The directories below appear to be extracted jar files. Fix this before you continue.",
    ) || all.contains("Extracted mod jars found, loading will NOT continue")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::ExtractedModFile, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Extracted mod jar folder found inside mods directory"),
        );
    }

    // 10. MissingMixinBootstrap
    if all.contains("java.lang.ClassNotFoundException: org.spongepowered.asm.launch.MixinTweaker") {
        res.push(
            InternalCrashFinding::new(CrashCause::MissingMixinBootstrap, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("org.spongepowered.asm.launch.MixinTweaker not found"),
        );
    }

    // 11. InvalidModFileName
    if all.contains("Invalid module name: '' is not a Java identifier") {
        res.push(
            InternalCrashFinding::new(CrashCause::InvalidModFileName, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Invalid module name (invalid mod file name)"),
        );
    }

    // 12. MultipleForgeInInstanceJson
    if all.contains(
        "Found multiple arguments for option fml.forgeVersion, but you asked for only one",
    ) {
        res.push(
            InternalCrashFinding::new(
                CrashCause::MultipleForgeInInstanceJson,
                CrashConfidence::High,
            )
            .with_stop(true)
            .with_evidence("Multiple fml.forgeVersion arguments found in instance configuration"),
        );
    }

    // 13. ShadersModWithOptiFine
    if all.contains("java.lang.RuntimeException: Shaders Mod detected. Please remove it, OptiFine has built-in support for shaders.") {
        res.push(
            InternalCrashFinding::new(CrashCause::ShadersModWithOptiFine, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Shaders Mod detected while using OptiFine"),
        );
    }

    // 14. IncompleteForgeInstallation
    if all.contains("Cannot find launch target fmlclient, unable to launch")
        || (all.contains("Invalid paths argument, contained no existing paths")
            && (all.contains("libraries\\net\\minecraftforge\\fmlcore")
                || all.contains("libraries/net/minecraftforge/fmlcore")
                || all.contains("fmlcore")))
    {
        res.push(
            InternalCrashFinding::new(
                CrashCause::IncompleteForgeInstallation,
                CrashConfidence::High,
            )
            .with_stop(true)
            .with_evidence(
                "Incomplete Forge installation (missing fmlclient or fmlcore libraries)",
            ),
        );
    }

    res
}

fn detect_phase2_primary(
    logs: &CrashLogSet,
    mod_index: &CrashModIndex,
) -> Vec<InternalCrashFinding> {
    let mut res = Vec::new();
    let all = &logs.all;

    // 1. ManualDebugCrash
    if all.contains("Manually triggered debug crash") {
        res.push(
            InternalCrashFinding::new(CrashCause::ManualDebugCrash, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Manually triggered debug crash (F3 + C)"),
        );
    }

    // 2. DuplicateMods
    if all.contains("DuplicateModsFoundException")
        || all.contains("Found a duplicate mod")
        || all.contains("Found duplicate mods")
        || all.contains("ModResolutionException: Duplicate")
    {
        let mut jars = Vec::new();
        for caps in DUPLICATE_JAR_REGEX.captures_iter(all) {
            if let Some(m) = caps.get(1) {
                let name = m.as_str().to_string();
                if !jars.contains(&name) {
                    jars.push(name);
                }
            }
        }
        let mut finding =
            InternalCrashFinding::new(CrashCause::DuplicateMods, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Duplicate mod files found");
        if !jars.is_empty() {
            finding = finding.with_param("mods", jars.join(", "));
        }
        res.push(finding);
    }

    // 3. IncompatibleMods
    if all.contains("Incompatible mods found!") {
        let snippet = extract_snippet(all, "Incompatible mods found!", "\tat ");
        let mut finding =
            InternalCrashFinding::new(CrashCause::IncompatibleMods, CrashConfidence::High)
                .with_stop(true)
                .with_evidence(snippet.clone());
        finding = finding.with_param("detail", snippet);
        res.push(finding);
    }

    // 4. FabricSolutionProvided
    if all.contains("A potential solution has been determined:")
        || all.contains("A potential solution has been determined, this may resolve your problem:")
        || all.contains("确定了一种可能的解决方法，这样做可能会解决你的问题：")
    {
        let mut solutions = Vec::new();
        for caps in FABRIC_SOLUTION_REGEX.captures_iter(all) {
            if let Some(m) = caps.get(1) {
                solutions.push(m.as_str().trim().to_string());
            }
        }
        let sol_text = if !solutions.is_empty() {
            solutions.join("\n - ")
        } else {
            "Follow the suggestions printed by Fabric Loader.".to_string()
        };
        res.push(
            InternalCrashFinding::new(CrashCause::FabricSolutionProvided, CrashConfidence::High)
                .with_stop(true)
                .with_param("solution", sol_text.clone())
                .with_evidence(sol_text),
        );
    }

    // 5. FabricError
    if (all.contains("net.fabricmc.loader.impl.FormattedException")
        || all.contains("Fabric Loader has encountered an error"))
        && !all.contains("A potential solution has been determined")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::FabricError, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Fabric Loader encountered an unhandled error"),
        );
    }

    // 6. ForgeError
    if all.contains("An exception was thrown, the game will display an error screen and halt.") {
        let snippet = extract_snippet(
            all,
            "the game will display an error screen and halt.",
            "\n\tat",
        );
        res.push(
            InternalCrashFinding::new(CrashCause::ForgeError, CrashConfidence::High)
                .with_stop(true)
                .with_param("detail", snippet.clone())
                .with_evidence(snippet),
        );
    }

    // 7. OldForgeNewJavaIncompatible
    if all.contains("java.lang.NoSuchMethodError: sun.security.util.ManifestEntryVerifier") {
        res.push(
            InternalCrashFinding::new(
                CrashCause::OldForgeNewJavaIncompatible,
                CrashConfidence::High,
            )
            .with_stop(true)
            .with_evidence(
                "Old Forge 1.7.10/1.8 requires Java 8; incompatible with modern Java runtime",
            ),
        );
    }

    // 8. TooManyModsIdLimit
    let lower_all = all.to_ascii_lowercase();
    if lower_all.contains("maximum id range exceeded") {
        res.push(
            InternalCrashFinding::new(CrashCause::TooManyModsIdLimit, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Maximum ID range exceeded (too many mods in pre-1.13 MC)"),
        );
    }

    // 9. NightConfigBug
    if all.contains(
        "com.electronwill.nightconfig.core.io.ParsingException: Not enough data available",
    ) {
        res.push(
            InternalCrashFinding::new(CrashCause::NightConfigBug, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("NightConfig empty or corrupted config file parsing bug"),
        );
    }

    // 10. ModRequiresJava11
    if all.contains("has been compiled by a more recent version of the Java Runtime (class file version 55.0)")
        || all.contains("java.lang.RuntimeException: java.lang.NoSuchMethodException: no such method: sun.misc.Unsafe.defineAnonymousClass(Class,byte[],Object[])Class/invokeVirtual")
        || all.contains("The requested compatibility level JAVA_11 could not be set")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::ModRequiresJava11, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("One of the installed mods requires at least Java 11"),
        );
    }

    // 11. MissingDependencyOrWrongMcVersion
    if all.contains("Missing or unsupported mandatory dependencies:") {
        let snippet = extract_snippet(
            all,
            "Missing or unsupported mandatory dependencies:",
            "\n\tat",
        );
        res.push(
            InternalCrashFinding::new(
                CrashCause::MissingDependencyOrWrongMcVersion,
                CrashConfidence::High,
            )
            .with_stop(true)
            .with_param("detail", snippet.clone())
            .with_evidence(snippet),
        );
    }

    // 12. OptiFineForgeIncompatible
    if all.contains("TRANSFORMER/net.optifine/net.optifine.reflect.Reflector.<clinit>(Reflector.java")
        || all.contains("java.lang.NoSuchMethodError: 'void net.minecraft.client.renderer.texture.SpriteContents.<init>")
        || all.contains("java.lang.NoSuchMethodError: 'java.lang.String com.mojang.blaze3d.systems.RenderSystem.getBackendDescription")
        || all.contains("java.lang.NoSuchMethodError: 'void net.minecraft.client.renderer.block.model.BakedQuad.<init>")
        || all.contains("java.lang.NoSuchMethodError: 'void net.minecraftforge.client.gui.overlay.ForgeGui.renderSelectedItemName")
        || all.contains("java.lang.NoSuchMethodError: 'void net.minecraft.server.level.DistanceManager")
        || all.contains("java.lang.NoSuchMethodError: 'net.minecraft.network.chat.FormattedText net.minecraft.client.gui.Font.ellipsize")
        || (all.contains("has mods that were not found") && all.to_ascii_lowercase().contains("optifine"))
    {
        res.push(
            InternalCrashFinding::new(CrashCause::OptiFineForgeIncompatible, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("OptiFine version is incompatible with current Forge / NeoForge version"),
        );
    }

    // 13. OptiFineWorldLoadCrash
    if all.contains("java.lang.NoSuchMethodError: net.minecraft.world.server.ChunkManager$ProxyTicketManager.shouldForceTicks(J)Z")
        && all.contains("OptiFine")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::OptiFineWorldLoadCrash, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("OptiFine ChunkManager.shouldForceTicks NoSuchMethodError world load crash"),
        );
    }

    // 14. SpecificBlockCrash
    if all.contains("Block location: World: ")
        || all.contains("Block: Block{")
        || all.contains("Block being ticked")
    {
        let mut block_info = String::new();
        if let Some(caps) = BLOCK_DESC_REGEX.captures(all) {
            block_info.push_str(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        }
        if let Some(caps) = BLOCK_LOCATION_REGEX.captures(all) {
            if !block_info.is_empty() {
                block_info.push(' ');
            }
            block_info.push_str(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        }
        let desc = if !block_info.is_empty() {
            block_info
        } else {
            "Specific block ticking crash".to_string()
        };
        res.push(
            InternalCrashFinding::new(CrashCause::SpecificBlockCrash, CrashConfidence::High)
                .with_stop(true)
                .with_param("block", desc.clone())
                .with_evidence(desc),
        );
    }

    // 15. SpecificEntityCrash
    if (all.contains("Entity Type: ") && all.contains("Entity's Exact location: "))
        || all.contains("Entity being ticked")
    {
        let mut entity_info = String::new();
        if let Some(caps) = ENTITY_TYPE_REGEX.captures(all) {
            entity_info.push_str(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        }
        if let Some(caps) = ENTITY_LOCATION_REGEX.captures(all) {
            if !entity_info.is_empty() {
                entity_info.push_str(" at ");
            }
            entity_info.push_str(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        }
        let desc = if !entity_info.is_empty() {
            entity_info
        } else {
            "Specific entity ticking crash".to_string()
        };
        res.push(
            InternalCrashFinding::new(CrashCause::SpecificEntityCrash, CrashConfidence::High)
                .with_stop(true)
                .with_param("entity", desc.clone())
                .with_evidence(desc),
        );
    }

    // 16. ResourcePackTooLarge
    if all.contains("Maybe try a lower resolution resourcepack?")
        || all.contains("StitcherException")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::ResourcePackTooLarge, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("StitcherException: Texture stitched exceeded maximum texture size / resource pack too large"),
        );
    }

    // 17. ModMixinFailed
    if looks_like_mixin_failure(all) {
        let hints = extract_mixin_mod_hints(all);
        let resolved = mod_index.resolve_to_display_names(&hints);
        let mut finding =
            InternalCrashFinding::new(CrashCause::ModMixinFailed, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Mixin injection or apply failure");
        if !resolved.is_empty() {
            finding = finding.with_param("mod", resolved.join(", "));
            finding = finding.with_param("mods", resolved.join("\n - "));
        }
        res.push(finding);
    }

    // 18. ModConfigCrash
    if all.contains("Failed loading config file ") || all.contains("FailedLoadingConfigException") {
        let mut finding =
            InternalCrashFinding::new(CrashCause::ModConfigCrash, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Failed loading mod configuration file");
        if let Some(caps) = MOD_CONFIG_REGEX.captures(all) {
            if let Some(cfg) = caps.get(1) {
                finding = finding.with_param("config", cfg.as_str());
            }
            if let Some(m) = caps.get(2) {
                let resolved = mod_index.resolve_to_display_names(&[m.as_str().to_string()]);
                finding = finding.with_param("mod", resolved.join(", "));
            }
        }
        res.push(finding);
    }

    // 19. ModInitializationFailed
    if all.contains("Failed to create mod instance.") {
        let regex = Regex::new(
            r"(?i)Failed to create mod instance\.\s*(?:ModID:\s*|ModId\s*)([a-zA-Z0-9_\-]+)",
        )
        .unwrap();
        let mut finding =
            InternalCrashFinding::new(CrashCause::ModInitializationFailed, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Failed to create mod instance during initialization");
        if let Some(caps) = regex.captures(all)
            && let Some(m) = caps.get(1)
        {
            let resolved = mod_index.resolve_to_display_names(&[m.as_str().to_string()]);
            finding = finding.with_param("mod", resolved.join(", "));
        }
        res.push(finding);
    }

    // 20. ModLoaderError
    if all.contains("Mod resolution failed") {
        res.push(
            InternalCrashFinding::new(CrashCause::ModLoaderError, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Mod resolution failed in loader"),
        );
    }

    // 21. ConfirmedModCrash
    let mut confirmed_mod_names = Vec::new();
    if all.contains("Caught exception from ") {
        let regex = Regex::new(r"(?m)Caught exception from\s+([^\r\n]+)").unwrap();
        for caps in regex.captures_iter(all) {
            if let Some(m) = caps.get(1) {
                confirmed_mod_names.push(m.as_str().trim().to_string());
            }
        }
    }
    if all.contains("LoaderExceptionModCrash: Caught exception from ") {
        let regex =
            Regex::new(r"(?m)LoaderExceptionModCrash: Caught exception from\s+([^\r\n]+)").unwrap();
        for caps in regex.captures_iter(all) {
            if let Some(m) = caps.get(1) {
                confirmed_mod_names.push(m.as_str().trim().to_string());
            }
        }
    }
    if all.contains("due to errors, provided by ") {
        let regex = Regex::new(r"(?i)due to errors, provided by\s+'([^']+)'").unwrap();
        for caps in regex.captures_iter(all) {
            if let Some(m) = caps.get(1) {
                confirmed_mod_names.push(m.as_str().trim().to_string());
            }
        }
    }
    if all.contains("Multiple entries with same key: ") {
        let regex = Regex::new(r"(?i)Multiple entries with same key:\s*([^=]+)").unwrap();
        if let Some(caps) = regex.captures(all)
            && let Some(m) = caps.get(1)
        {
            confirmed_mod_names.push(m.as_str().trim().to_string());
        }
    }
    if !confirmed_mod_names.is_empty() {
        let resolved = mod_index.resolve_to_display_names(&confirmed_mod_names);
        let mut finding =
            InternalCrashFinding::new(CrashCause::ConfirmedModCrash, CrashConfidence::High)
                .with_stop(true)
                .with_evidence("Explicit mod exception caught in crash report");
        finding = finding.with_param("mod", resolved.join(", "));
        finding = finding.with_param("mods", resolved.join("\n - "));
        res.push(finding);
    }

    res
}

fn detect_suspected_mods(
    logs: &CrashLogSet,
    mod_index: &CrashModIndex,
) -> Option<InternalCrashFinding> {
    let cr = logs.crash_report.as_ref()?;
    if !cr.text.contains("Suspected Mod") {
        return None;
    }
    let start_pos = cr.text.find("Suspected Mod")?;
    let slice = &cr.text[start_pos..];
    let end_pos = slice.find("Stacktrace").unwrap_or(slice.len());
    let section = &slice[..end_pos];

    if section.contains("s: None") {
        return None;
    }

    let regex = Regex::new(r"(?m)(?:^\t|[ :])[^\(\t\r\n:]+\(([^\)\r\n]+)\)").unwrap();
    let mut suspects = Vec::new();
    for caps in regex.captures_iter(section) {
        if let Some(m) = caps.get(1) {
            suspects.push(m.as_str().trim().to_string());
        }
    }

    if suspects.is_empty() {
        return None;
    }

    let resolved = mod_index.resolve_to_display_names(&suspects);
    let mut finding =
        InternalCrashFinding::new(CrashCause::SuspectedModCrash, CrashConfidence::Medium)
            .with_stop(true)
            .with_evidence("Mod suspected by loader crash handler");
    finding = finding.with_param("mod", resolved.join(", "));
    finding = finding.with_param("mods", resolved.join("\n - "));
    Some(finding)
}

fn detect_phase4_secondary(logs: &CrashLogSet) -> Vec<InternalCrashFinding> {
    let mut res = Vec::new();
    let all = &logs.all;

    // 1. OpenGl1282
    if all.contains("1282: Invalid operation")
        || all.contains("GL error 1282")
        || all.contains("OpenGL Error: 1282")
    {
        res.push(
            InternalCrashFinding::new(CrashCause::OpenGl1282, CrashConfidence::Medium)
                .with_evidence("OpenGL Error 1282 (Invalid operation)"),
        );
    }

    // 2. FileOrContentValidationFailed
    if all.contains(
        "signer information does not match signer information of other classes in the same package",
    ) {
        res.push(
            InternalCrashFinding::new(
                CrashCause::FileOrContentValidationFailed,
                CrashConfidence::High,
            )
            .with_evidence(
                "Jar signer information does not match other classes in the same package",
            ),
        );
    }

    // 3. VeryShortOutput
    if logs.hs_err.is_none()
        && logs.crash_report.is_none()
        && let Some(game) = &logs.game
    {
        let t = game.text.trim();
        if !t.is_empty() && t.len() < 100 && !t.contains("at net.") && !t.contains("INFO]") {
            res.push(
                InternalCrashFinding::new(CrashCause::VeryShortOutput, CrashConfidence::Low)
                    .with_param("output", t)
                    .with_evidence(
                        "Game output was abruptly terminated with fewer than 100 characters.",
                    ),
            );
        }
    }

    res
}

fn looks_like_mixin_failure(text: &str) -> bool {
    text.contains("Mixin prepare failed ")
        || text.contains("Mixin apply failed ")
        || text.contains("MixinApplyError")
        || text.contains("MixinPrepareError")
        || text.contains("MixinTransformerError")
        || text.contains("mixin.injection.throwables.")
        || text.contains(".json] FAILED during )")
}

fn extract_mixin_mod_hints(text: &str) -> Vec<String> {
    let mut hints = Vec::new();
    let r1 = Regex::new(r"(?i)from mod\s+([^\./\s\]]+)").unwrap();
    for caps in r1.captures_iter(text) {
        if let Some(m) = caps.get(1) {
            hints.push(m.as_str().to_string());
        }
    }
    let r2 = Regex::new(r"(?i)for mod\s+([^\./\s]+)\s+failed").unwrap();
    for caps in r2.captures_iter(text) {
        if let Some(m) = caps.get(1) {
            hints.push(m.as_str().to_string());
        }
    }
    let r3 = Regex::new(r"(?m)(?:^|[ \[\{(])([a-zA-Z0-9_\-]+)\.mixin[s]?\.json").unwrap();
    for caps in r3.captures_iter(text) {
        if let Some(m) = caps.get(1) {
            hints.push(m.as_str().to_string());
        }
    }
    hints
}

fn extract_snippet(text: &str, start_marker: &str, end_marker: &str) -> String {
    if let Some(start) = text.find(start_marker) {
        let after = &text[start..];
        if let Some(end) = after.find(end_marker) {
            return after[..end].trim().to_string();
        }
        let end_line = after.lines().take(5).collect::<Vec<_>>().join("\n");
        return end_line.trim().to_string();
    }
    String::new()
}
