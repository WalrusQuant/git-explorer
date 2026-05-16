# Release & Site Plan

Plan for shipping `git-explorer` v0.1.0 as a standalone macOS app with a GitHub Pages site (landing + docs). Unsigned for the first release; signing + notarization deferred.

---

## 1. Versioning

Three files must stay in lockstep:

| File | Field |
|---|---|
| `src-tauri/Cargo.toml` | `version = "0.1.0"` |
| `src-tauri/tauri.conf.json` | `version` |
| `package.json` | `version` |

**Convention:** SemVer, tag as `vX.Y.Z` (e.g., `v0.1.0`). No `-alpha`/`-beta` suffix — semver pre-1.0 already signals instability.

**Pre-release checklist** (also `RELEASING.md` later if it grows):
1. `pnpm check && cd src-tauri && cargo test`
2. Bump version in all three files
3. Update `CHANGELOG.md`
4. Commit `chore: release v0.1.0`
5. `git tag v0.1.0 && git push --tags`

---

## 2. Build artifacts

**Target:** macOS, Apple Silicon only.

- `aarch64-apple-darwin`

Intel (`x86_64-apple-darwin`) was dropped from v0.1.0 after the cross-compile failed on `openssl-sys`. Apple Silicon has been the only Mac target since 2020; Intel users can build from source if needed.

**Asset name** (renamed in the release workflow so the download URL stays stable):

- `git-explorer-macos-aarch64.dmg`

The site links to `https://github.com/<user>/git-explorer/releases/latest/download/git-explorer-macos-aarch64.dmg` permanently.

---

## 3. Release workflow

New file: `.github/workflows/release.yml`. Triggered on tag push matching `v*`.

**Job shape:**

```yaml
on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: macos-latest
    env:
      TARGET: aarch64-apple-darwin
      ARCH: aarch64
    steps:
      - checkout
      - pnpm + node setup (same as ci.yml)
      - rust toolchain
      - rust-cache scoped to src-tauri
      - pnpm install --frozen-lockfile
      - pnpm tauri build --target $TARGET
      - rename .dmg to git-explorer-macos-aarch64.dmg
      - upload artifact

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - download artifact
      - softprops/action-gh-release@v2: create draft release, upload .dmg, auto-generate notes
```

**Auto-generated notes:** GitHub's `generate_release_notes: true` works fine for v0.1. Switch to CHANGELOG-extracted notes later if churn warrants.

---

## 4. Unsigned first-launch UX

Gatekeeper will refuse `.dmg` from an unidentified developer. Two ways to handle this; document both:

1. **Right-click → Open** the first time (one-shot bypass)
2. `xattr -d com.apple.quarantine /Applications/Git\ Explorer.app` (terminal one-liner)

Mention prominently in:
- README install section
- Site download page (a yellow callout under the download button)
- Release notes for v0.1.0

---

## 5. GitHub Pages site

**Hosting:** `/docs` folder on `main`, Pages set to "Deploy from branch → main → /docs". No CI needed. CNAME deferred — use the default `<user>.github.io/git-explorer` URL.

**Stack:** Plain HTML + one CSS file. No framework. Reasoning: site is small (5-ish pages), no JS interactivity needed, framework overhead is pure cost at this scale. Revisit if it grows past ~15 pages.

**File layout:**

```
docs/
  index.html             # Landing
  styles.css             # Single stylesheet
  getting-started.html
  keyboard-shortcuts.html
  faq.html
  CNAME                  # only if custom domain later
  assets/
    screenshot-main.png
    screenshot-diff.png
    icon.png
    favicon.ico
```

**Landing (`index.html`) sections:**

1. Header: app name + nav (Docs, GitHub)
2. Hero: tagline, hero screenshot, single download button (links to `/releases/latest/download/git-explorer-macos-aarch64.dmg`)
3. Gatekeeper callout: collapsible "First launch on macOS?" with the right-click trick
4. Feature grid (3–5 cards): folder tree with status dots, hunk-level diff, staging UI, branch switcher, commit history
5. Screenshots (2–3 images)
6. Footer: source link, license (Apache 2.0), docs link

**Docs pages (minimum viable):**

- **Getting Started** — install (download, drag to /Applications, first launch), point root directory at a folder, scan results explained, status dot legend.
- **Keyboard Shortcuts** — currently just the CommitDialog shortcut; expand as more land.
- **FAQ** — why unsigned, why mac only, where config is stored, how to reset config, how to report bugs.

**Visual direction:** match the app's monochrome / status-dot palette. Light mode default, dark mode via `prefers-color-scheme`. No animations beyond a subtle button hover.

---

## 6. README updates (separate from site)

Currently a stub. Bring it up to releasable shape:

- Hero screenshot at top
- One-paragraph description
- Install section: download from Releases + Gatekeeper note
- Feature list (mirror site)
- Status color legend (already in CLAUDE.md — copy out)
- Link to site for full docs
- Development section: clone, `pnpm install`, `pnpm tauri dev`
- License badge

---

## 7. CHANGELOG.md

New file at repo root, [Keep a Changelog](https://keepachangelog.com) format.

```markdown
## [Unreleased]

## [0.1.0] - 2026-05-16

### Added
- Initial release: folder tree scanner, repo status, staging UI,
  hunk-level diff, commit, branch switcher, commit log, fetch/push/pull
  via system git, stash management.
```

Update on every release. Source of truth for release notes if we move off auto-generated.

---

## 8. Out of scope for v0.1

- **Auto-updater** (Tauri updater plugin) — requires signed manifests; defer until signing lands.
- **Apple Developer Program signing + notarization** — $99/yr; revisit before v0.2 or first non-trivial user adoption.
- **Custom domain** (e.g., `gitexplorer.app`) — `github.io` URL is fine for now.
- **Analytics / telemetry** — keep it private and offline.
- **Windows / Linux builds** — macOS-only by design.
- **App icon refresh** — current icon was just added; revisit when there's a brand direction.

---

## 9. Suggested order

1. **CHANGELOG.md + README polish** — does not depend on anything; do first.
2. **`docs/` site skeleton + landing page** — Pages can go live before there's a binary to link to; link initially points at the repo until a release exists.
3. **Release workflow** — tag a `v0.1.0-rc1` first to dry-run the workflow without committing to a real release.
4. **Real `v0.1.0` tag** — only after rc1 produces a clean .dmg that launches.
5. **Docs pages 2 + 3** (keyboard-shortcuts, faq) — flesh out after the landing page is live.

---

## 10. End-to-end verification

- `pnpm check` and `cargo test` green on `main`.
- Push `v0.1.0-rc1`: release workflow completes, both `.dmg` files appear as assets on a draft release.
- Download `git-explorer-macos-aarch64.dmg` on a fresh Mac, right-click → Open, app launches and scans a folder.
- Promote to `v0.1.0`, mark as latest release.
- Visit `https://<user>.github.io/git-explorer/`: hero renders, download button hits the right `.dmg`, docs links work, screenshots load.
