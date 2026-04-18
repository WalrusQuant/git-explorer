# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

- `pnpm tauri dev` — run the desktop app (Vite dev server + Rust backend, HMR enabled)
- `pnpm tauri build` — produce a macOS `.app` bundle in `src-tauri/target/release/bundle/macos/`
- `pnpm check` — TypeScript + Svelte type check (`svelte-check`)
- `pnpm dev` / `pnpm build` — frontend only; usually invoked via `tauri` rather than directly
- `cargo check` / `cargo build` — run from `src-tauri/` for Rust-only checks without launching the app

There is no test suite and no linter configured — `pnpm check` is the only automated verification step.

## Architecture

**Tauri v2 desktop app.** A Svelte 5 frontend in `src/` talks to a Rust backend in `src-tauri/src/` via Tauri IPC (`invoke()`). macOS is the only supported target.

### Rust backend (`src-tauri/src/commands.rs`)

Every Tauri command lives in this one file (~1300 lines). `lib.rs` wires them into `invoke_handler!` — when you add a new command, register it there too. Commands fall into two categories:

- **git2-based** (queries, diffs, index manipulation, stashes): use the `git2` crate directly. Examples: `scan_directory`, `get_repo_status`, `get_commit_log`, `get_file_diff`, `stage_file`, `git_commit`, `list_branches`, `checkout_branch`.
- **subprocess-based** (network + auth-sensitive ops): shell out to the `git` CLI via `run_git_cancellable`. Examples: `git_fetch`, `git_push_system`, `git_pull_system`. This path exists because `git2` can't easily handle credential helpers/2FA/SSH prompts, and because subprocesses can be cancelled.

`AppState.git_op_pid` (an `Arc<Mutex<Option<u32>>>`) tracks the PID of any in-flight cancellable git subprocess. `cancel_git_op` sends `SIGTERM` to it via `libc::kill`. Only one cancellable op can run at a time.

Repo status is computed in `compute_repo_status` with this precedence: `dirty` > `diverged` > `ahead` > `behind` > `clean`. The scanner (`scan_directory`) walks up to depth 10, skips dotfiles, and marks ancestor nodes with `has_repo_descendant` so the frontend can render a sparse tree.

Config lives at `~/.config/git-explorer/config.json` (single field: `root_path`). Managed by `load_config` / `save_config`.

### Svelte 5 frontend (`src/`)

**Runes-only — no Svelte stores, no `writable`/`readable`.** Use `$state`, `$derived`, `$derived.by`, `$effect`, `$props`. `App.svelte` owns the top-level state (config, tree nodes, selection, toasts) and passes it down as props. Components:

- `FolderTree.svelte` — expandable directory tree with colored status dots
- `RepoDetail.svelte` — branch, changes, commit log, remotes, ahead/behind
- `ChangedFiles.svelte` + `DiffViewer.svelte` — staging UI and hunk-level diff
- `CommitDialog.svelte`, `SearchBar.svelte`, `Toast.svelte`

Types in `src/lib/types.ts` mirror the Rust structs in `commands.rs` — when you change a `#[derive(Serialize)]` struct, update the TS type in the same change. Field names use `snake_case` on both sides (the Rust structs don't rename).

Styling is Tailwind CSS v4 via `@tailwindcss/vite` (no `tailwind.config.js` — configured in CSS). Theme toggling (`src/lib/theme.ts`) adds/removes a `light` class on `<html>` and persists to `localStorage`.

### Status color legend

| Color  | Meaning                        |
|--------|--------------------------------|
| Green  | Clean — no changes, up to date |
| Yellow | Dirty — uncommitted changes    |
| Blue   | Ahead of remote                |
| Red    | Behind remote                  |
| Orange | Diverged (both ahead & behind) |
