# Addon Release Spec

Each managed addon repository must publish a stable GitHub release containing:

- one `addon-manifest.json` asset
- one zip asset whose name matches the catalog entry `assetNamePattern`

## Zip Layout

The zip must contain the real addon folders at the zip root.

Valid:

```text
MyAddon-v1.2.0.zip
  MyAddon/
    MyAddon.toc
    main.lua
```

Also valid:

```text
MyAddon-v1.2.0.zip
  MyAddon/
    MyAddon.toc
  MyAddon_Lib/
    MyAddon_Lib.toc
```

Invalid:

```text
MyAddon-v1.2.0.zip
  release/
    MyAddon/
```

## Manifest

```json
{
  "schemaVersion": 1,
  "addonId": "my-addon",
  "displayName": "My Addon",
  "version": "1.2.0",
  "targetSupport": ["Bronzebeard", "CoA"],
  "folders": ["MyAddon"],
  "assetName": "MyAddon-v1.2.0.zip",
  "sha256": "abc123...",
  "minInstallerVersion": "1.0.0",
  "releaseNotes": "Fixes target frame logic."
}
```

Rules:

- `addonId` must match the installer catalog entry.
- `folders` must exactly match the catalog entry folders.
- `assetName` must exactly match the uploaded zip asset name.
- `targetSupport` must include `Bronzebeard` or `CoA`, depending on the addon build.
- `minInstallerVersion` must be valid semver.

## Catalog Entry

```json
{
  "addonId": "my-addon",
  "displayName": "My Addon",
  "description": "Utility addon for Ascension.",
  "owner": "your-github-user",
  "repo": "my-addon",
  "targets": ["Bronzebeard", "CoA"],
  "folders": ["MyAddon"],
  "manifestStrategy": "release-asset",
  "manifestAssetName": "addon-manifest.json",
  "assetNamePattern": "MyAddon-v{version}.zip",
  "iconUrl": null
}
```

The installer rejects:

- extra zip nesting
- zip path traversal
- root-level files outside the declared addon folders
- manifest folder sets that do not match the catalog
