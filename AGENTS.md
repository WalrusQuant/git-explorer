# AGENTS.md

Project-specific guidance for ZCode agents working in this repo.

## What this is

**Git Explorer** — a macOS-only desktop app (Tauri v2) that renders git repos as a
navigable folder tree with status dots, diffs, staging, branches, and stash UI.
Svelte 5 frontend talks to a Rust backend over Tauri IPC (`invoke()`).

> **Note:** `CLAUDE.md` exists for deeper architecture notes but is **partly stale** —
> it describes a single `commands.rs` file. The code is now split per-domain under
> `src-tauri/src/commands/` (see below). Trust the filesystem over `CLAUDE.md` on
> structure; this file reflects the current layout.

## Layout

```
src/                      Svelte 5 frontend (runes-only)
  App.svelte              Top-level state, passed down as props
  lib/components/         FolderTree, RepoDetail, ChangedFiles, DiffViewer,
                          CommitDialog, SearchBar, Toast
  lib/types.ts            TS types mirroring Rust structs (keep in sync)
  lib/theme.ts            light/dark toggle (class on <html>, persisted)
src-tauri/src/
  lib.rs                  App entry — run(), plugins, AppState, invoke_handler!
  commands/               Per-domain Tauri commands (see below)
    mod.rs                AppState + module wiring + `pub use *`
    types.rs              Serialize structs shared with frontend
  main.rs                 Tauri bootstrap
src-tauri/tests/          Rust integration tests (integration.rs)
docs/                     Static GitHub Pages landing site (HTML)
.github/workflows/        ci.yml (macos-latest), release.yml
```

## Commands

```bash
pnpm install              # install frontend deps
pnpm tauri dev            # run the app (Vite + Rust, HMR) — primary dev loop
pnpm tauri build          # produce macOS .app bundle
pnpm check                # svelte-check: TS + Svelte type check (frontend verification)
pnpm dev / pnpm build     # frontend only; usually run via tauri

# Rust (run from src-tauri/)
cargo check --all-targets
cargo test                # unit + integration tests (Rust verification)
```

**No frontend tests or linter are configured.** Automated verification = `pnpm check`
+ `cargo test` (this is also what CI runs). CI runs on `macos-latest`.

## Architecture rules (matter when editing)

- **Two command categories** in `src-tauri/src/commands/`:
  - **git2-based** (`status`, `scan`, `staging`, `branches`, `commits`, `stash`,
    `config`, `helpers`): use the `git2` crate directly for local/query ops.
  - **subprocess-based** (`remote.rs` — `git_fetch`, `git_push_system`,
    `git_pull_system`, `git_merge_branch`): shell out to the `git` CLI via the
    cancellable runner. This path exists because `git2` can't handle credential
    helpers / 2FA / SSH prompts, and subprocesses can be cancelled.
- **Register every new command** in the `tauri::generate_handler![...]` macro in
  `src-tauri/src/lib.rs`. A command added to `commands/` but not registered there
  is invisible to the frontend.
- **One cancellable op at a time.** `AppState.git_op_pid`
  (`Arc<Mutex<Option<u32>>>`) tracks the in-flight cancellable git subprocess;
  `cancel_git_op` sends `SIGTERM` via `libc::kill`.
- **Status precedence** (`compute_repo_status`):
  `dirty` > `diverged` > `ahead` > `behind` > `clean`.
- **Config** lives at `~/.config/git-explorer/config.json` (single field:
  `root_path`), managed by `config.rs`.

## Conventions

- **Frontend is Svelte 5 runes-only** — `$state`, `$derived`, `$derived.by`,
  `$effect`, `$props`. **No Svelte stores** (`writable`/`readable`).
- **Type mirroring:** Rust `#[derive(Serialize)]` structs in `commands/types.rs`
  map 1:1 to TS interfaces in `src/lib/types.ts`. Field names are `snake_case`
  on both sides (no serde rename). **Change both in the same commit.**
- **Logging:** `tracing` crate (configured in `lib.rs`, default `warn` level).
  Use `tracing::warn!`/`tracing::error!` for silent-error paths; raise with
  `RUST_LOG` during dev.
- **Styling:** Tailwind CSS v4 via `@tailwindcss/vite` (no `tailwind.config.js` —
  configured in CSS). Theme toggle adds/removes `light` class on `<html>`.
- **Status dot colors:** green=clean, yellow=dirty, blue=ahead, red=behind,
  orange=diverged, gray=no-remote, dark-red=error. Full set defined in
  `RepoStatusIndicator` (`src/lib/types.ts`) and `FolderTree.svelte`'s
  `statusDotColor`.

## Platform / release gotchas

- **macOS is the only supported target** (Apple Silicon releases; Intel must build
  from source). Don't add Windows/Linux-specific code paths.
- Releases from v0.1.1 onward are **ad-hoc signed** (`signingIdentity: "-"` in
  `tauri.conf.json`). Earlier unsigned builds require `xattr -cr` on the bundle.
- See `RELEASE.md` for the release process and `CHANGELOG.md` for version history.
