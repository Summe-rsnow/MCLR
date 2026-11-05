# MCLR — Multi-Agent Protocol

## Roles & Ownership

- **Main agent = integrator**: owns scaffolding, `docs/contracts.md`, workspace `Cargo.toml`, `src-tauri/src/commands.rs` skeleton, `docs/progress.md`, version bump, the check gate, commit + tag. Sub-agents NEVER commit, bump versions, or edit progress files; they report done/blocked in their final message and the main agent records it.
- **Spawning**: per milestone the main agent writes the milestone block in `docs/progress.md` (task table: `ID | slice | owner | status todo/doing/done/blocked | acceptance`), then spawns one `task` sub-agent per independent slice in a single `tasks[]` batch, each with: slice ownership (exact dirs/crates it may edit), contract excerpt, acceptance command, "skip repo-wide gates; run only `cargo test -p <crate>` / `vitest run <path>`; do not run `pnpm check`".
- **Ownership**: a slice edits only its crate/feature directories plus its append-only blocks (settings sub-struct, command block, locale file, architecture row). Needing a change elsewhere → message the main agent.
- **Integration**: after all slices return, the main agent runs `pnpm check`, fixes seams, and conducts visual and automated smoke tests. Note the mandatory release gate below before tagging or bumping version.
- **Reference rule**: `../PCL-CE/` is the behavioural reference (read-only). Locate the feature there, then re-implement it — never copy code, assets or `Plain Craft Launcher 2/` resources. Architecture, IPC and error semantics for the re-implementation live in `docs/architecture.md` and `docs/contracts.md`.
- **Progress format**: `docs/progress.md` holds the header (current version, next milestone), the milestone table `Milestone | Title | Status (todo/doing/done) | Commit | Tag | Date`, and one block per recent milestone with its task table and a "Notes/Blockers" list. There is no `progress/` directory.
- **UI**: use only tokens/classes from `docs/design-language.md`; `lint-ui-contracts` must pass.

## Architecture Invariants for New Work

These are enforced in review and by the gate; violating them is a defect, not a style choice:

- **Long-running work is a task**: installs, downloads, imports/exports, preset applies and updates go through `TaskHandler` + `TaskEngine` (`mclr-core/src/tasks/`, handlers in `src-tauri/src/tasks/handlers/`). Never spawn ad-hoc work from an IPC command and never reintroduce a legacy `TaskCenter` tree.
- **Settings changes bump the schema**: any field added to `Settings` requires a `CURRENT_SCHEMA_VERSION` bump and a migration in `mclr-core/src/settings/mod.rs`.
- **One version ordering**: compare versions through `mclr_core::mcver` (frontend mirror: `src/shared/mcver.ts`). Never re-implement segment parsing locally.
- **Blocking work never runs on the async runtime**: filesystem, archive, hash, NBT and image work inside a command goes through `ipc::run_blocking`.
- **Text is selectable by default**: do not add `select-none` to `body`, `#app`, `AppShell` or a page root; opt out only on controls and window chrome (see `docs/design-language.md` §6c).

## Release Gate Policy

- **Explicit User Confirmation Required**: Never bump version, finalize CHANGELOG/`docs/progress.md` as released, or tag a release until the user explicitly confirms iteration completion.
- **Active Iteration State**: While an iteration is in progress, intermediate code edits, refactorings, and temporary commits are allowed, but version fields (`package.json`, `Cargo.toml`, `tauri.conf.json`) and release milestones in `docs/progress.md` and `CHANGELOG.md` must remain in their pre-release/active state until the user explicitly approves closing the milestone.

### Release Procedure (after confirmation)

1. **Documentation first** — write the `CHANGELOG.md` entry (`## [x.y.z] - YYYY-MM-DD`, Keep a Changelog sections), update `docs/progress.md` (current version, next milestone, milestone row, task table) and any affected `docs/*.md`, plus the `README.md` version badge and feature list.
2. **Bump** — `pnpm bump <x.y.z>` (syncs `package.json`, workspace `Cargo.toml`, `src-tauri/tauri.conf.json`); then `cargo check --workspace` to refresh the workspace versions in `Cargo.lock`, and update any hardcoded version in `src/ipc/mock.ts`.
3. **Gate** — `pnpm check` must print `All check gates passed.` A configured system proxy is fine: `scripts/check.mjs` exempts loopback.
4. **Commit** — `feat: release vX.Y.Z - <highlights>`, body covering each area; stage everything (`git add -A`, `.env` and `dist/` are ignored).
5. **Tag** — annotated tag `vX.Y.Z` with the same highlights.
6. **Record** — follow-up `docs: record vX.Y.Z commit hash in PROGRESS` so the milestone table points at the release commit; leave the worktree clean.
