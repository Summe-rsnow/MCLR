# MCLR — Multi-Agent Protocol

## Roles & Ownership

- **Main agent = integrator**: owns scaffolding, `docs/contracts.md`, workspace `Cargo.toml`, `src-tauri/src/commands.rs` skeleton, `PROGRESS.md`, `progress/*.md`, version bump, the check gate, commit + tag. Sub-agents NEVER commit, bump versions, or edit progress files; they report done/blocked in their final message and the main agent records it.
- **Spawning**: per milestone the main agent writes `progress/vX.Y.Z.md` (task table: `ID | slice | owner | status todo/doing/done/blocked | acceptance`), then spawns one `task` sub-agent per independent slice in a single `tasks[]` batch, each with: slice ownership (exact dirs/crates it may edit), contract excerpt, acceptance command, "skip repo-wide gates; run only `cargo test -p <crate>` / `vitest run <path>`; do not run `pnpm check`".
- **Ownership**: a slice edits only its crate/feature directories plus its append-only blocks (settings sub-struct, command block, locale file, `docs/reference-map.md` row). Needing a change elsewhere → message the main agent.
- **Integration**: after all slices return, the main agent runs `pnpm check`, fixes seams, and conducts visual and automated smoke tests. Note the mandatory release gate below before tagging or bumping version.
- **Reference rule**: before implementing a behavior, read the PCL-CE files named in `docs/reference-map.md` (`../PCL-CE/...`); re-implement, never copy code or assets from `Plain Craft Launcher 2/`.
- **PROGRESS.md format**: header (current version, next milestone) + table `Milestone | Title | Status (todo/doing/done) | Commit | Tag | Date`; each `progress/vX.Y.Z.md` holds that milestone's task table and a "Notes/Blockers" list.
- **UI**: use only tokens/classes from `docs/design-language.md`; `lint-ui-contracts` must pass.

## Release Gate Policy

- **Explicit User Confirmation Required**: Never bump version, finalize CHANGELOG/PROGRESS as released, or tag a release until the user explicitly confirms iteration completion.
- **Active Iteration State**: While an iteration is in progress, intermediate code edits, refactorings, and temporary commits are allowed, but version fields (`package.json`, `Cargo.toml`, `tauri.conf.json`) and release milestones in `PROGRESS.md` and `CHANGELOG.md` must remain in their pre-release/active state until the user explicitly approves closing the milestone.
