# Git Explorer

A macOS desktop app that shows git repos as a navigable folder tree. Built with Tauri v2, Svelte 5, and the git2 Rust crate.

![Overview — repo status, remotes, recent commits](docs/screenshot.png)
*Overview: repo status, remotes, and recent commits.*

![Changes — staged/unstaged files with inline diff](docs/screenshot-diff.png)
*Changes: staged/unstaged files with an inline diff view.*

## Prerequisites

- **macOS** (primary target)
- **Rust** — install via [rustup](https://rustup.rs/)
- **pnpm** — `npm install -g pnpm`
- **Xcode Command Line Tools** — `xcode-select --install`

## Install Dependencies

```bash
# Frontend dependencies
pnpm install

# Rust dependencies are handled automatically by Cargo
# when you run any tauri command
```

## Development

```bash
pnpm tauri dev
```

This starts the Vite dev server with HMR and compiles the Rust backend. The app window opens automatically.

## Build

```bash
pnpm tauri build
```

Produces a macOS `.app` bundle in `src-tauri/target/release/bundle/macos/`.

## Type Checking

```bash
pnpm check
```

## Configuration

On first launch, the app prompts you to set a root directory path. This is saved to:

```
~/.config/git-explorer/config.json
```

Example config:

```json
{
  "root_path": "/Users/you/Code"
}
```

## Architecture

- **Rust backend** (`src-tauri/src/commands.rs`): All git operations via the `git2` crate
  - `scan_directory` — walks directory tree, identifies git repos, fetches from remotes
  - `get_repo_status` — branch, staged/unstaged/untracked counts, ahead/behind, remote URLs
  - `get_commit_log` — recent commits with hash, message, author, timestamp
  - `load_config` / `save_config` — persist settings to `~/.config/git-explorer/config.json`

- **Svelte 5 frontend** — runes only (`$state`, `$derived`, `$effect`, `$props`), no stores
  - `App.svelte` — layout, state management, Tauri IPC
  - `FolderTree.svelte` — expandable directory tree with status indicators
  - `RepoDetail.svelte` — branch info, changes, commits, remotes, ahead/behind
  - `SearchBar.svelte` — repo name filter

## Status Indicators

| Color   | Meaning                         |
|---------|---------------------------------|
| Green   | Clean — no changes, up to date  |
| Yellow  | Dirty — uncommitted changes     |
| Blue    | Ahead of remote                 |
| Red     | Behind remote                   |
| Orange  | Diverged (both ahead & behind)  |

## License

MIT — see [LICENSE](LICENSE).
