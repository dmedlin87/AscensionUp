# AGENTS.md

This file is the repository-specific handoff for coding agents and human contributors working in `AscensionUp`.

## Project Snapshot

- Product: Windows desktop installer/updater for privately managed Project Ascension addons.
- Frontend: React 19 + TypeScript + Vite.
- Desktop/runtime shell: Tauri 2.
- Backend/service layer: Rust in `src-tauri/src`.
- Target audience and scope: Bronzebeard and CoA, public GitHub releases only, stable channel only, one saved Ascension profile.

## Source of Truth

When docs disagree, prefer these files in this order:

1. `README.md` for the supported product scope and release shape.
2. `ROADMAP.md` for shipped status and the current release checklist.
3. `docs/addon-release-spec.md` for addon package and manifest requirements.
4. `.github/workflows/ci.yml` and `.github/workflows/release.yml` for verified build, test, and release commands.
5. `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` for runtime/tooling configuration.

## Working Rules

- Keep the app scoped to the documented installer constraints in `README.md`.
- Do not broaden target support beyond `Bronzebeard` and `CoA` unless the product docs and code change together.
- Preserve the write-boundary guarantees documented in `README.md`:
  - app config/log/cache directories
  - the confirmed addon directory
  - the app backup directory
- Treat `catalog.json` and `docs/addon-release-spec.md` as a contract with managed addon repositories.
- Prefer surgical documentation updates over broad rewrites.

## Verification Commands

Run the smallest relevant validation for doc or code changes:

```powershell
npm run test:run
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

For local desktop development:

```powershell
npm run tauri dev
```

## Documentation Expectations

- Update `README.md` when setup, scope, release shape, or operator-facing behavior changes.
- Update `ROADMAP.md` when shipped scope or the pre-release checklist changes.
- Update `docs/addon-release-spec.md` when catalog or addon release contract requirements change.
- Keep `CLAUDE.md` as a lightweight pointer to `@AGENTS.md`.
