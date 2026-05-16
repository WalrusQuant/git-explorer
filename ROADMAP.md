# Roadmap

Four hardening additions extracted from a survey of a more-mature sibling project. Each ships as its own PR, in the order listed.

---

## 1. Split `commands.rs` by domain

**Why:** `src-tauri/src/commands.rs` is 1300+ lines in a single file. Reviews are slow, merge conflicts are inevitable, and module boundaries are invisible.

**Layout** (under `src-tauri/src/commands/`):

| File | Contents |
|---|---|
| `mod.rs` | Re-exports each submodule's `pub fn` so `lib.rs`'s `tauri::generate_handler!` keeps working unchanged. Holds `AppState`. |
| `types.rs` | All `#[derive(Serialize/Deserialize)]` structs: `RepoStatus`, `TreeNode`, `Commit`, `FileChange`, `DiffLine`, `FileDiff`, `HunkDiff`, `BranchInfo`, `Config`, `CommitDetail`, `CommitFileStat`, `StashEntry`. |
| `helpers.rs` | `is_valid_branch_name`, `delta_status_str`, `make_remote_callbacks`, `make_fetch_options`, `config_path`, `PidGuard`, `run_git_cancellable`. Existing `#[cfg(test)] mod tests` moves here. |
| `status.rs` | `compute_repo_status`, `count_staged`, `count_unstaged`, `compute_remote_info`, `get_repo_status`. |
| `scan.rs` | `scan_directory`. |
| `staging.rs` | `get_changed_files`, `get_file_diff`, `stage_file`, `unstage_file`, `stage_all`, `unstage_all`, `git_commit`, `discard_file`. |
| `branches.rs` | `list_branches`, `checkout_branch`, `create_branch`, `delete_branch`. |
| `commits.rs` | `get_commit_log`, `get_commit_details`. |
| `remote.rs` | `git_fetch`, `git_push_system`, `git_pull_system`, `git_merge_branch`, `cancel_git_op`. |
| `stash.rs` | `git_stash_save`, `git_stash_pop`, `git_stash_apply`, `git_stash_drop`, `git_stash_list`. |
| `config.rs` | `load_config`, `save_config`. |

**Mechanics:** delete the old `commands.rs`, create `commands/` with the files above. Keep cross-module helpers `pub(super)` or `pub(crate)`; nothing leaks outside the `commands` module. `lib.rs`'s handler list stays untouched.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml` — all 21 existing tests pass.
- `pnpm tauri dev` smoke test: app launches, repos load.

---

## 2. CI: concurrency cancellation + timeout

**Why:** Pushing twice to a PR currently runs two parallel jobs and wastes a runner.

**Changes to `.github/workflows/ci.yml`:**

```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

Add `timeout-minutes: 20` to the `check` job. `Swatinem/rust-cache` already has `workspaces: src-tauri -> target` — leave as-is.

**Verification:** push two commits in quick succession; confirm the first run is cancelled in the Actions tab.

---

## 3. CI test isolation via `$HOME` override

**Why:** `load_config` / `save_config` read and write `$HOME/.config/git-explorer/config.json`. Config-touching tests were skipped to avoid polluting the runner. Overriding `$HOME` to a temp dir unblocks them.

**Changes to `.github/workflows/ci.yml`:** add `env: { HOME: ${{ runner.temp }}/test-home }` (and `mkdir -p`) on the `Rust tests` step only — keep the real `$HOME` for `pnpm install` and frontend caches.

**Follow-up (separate PR, out of scope here):** add a `commands::config::tests` module covering `save_config` then `load_config` roundtrip.

---

## 4. `tracing` for the silent error paths

**Why:** Several places swallow `Result`s with `let _ = ...` or `Err(_) => ...`, making field debugging impossible. Replace with structured logging gated by `RUST_LOG`.

**Dependencies (`src-tauri/Cargo.toml`):**

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

**Subscriber init (`src-tauri/src/lib.rs`, top of `run()`):**

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
    )
    .init();
```

**Call sites to convert** (paths assume section 1 has landed):

- `commands/status.rs::compute_remote_info` — `let _ = remote.fetch(...)` → `tracing::warn!("fetch failed for origin: {}", err)`.
- `commands/commits.rs::get_commit_details` — trailing `.unwrap_or(())` on diff iteration → `tracing::warn!`.

Scope is intentionally minimal. **No** `#[tracing::instrument]` on every command in this PR.

**Verification:** `RUST_LOG=git_explorer_lib=warn pnpm tauri dev` against a repo with a broken origin remote should print the warning.

---

## End-to-end verification once all four ship

- `cd src-tauri && cargo test` — 21+ tests pass.
- `pnpm check` — no Svelte/TS errors.
- `pnpm tauri dev` — app launches, repos load, status dots render.
- Broken-remote scenario emits a tracing warning.
- Two rapid pushes cancel the older CI run.

## Out of scope (deferred)

- `anyhow` / `thiserror` — Tauri commands serialize as `Result<T, String>`; defer until error chaining is actually needed.
- Workspace `Cargo.toml` — only useful when splitting into multiple crates.
- Auto-update, code signing, notarization.
- Frontend testing setup.
