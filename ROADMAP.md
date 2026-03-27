# ROADMAP

This roadmap is constrained to what is already documented and implemented in the repository today.

## Shipped Baseline

- Windows desktop installer shell built with Tauri 2, React, and TypeScript.
- Catalog-driven addon library backed by `catalog.json` and remote catalog fetching with cached fallback.
- Addon release resolution from public GitHub releases plus manifest validation.
- Install, update, and rollback flows backed by the Rust service layer.
- Windows CI that runs frontend tests, frontend build, and Rust tests.
- Tag-driven Windows release packaging for `AscensionAddonInstaller-win-x64.zip` and its SHA-256 file.

## Current Release Checklist

- Populate `catalog.json` with real addon entries before cutting a production release.
- Publish addon repositories that satisfy [`docs/addon-release-spec.md`](docs/addon-release-spec.md).
- Set `ASCENSIONUP_REPO_OWNER`, `ASCENSIONUP_REPO_NAME`, and `ASCENSIONUP_CATALOG_URL` when the production release repo differs from the repository defaults.
- Cut a `v*` git tag to trigger the Windows release workflow after tests and build remain green.

## Scope Guardrails

- Bronzebeard only.
- Public GitHub releases only.
- Stable channel only.
- One saved Ascension profile.
- No writes outside app config/log/cache directories, the confirmed addon directory, and the app backup directory.
