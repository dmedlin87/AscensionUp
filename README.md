# Ascension Addon Installer

Windows desktop installer/updater for privately managed Project Ascension addons.

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

## Build-Time Configuration

The app resolves the remote catalog and installer release repo from build-time environment variables.

- `ASCENSIONUP_REPO_OWNER`
- `ASCENSIONUP_REPO_NAME`
- `ASCENSIONUP_CATALOG_URL`

If they are not set, the app falls back to:

- owner: `dmedl`
- repo: `AscensionUp`
- catalog URL: `https://raw.githubusercontent.com/dmedl/AscensionUp/main/catalog.json`

Set these before cutting a real release if the production repo differs.

## Release Shape

- Portable Windows zip asset: `AscensionAddonInstaller-win-x64.zip`
- Expected runtime: Windows 10/11 with WebView2 available
- Installer self-update behavior: detect latest release and open the direct download URL

## Catalog

The remote catalog lives at the repo root as `catalog.json`. The sample file in this repo starts empty on purpose so you can populate real addon entries before releasing.

## Addon Packaging

Each addon repo publishes:

- a stable release zip with addon folders at the zip root
- `addon-manifest.json`

See `docs/addon-release-spec.md` for the expected package and manifest rules.
