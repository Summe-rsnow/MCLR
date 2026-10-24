# PCL-CE Reference Map

Read-only reference repository at `../PCL-CE/`. Re-implement behaviors cleanly in Rust/Vue; NEVER copy code or assets from `Plain Craft Launcher 2/`.

| Milestone | Feature / Crate | PCL-CE Reference Files |
|---|---|---|
| v0.0.2 | `mclr-core` (Config, Paths, Secrets, Tasks) | `PCL.Core/App/{Config.cs,Paths.cs,Secrets.cs,Tasks/*,Configuration/*}` |
| v0.0.3 | `mclr-net` (HTTP, Mirrors, Racing, Downloader) | `Plain Craft Launcher 2/Modules/Network/**`, `Plain Craft Launcher 2/Modules/Minecraft/ModDownload.cs` (2180–2380), `PCL.Core/Network` |
| v0.0.3 | `mclr-game` (Version, Instance, Java) | `PCL.Core/Minecraft/{GameCore.cs,McVersionClassifier.cs,Java/**}`, `Plain Craft Launcher 2/Modules/Minecraft/{McInstance*.cs,ModInstanceList.cs,ModFolder.cs,ModJava.cs,ModLibrary.cs}` |
| v0.0.4 | `mclr-auth` (Profiles, MS, Yggdrasil, Skin) | `PCL.Core/Minecraft/Profile/**`, `PCL.Core/Minecraft/IdentityModel/**`, `PCL.Core/Minecraft/Yggdrasil/ApiLocation.cs`, `Plain Craft Launcher 2/Pages/PageLaunch/*`, `Plain Craft Launcher 2/Modules/Minecraft/ModSkin.cs` |
| v0.0.4 | `mclr-game::install` (Vanilla Install & Repair) | `Plain Craft Launcher 2/Modules/Minecraft/ModDownload.cs`, `Plain Craft Launcher 2/Pages/PageDownload/{PageDownloadInstall,PageDownloadClient}`, `Plain Craft Launcher 2/Modules/Minecraft/ModLibrary.cs` |
| v0.0.5 | `mclr-game::launch` (Launch Pipeline & Watcher) | `Plain Craft Launcher 2/Modules/Minecraft/{ModLaunch.cs,ModWatcher.cs,McLogFilter.cs}` |
| v0.0.6 | `mclr-loaders` (Fabric, Forge, NeoForge, OptiFine, etc.) | `Plain Craft Launcher 2/Pages/PageDownload/ModDownloadLib.cs`, `Plain Craft Launcher 2/Pages/PageDownload/PageDownload*{Forge,NeoForge,Fabric,…}.xaml` |
| v0.0.7 | `mclr-comp` (Modrinth, CurseForge) | `PCL.Core/Minecraft/ResourceProject/**`, `Plain Craft Launcher 2/Modules/Minecraft/{ModComp.cs,ModCompDependency.cs}`, `Plain Craft Launcher 2/Pages/PageDownload/Comp/**` |
| v0.0.8 | `mclr-comp` (Modpacks, Local Mods, JarInJar, Resolver) | `Plain Craft Launcher 2/Modules/Minecraft/{ModModpack.cs,ModLocalComp.cs,ModJarInJar*.cs}`, `PCL.Core/Minecraft/ResourceProject/ModDependencyResolver.cs` |
| v0.0.9 | `mclr-saves` & Instance Pages | `Plain Craft Launcher 2/Pages/PageInstance/**`, `PCL.Core/Minecraft/Saves/**`, `PCL.Core/Minecraft/ServerAddressResolver.cs`, `PCL.Core/Minecraft/NbtFileHandler.cs` |
| v0.0.10 | `mclr-crash` & Modpack Export | `Plain Craft Launcher 2/Pages/PageInstance/PageInstanceExport*`, `Plain Craft Launcher 2/Modules/Minecraft/CrashAnalysis/**` (`CrashCause.cs`, `Detection/*`) |
| v0.0.11 | Personalisation, Home, Help, Tools | `Plain Craft Launcher 2/Pages/PageSetup/PageSetupUI*`, `Plain Craft Launcher 2/Modules/{ModMusic.cs,ModVideoBack.cs}`, `Plain Craft Launcher 2/Pages/PageHomepage/**`, `Plain Craft Launcher 2/Pages/PageHelpDetail*`, `Plain Craft Launcher 2/Pages/PageTools/PageToolsTest*`, `Plain Craft Launcher 2/Modules/UI/Theme` |
| v0.0.12 | `mclr-update` & Remaining Settings | `Plain Craft Launcher 2/Modules/Updates/**`, `Plain Craft Launcher 2/Pages/PageSetup/PageSetup{About,Update,Feedback,Log,LauncherMisc,LauncherLanguage,GameManage}*` |
