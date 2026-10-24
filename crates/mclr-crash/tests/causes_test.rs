use std::collections::HashSet;
use std::path::Path;

use mclr_crash::causes::CrashCause;
use mclr_crash::preparer::{mask_sensitive, normalize_crlf, prepare_text, strip_ansi};
use mclr_crash::rules::analyze_crash;
use mclr_crash::types::{CollectedLogEntry, CrashLogKind, CrashLogSet};

#[test]
fn test_all_45_causes_count_and_unique_codes() {
    assert_eq!(CrashCause::ALL.len(), 45);

    let mut codes = HashSet::new();
    for cause in CrashCause::ALL {
        let code = cause.code();
        assert!(!code.is_empty(), "Code should not be empty");
        assert!(
            codes.insert(code),
            "Duplicate crash cause code found: {code}"
        );
        assert_eq!(
            CrashCause::from_code(code),
            Some(cause),
            "Round-trip from_code failed for {code}"
        );
    }
    assert_eq!(codes.len(), 45);
}

fn analyze_text(text: &str) -> mclr_crash::types::CrashResult {
    let dummy_dir = Path::new("dummy");
    let mut log_set = CrashLogSet::default();
    let prepared = prepare_text(text);
    log_set.all = prepared.clone();
    log_set.game = Some(CollectedLogEntry {
        path: dummy_dir.join("latest.log"),
        kind: CrashLogKind::Game,
        lines: prepared.lines().map(|s| s.to_string()).collect(),
        text: prepared,
    });
    analyze_crash("test-inst", dummy_dir, &log_set)
}

fn analyze_crash_report(text: &str) -> mclr_crash::types::CrashResult {
    let dummy_dir = Path::new("dummy");
    let mut log_set = CrashLogSet::default();
    let prepared = prepare_text(text);
    log_set.all = prepared.clone();
    log_set.crash_report = Some(CollectedLogEntry {
        path: dummy_dir.join("crash-reports/crash-test.txt"),
        kind: CrashLogKind::CrashReport,
        lines: prepared.lines().map(|s| s.to_string()).collect(),
        text: prepared,
    });
    analyze_crash("test-inst", dummy_dir, &log_set)
}

fn analyze_hs_err(text: &str) -> mclr_crash::types::CrashResult {
    let dummy_dir = Path::new("dummy");
    let mut log_set = CrashLogSet::default();
    let prepared = prepare_text(text);
    log_set.all = prepared.clone();
    log_set.hs_err = Some(CollectedLogEntry {
        path: dummy_dir.join("hs_err_pid1234.log"),
        kind: CrashLogKind::HsErr,
        lines: prepared.lines().map(|s| s.to_string()).collect(),
        text: prepared,
    });
    analyze_crash("test-inst", dummy_dir, &log_set)
}

#[test]
fn test_no_analyzable_file() {
    let dummy_dir = Path::new("dummy");
    let log_set = CrashLogSet::default();
    let result = analyze_crash("test-inst", dummy_dir, &log_set);
    assert_eq!(result.causes.len(), 1);
    assert_eq!(result.causes[0].code, "noAnalyzableFile");
}

#[test]
fn test_out_of_memory() {
    let log = "[12:00:00] [main/ERROR]: java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.Minecraft.run(Minecraft.java:100)";
    let res = analyze_text(log);
    assert!(res.causes.iter().any(|c| c.code == "outOfMemory"));
}

#[test]
fn test_x86_java_memory_limit() {
    let log = "Error occurred during initialization of VM\nCould not reserve enough space for 1048576KB object heap";
    let res = analyze_text(log);
    assert!(res.causes.iter().any(|c| c.code == "x86JavaMemoryLimit"));
}

#[test]
fn test_driver_access_violations() {
    let intel = "# A fatal error has been detected by the Java Runtime Environment:\n# EXCEPTION_ACCESS_VIOLATION (0xc0000005)\n# C  [ig7icd64.dll+0x1234]";
    let res_intel = analyze_hs_err(intel);
    assert!(
        res_intel
            .causes
            .iter()
            .any(|c| c.code == "intelDriverAccessViolation")
    );

    let amd = "# EXCEPTION_ACCESS_VIOLATION\n# C  [atio6axx.dll+0x5678]";
    let res_amd = analyze_hs_err(amd);
    assert!(
        res_amd
            .causes
            .iter()
            .any(|c| c.code == "amdDriverAccessViolation")
    );

    let nvidia = "# EXCEPTION_ACCESS_VIOLATION\n# C  [nvoglv64.dll+0x9abc]";
    let res_nvidia = analyze_hs_err(nvidia);
    assert!(
        res_nvidia
            .causes
            .iter()
            .any(|c| c.code == "nvidiaDriverAccessViolation")
    );
}

#[test]
fn test_pixel_format_and_opengl() {
    let pf = "[main/ERROR]: org.lwjgl.LWJGLException: Couldn't set pixel format";
    assert!(
        analyze_text(pf)
            .causes
            .iter()
            .any(|c| c.code == "pixelFormatNotSupported")
    );

    let og = "[main/ERROR]: The driver does not appear to support OpenGL";
    assert!(
        analyze_text(og)
            .causes
            .iter()
            .any(|c| c.code == "unsupportedOpenGl")
    );
}

#[test]
fn test_openj9_and_jdk_and_java_incompat() {
    let j9 = "Open J9 is not supported by Minecraft Forge.";
    assert!(
        analyze_text(j9)
            .causes
            .iter()
            .any(|c| c.code == "usingOpenJ9")
    );

    let jdk = "java.lang.ClassCastException: java.base/jdk.internal.loader.ClassLoaders";
    assert!(
        analyze_text(jdk)
            .causes
            .iter()
            .any(|c| c.code == "usingJdk")
    );

    let too_new =
        "java.lang.NoSuchFieldException: ucp\n\tat java.base/java.lang.Class.getDeclaredField";
    assert!(
        analyze_text(too_new)
            .causes
            .iter()
            .any(|c| c.code == "javaTooNew")
    );

    let incompat =
        "java.lang.UnsupportedClassVersionError: Unsupported class file major version 61";
    assert!(
        analyze_text(incompat)
            .causes
            .iter()
            .any(|c| c.code == "javaIncompatible")
    );

    let java11 =
        "has been compiled by a more recent version of the Java Runtime (class file version 55.0)";
    assert!(
        analyze_text(java11)
            .causes
            .iter()
            .any(|c| c.code == "modRequiresJava11")
    );
}

#[test]
fn test_forge_and_loader_issues() {
    let extracted = "Extracted mod jars found, loading will NOT continue";
    assert!(
        analyze_text(extracted)
            .causes
            .iter()
            .any(|c| c.code == "extractedModFile")
    );

    let mixin_boot = "java.lang.ClassNotFoundException: org.spongepowered.asm.launch.MixinTweaker";
    assert!(
        analyze_text(mixin_boot)
            .causes
            .iter()
            .any(|c| c.code == "missingMixinBootstrap")
    );

    let mod_name = "Invalid module name: '' is not a Java identifier";
    assert!(
        analyze_text(mod_name)
            .causes
            .iter()
            .any(|c| c.code == "invalidModFileName")
    );

    let multi_forge =
        "Found multiple arguments for option fml.forgeVersion, but you asked for only one";
    assert!(
        analyze_text(multi_forge)
            .causes
            .iter()
            .any(|c| c.code == "multipleForgeInInstanceJson")
    );

    let shaders_opt = "java.lang.RuntimeException: Shaders Mod detected. Please remove it, OptiFine has built-in support for shaders.";
    assert!(
        analyze_text(shaders_opt)
            .causes
            .iter()
            .any(|c| c.code == "shadersModWithOptiFine")
    );

    let inc_forge = "Cannot find launch target fmlclient, unable to launch";
    assert!(
        analyze_text(inc_forge)
            .causes
            .iter()
            .any(|c| c.code == "incompleteForgeInstallation")
    );

    let old_forge = "java.lang.NoSuchMethodError: sun.security.util.ManifestEntryVerifier";
    assert!(
        analyze_text(old_forge)
            .causes
            .iter()
            .any(|c| c.code == "oldForgeNewJavaIncompatible")
    );

    let forge_err = "An exception was thrown, the game will display an error screen and halt.\nnet.minecraftforge.fml.common.LoaderException: crash\n\tat net.minecraftforge";
    let res = analyze_text(forge_err);
    assert!(res.causes.iter().any(|c| c.code == "forgeError"));
    assert!(res.causes.iter().any(|c| c.params.contains_key("detail")));
}

#[test]
fn test_manual_debug_crash() {
    let crash = "---- Minecraft Crash Report ----\n// Manually triggered debug crash\nTime: 2026-10-24\nDescription: Manually triggered debug crash";
    let res = analyze_crash_report(crash);
    assert!(res.causes.iter().any(|c| c.code == "manualDebugCrash"));
}

#[test]
fn test_duplicate_and_incompatible_mods() {
    let dup = "Found duplicate mods:\n\tMod ID: 'jei' from mod files: [jei-1.20.1-1.0.jar, jei-1.20.1-2.0.jar]";
    let res_dup = analyze_text(dup);
    assert!(res_dup.causes.iter().any(|c| c.code == "duplicateMods"));

    let incomp = "Incompatible mods found!\n\tnet.fabricmc.loader.impl.FormattedException: Mod A is incompatible with Mod B\n\tat net.fabricmc";
    let res_incomp = analyze_text(incomp);
    assert!(
        res_incomp
            .causes
            .iter()
            .any(|c| c.code == "incompatibleMods")
    );
}

#[test]
fn test_fabric_solution_and_error() {
    let sol = "A potential solution has been determined:\n\t - Install fabric-api 0.90.0+\n\t - Remove conflicting mod C";
    let res_sol = analyze_text(sol);
    let finding = res_sol
        .causes
        .iter()
        .find(|c| c.code == "fabricSolutionProvided")
        .unwrap();
    assert!(
        finding
            .params
            .get("solution")
            .unwrap()
            .contains("Install fabric-api")
    );

    let fab_err = "net.fabricmc.loader.impl.FormattedException: Fabric Loader has encountered an error\n\tat net.fabricmc.loader";
    assert!(
        analyze_text(fab_err)
            .causes
            .iter()
            .any(|c| c.code == "fabricError")
    );
}

#[test]
fn test_night_config_and_too_many_mods() {
    let night = "com.electronwill.nightconfig.core.io.ParsingException: Not enough data available";
    assert!(
        analyze_text(night)
            .causes
            .iter()
            .any(|c| c.code == "nightConfigBug")
    );

    let ids = "java.lang.RuntimeException: maximum id range exceeded for block registry";
    assert!(
        analyze_text(ids)
            .causes
            .iter()
            .any(|c| c.code == "tooManyModsIdLimit")
    );
}

#[test]
fn test_missing_dependencies() {
    let dep = "Missing or unsupported mandatory dependencies:\n\tMod 'create' requires 'flywheel'\n\tat net.minecraftforge";
    let res = analyze_text(dep);
    assert!(
        res.causes
            .iter()
            .any(|c| c.code == "missingDependencyOrWrongMcVersion")
    );
    assert!(res.causes.iter().any(|c| c.params.contains_key("detail")));
}

#[test]
fn test_optifine_conflicts() {
    let of = "TRANSFORMER/net.optifine/net.optifine.reflect.Reflector.<clinit>(Reflector.java:123)";
    assert!(
        analyze_text(of)
            .causes
            .iter()
            .any(|c| c.code == "optiFineForgeIncompatible")
    );

    let of_world = "java.lang.NoSuchMethodError: net.minecraft.world.server.ChunkManager$ProxyTicketManager.shouldForceTicks(J)Z\nOptiFine 1.16.5";
    assert!(
        analyze_text(of_world)
            .causes
            .iter()
            .any(|c| c.code == "optiFineWorldLoadCrash")
    );
}

#[test]
fn test_block_and_entity_crashes() {
    let block_log = "---- Minecraft Crash Report ----\nBlock being ticked\n\tBlock: Block{minecraft:furnace}\n\tBlock location: World: (-120,64,305)";
    let res_block = analyze_crash_report(block_log);
    let block_finding = res_block
        .causes
        .iter()
        .find(|c| c.code == "specificBlockCrash")
        .unwrap();
    assert!(
        block_finding
            .params
            .get("block")
            .unwrap()
            .contains("minecraft:furnace")
    );

    let entity_log = "---- Minecraft Crash Report ----\nEntity being ticked\n\tEntity Type: minecraft:creeper (net.minecraft.world.entity.monster.Creeper)\n\tEntity's Exact location: 12.5, 64.0, -88.3";
    let res_entity = analyze_crash_report(entity_log);
    let entity_finding = res_entity
        .causes
        .iter()
        .find(|c| c.code == "specificEntityCrash")
        .unwrap();
    assert!(
        entity_finding
            .params
            .get("entity")
            .unwrap()
            .contains("minecraft:creeper")
    );
}

#[test]
fn test_resource_pack_and_mixin_and_config() {
    let rp = "Maybe try a lower resolution resourcepack?\nStitcherException: Unable to fit: textures/block/stone.png";
    assert!(
        analyze_text(rp)
            .causes
            .iter()
            .any(|c| c.code == "resourcePackTooLarge")
    );

    let mixin = "org.spongepowered.asm.mixin.transformer.throwables.MixinApplyError: Mixin apply failed for mod create failed injection";
    let res_mixin = analyze_text(mixin);
    assert!(res_mixin.causes.iter().any(|c| c.code == "modMixinFailed"));

    let config = "Failed loading config file create-common.toml of type COMMON for modid create";
    let res_config = analyze_text(config);
    let cfg_finding = res_config
        .causes
        .iter()
        .find(|c| c.code == "modConfigCrash")
        .unwrap();
    assert_eq!(
        cfg_finding.params.get("config").unwrap(),
        "create-common.toml"
    );
}

#[test]
fn test_mod_init_and_loader_error_and_confirmed() {
    let init =
        "Failed to create mod instance. ModId ironchest for net.cpw.mods.ironchest.IronChest";
    let res_init = analyze_text(init);
    assert!(
        res_init
            .causes
            .iter()
            .any(|c| c.code == "modInitializationFailed")
    );

    let res_err = "Mod resolution failed\nCould not resolve mod dependencies";
    assert!(
        analyze_text(res_err)
            .causes
            .iter()
            .any(|c| c.code == "modLoaderError")
    );

    let confirmed = "Caught exception from Waystones (waystones)";
    let res_conf = analyze_text(confirmed);
    assert!(
        res_conf
            .causes
            .iter()
            .any(|c| c.code == "confirmedModCrash")
    );
}

#[test]
fn test_suspected_mod() {
    let suspected_report = "---- Minecraft Crash Report ----\nSuspected Mod: Waystones (waystones)\nStacktrace:\n\tat net.blay09.mods.waystones.Waystones.init";
    let res = analyze_crash_report(suspected_report);
    assert!(res.causes.iter().any(|c| c.code == "suspectedModCrash"));
}

#[test]
fn test_stack_analysis_keywords() {
    let stack_log = "[12:00:00] [main/FATAL] [neoforge]: Unhandled exception in game loop\n\tat com.acme.supermagic.spells.CastSpell.execute(CastSpell.java:42)\n\tat net.minecraft.client.Minecraft.run(Minecraft.java:200)";
    let res = analyze_text(stack_log);
    let stack_cause = res
        .causes
        .iter()
        .find(|c| c.code == "stackKeywordFound" || c.code == "stackModNameFound");
    assert!(
        stack_cause.is_some(),
        "Expected stack keyword finding in Neoforge stack"
    );
}

#[test]
fn test_secondary_causes() {
    let opengl = "OpenGL Error: 1282 (Invalid operation)\n\tat net.minecraft.client.renderer";
    assert!(
        analyze_text(opengl)
            .causes
            .iter()
            .any(|c| c.code == "openGl1282")
    );

    let signer =
        "signer information does not match signer information of other classes in the same package";
    assert!(
        analyze_text(signer)
            .causes
            .iter()
            .any(|c| c.code == "fileOrContentValidationFailed")
    );

    let short = "Exit code 1";
    let res_short = analyze_text(short);
    assert!(res_short.causes.iter().any(|c| c.code == "veryShortOutput"));
}

#[test]
fn test_preparer_sanitization() {
    let ansi_text = "\x1b[31;1mError:\x1b[0m Something failed";
    assert_eq!(strip_ansi(ansi_text), "Error: Something failed");

    let crlf = "Line 1\r\nLine 2\rLine 3\n";
    assert_eq!(normalize_crlf(crlf), "Line 1\nLine 2\nLine 3\n");

    let sensitive = "java -jar game.jar --accessToken eyJhbGciOiJIUzI1NiJ9.abcd.efgh --username Steve C:\\Users\\Administrator\\AppData\\Roaming\\.minecraft Bearer eyJtoken123456";
    let masked = mask_sensitive(sensitive);
    assert!(!masked.contains("eyJhbGciOiJIUzI1NiJ9"));
    assert!(masked.contains("--accessToken ***"));
    assert!(masked.contains("Bearer ***"));
    assert!(!masked.contains("Administrator"));
    assert!(masked.contains("C:\\Users\\***\\AppData"));
}
