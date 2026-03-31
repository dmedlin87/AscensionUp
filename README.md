# AscensionUp

Windows desktop installer/updater for privately managed Project Ascension addons.

## Documentation

- [ROADMAP.md](ROADMAP.md) for the current release checklist and shipped scope
- [AGENTS.md](AGENTS.md) for repository-specific agent and contributor guidance
- [CLAUDE.md](CLAUDE.md) for the Claude entry point that redirects to `AGENTS.md`
- [docs/addon-release-spec.md](docs/addon-release-spec.md) for managed addon packaging rules

## Scope

- Bronzebeard only
- Public GitHub Releases only
- Stable channel only
- One saved Ascension profile
- Prompt-only installer update flow
- No writes outside:
  - app config/log/cache directories
  - the confirmed addon directory
  - the app backup directory

## Stack

- Tauri 2
- React + TypeScript
- Rust service layer for filesystem, network, validation, and rollback logic

## Local Development

```powershell
npm install
npm run test:run
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

CI runs on Windows with Node 24, `npm ci`, `npm run test:run`, `npm run build`, and `cargo test --manifest-path src-tauri/Cargo.toml`.

## Build-Time Configuration

The app resolves the remote catalog and installer release repo from build-time environment variables.

- `ASCENSIONUP_REPO_OWNER`
- `ASCENSIONUP_REPO_NAME`
- `ASCENSIONUP_CATALOG_URL`

If they are not set, the app falls back to:

- owner: `dmedlin87`
- repo: `AscensionUp`
- catalog URL: `https://raw.githubusercontent.com/dmedlin87/AscensionUp/main/catalog.json`

Set these before cutting a real release if the production repo differs.

## Release Shape

- Portable Windows zip asset: `AscensionAddonInstaller-win-x64.zip`
- Expected runtime: Windows 10/11 with WebView2 available
- Installer self-update behavior: detect the latest GitHub release and open the portable zip when present, otherwise fall back to the release page

## Catalog

The remote catalog lives at the repo root as `catalog.json`. This repository currently ships managed addon entries for `DingTimer`, `QuestBuddy`, `FeedMe`, and `AltsDB`; update the list to match the production addon set before release.

## Automation

For a scheduled repository audit focused on frontend polish and usability, use [docs/daily-ux-ui-agent-prompt.md](docs/daily-ux-ui-agent-prompt.md).

## Addon Packaging

Each addon repo publishes:

- a stable release zip with addon folders at the zip root
- `addon-manifest.json`

See `docs/addon-release-spec.md` for the expected package and manifest rules.
