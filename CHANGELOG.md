# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Folder tree scanner that walks up to depth 10 and identifies git repos by `.git` presence (file or directory, so worktrees and submodules are detected).
- Per-repo status with precedence: dirty > diverged > ahead > behind > no-remote > clean.
- Color-coded status dots in the folder tree.
- Repo detail panel: branch, remotes, ahead/behind, recent commits, commit details with per-file additions/deletions.
- Changed-files view with staged/unstaged sections and hunk-level diff rendering.
- Stage, unstage, discard, and stage-all/unstage-all operations.
- Commit dialog with keyboard shortcuts.
- Branch operations: list (local + remote), checkout (with dirty-tree block), create, delete, merge.
- Stash operations: save, pop, apply, drop, list.
- Network operations via the `git` CLI for credential/2FA/SSH-agent support: fetch, push (auto `--set-upstream` on first push), pull. All cancellable via SIGTERM.
- Light/dark theme toggle persisted to `localStorage`.
- Config persisted to `~/.config/git-explorer/config.json` (single field: `root_path`).
- Rust test suite: 3 unit tests + 18 integration tests against real `git2` repos in temp dirs.
- GitHub Actions CI running `pnpm check`, `cargo check`, and `cargo test` on every push and PR.
- `tracing` for previously-silent error paths; honors `RUST_LOG` (default `warn`).

### Notes
- macOS only. Both Apple Silicon and Intel will be supported in the first release.
- First release will ship unsigned; right-click → Open the first time to bypass Gatekeeper.
