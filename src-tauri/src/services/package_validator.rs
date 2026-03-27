use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use semver::Version;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::{
    app_config::TARGET_NAME,
    domain::{AddonManifest, CatalogAddon},
    error::InstallerError,
};

pub struct PackageValidator;

impl PackageValidator {
    pub fn validate_manifest(
        catalog_addon: &CatalogAddon,
        manifest: &AddonManifest,
    ) -> Result<(), InstallerError> {
        if manifest.addon_id != catalog_addon.addon_id {
            return Err(InstallerError::validation(
                "manifest_addon_id",
                format!(
                    "The manifest addonId '{}' does not match the catalog addon '{}'.",
                    manifest.addon_id, catalog_addon.addon_id
                ),
            ));
        }

        if !manifest
            .target_support
            .iter()
            .any(|target| target == TARGET_NAME)
        {
            return Err(InstallerError::validation(
                "manifest_target",
                "This addon does not support the selected target.",
            ));
        }

        Self::validate_folder_names(&manifest.folders)?;
        if normalize_folder_set(&manifest.folders) != normalize_folder_set(&catalog_addon.folders) {
            return Err(InstallerError::validation(
                "manifest_folders",
                "The manifest folders do not match the catalog folders.",
            ));
        }

        Self::validate_semver(
            &manifest.version,
            "The addon manifest version is not valid semver.",
        )?;
        Self::validate_semver(
            &manifest.min_installer_version,
            "The manifest minInstallerVersion is not valid semver.",
        )?;
        Self::validate_minimum_installer_version(&manifest.min_installer_version)?;

        if !Self::asset_name_matches(
            &catalog_addon.asset_name_pattern,
            &manifest.version,
            &manifest.asset_name,
        ) {
            return Err(InstallerError::validation(
                "manifest_asset_name",
                format!(
                    "The manifest asset '{}' does not match '{}'.",
                    manifest.asset_name, catalog_addon.asset_name_pattern
                ),
            ));
        }

        Ok(())
    }

    pub fn validate_minimum_installer_version(required: &str) -> Result<(), InstallerError> {
        let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|err| {
            InstallerError::validation_with_details(
                "installer_version",
                "The current installer version is not valid semver.",
                err,
            )
        })?;
        let minimum = Version::parse(required).map_err(|err| {
            InstallerError::validation_with_details(
                "installer_version",
                "The required installer version is not valid semver.",
                err,
            )
        })?;

        if current < minimum {
            return Err(InstallerError::validation(
                "installer_too_old",
                "A newer installer version is required for this addon.",
            ));
        }

        Ok(())
    }

    pub fn validate_semver(value: &str, message: &str) -> Result<(), InstallerError> {
        Version::parse(value).map_err(|err| {
            InstallerError::validation_with_details("semver", message.to_string(), err)
        })?;
        Ok(())
    }

    pub fn asset_name_matches(pattern: &str, version: &str, asset_name: &str) -> bool {
        pattern.replace("{version}", version) == asset_name
    }

    pub fn validate_folder_names(folders: &[String]) -> Result<(), InstallerError> {
        if folders.is_empty() {
            return Err(InstallerError::validation(
                "folder_names",
                "At least one managed addon folder is required.",
            ));
        }

        let mut unique = BTreeSet::new();
        for folder in folders {
            let trimmed = folder.trim();
            if trimmed.is_empty()
                || trimmed.contains('/')
                || trimmed.contains('\\')
                || trimmed.contains(':')
                || trimmed == "."
                || trimmed == ".."
            {
                return Err(InstallerError::validation(
                    "folder_names",
                    format!("'{}' is not a valid top-level addon folder name.", folder),
                ));
            }

            if !unique.insert(trimmed.to_string()) {
                return Err(InstallerError::validation(
                    "folder_names",
                    format!("'{}' appears more than once.", folder),
                ));
            }
        }

        Ok(())
    }

    pub fn verify_checksum(zip_path: &Path, expected_sha256: &str) -> Result<(), InstallerError> {
        let mut file = File::open(zip_path).map_err(|err| {
            InstallerError::io(
                "checksum",
                format!("Could not open '{}'.", zip_path.display()),
                err,
            )
        })?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer).map_err(|err| {
                InstallerError::io(
                    "checksum",
                    format!("Could not read '{}'.", zip_path.display()),
                    err,
                )
            })?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        let actual = hex::encode(hasher.finalize());
        if actual != expected_sha256.to_lowercase() {
            return Err(InstallerError::validation(
                "checksum",
                "The release checksum did not match the downloaded zip.",
            ));
        }
        Ok(())
    }

    pub fn validate_and_extract(
        zip_path: &Path,
        expected_folders: &[String],
        stage_dir: &Path,
    ) -> Result<(), InstallerError> {
        if stage_dir.exists() {
            fs::remove_dir_all(stage_dir).map_err(|err| {
                InstallerError::io(
                    "zip_extract",
                    format!("Could not reset '{}'.", stage_dir.display()),
                    err,
                )
            })?;
        }
        fs::create_dir_all(stage_dir).map_err(|err| {
            InstallerError::io(
                "zip_extract",
                format!("Could not create '{}'.", stage_dir.display()),
                err,
            )
        })?;

        let zip_file = File::open(zip_path).map_err(|err| {
            InstallerError::io(
                "zip_extract",
                format!("Could not open '{}'.", zip_path.display()),
                err,
            )
        })?;
        let mut archive = ZipArchive::new(zip_file).map_err(|err| {
            InstallerError::validation_with_details(
                "zip_extract",
                "Downloaded release zip is malformed.",
                err,
            )
        })?;

        let expected = normalize_folder_set(expected_folders);
        let mut found = BTreeSet::new();

        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|err| {
                InstallerError::validation_with_details(
                    "zip_extract",
                    "Downloaded release zip is malformed.",
                    err,
                )
            })?;

            let entry_name = entry.name();
            let (components, is_dir) = normalize_zip_components(entry_name)?;
            if components.is_empty() {
                continue;
            }

            if components.len() == 1 && !is_dir {
                return Err(InstallerError::validation(
                    "zip_structure",
                    "Downloaded release zip contains files at the zip root.",
                ));
            }

            let top_level = components[0];
            if !expected.contains(top_level) {
                return Err(InstallerError::validation(
                    "zip_structure",
                    "Downloaded release zip is malformed.",
                ));
            }
            if !found.contains(top_level) {
                found.insert(top_level.to_string());
            }

            let destination = join_components(stage_dir, &components);
            if !destination.starts_with(stage_dir) {
                return Err(InstallerError::validation(
                    "zip_traversal",
                    "The release package attempted to write outside the staging directory.",
                ));
            }

            if is_dir {
                fs::create_dir_all(&destination).map_err(|err| {
                    InstallerError::io(
                        "zip_extract",
                        format!("Could not create '{}'.", destination.display()),
                        err,
                    )
                })?;
                continue;
            }

            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    InstallerError::io(
                        "zip_extract",
                        format!("Could not create '{}'.", parent.display()),
                        err,
                    )
                })?;
            }

            let mut output = File::create(&destination).map_err(|err| {
                InstallerError::io(
                    "zip_extract",
                    format!("Could not create '{}'.", destination.display()),
                    err,
                )
            })?;
            io::copy(&mut entry, &mut output).map_err(|err| {
                InstallerError::io(
                    "zip_extract",
                    format!("Could not extract '{}'.", destination.display()),
                    err,
                )
            })?;
        }

        if found != expected {
            return Err(InstallerError::validation(
                "zip_structure",
                "Downloaded release zip is missing one or more declared addon folders.",
            ));
        }

        Ok(())
    }
}

fn normalize_folder_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_string())
        .collect()
}

fn normalize_zip_components(raw: &str) -> Result<(Vec<&str>, bool), InstallerError> {
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(InstallerError::validation(
            "zip_traversal",
            "The release package attempted to write outside the addon directory.",
        ));
    }
    let is_dir = raw.ends_with('/') || raw.ends_with('\\');
    let trimmed = raw.trim_end_matches(|c| c == '/' || c == '\\');
    if trimmed.is_empty() {
        return Ok((Vec::new(), true));
    }

    let mut components = Vec::new();
    for component in trimmed.split(|c| c == '/' || c == '\\') {
        if component.is_empty() || component == "." || component == ".." || component.contains(':')
        {
            return Err(InstallerError::validation(
                "zip_traversal",
                "The release package attempted to write outside the addon directory.",
            ));
        }
        components.push(component);
    }

    Ok((components, is_dir))
}

fn join_components<S: AsRef<str>>(root: &Path, components: &[S]) -> PathBuf {
    let mut destination = root.to_path_buf();
    for component in components {
        destination.push(component.as_ref());
    }
    destination
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;
    use uuid::Uuid;
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    use super::PackageValidator;

    fn create_zip(entries: &[(&str, &[u8])]) -> std::path::PathBuf {
        let zip_path = std::env::temp_dir().join(format!("ascensionup-{}.zip", Uuid::new_v4()));
        let file = std::fs::File::create(&zip_path).expect("zip file");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        for (name, contents) in entries {
            writer.start_file(name, options).expect("start file");
            writer.write_all(contents).expect("write zip contents");
        }

        writer.finish().expect("finish zip");
        zip_path
    }

    #[test]
    fn extracts_valid_single_folder_zip() {
        let zip_path = create_zip(&[
            ("MyAddon/MyAddon.toc", b"## Interface"),
            ("MyAddon/main.lua", b"print(1)"),
        ]);
        let temp = tempdir().expect("tempdir");
        let stage = temp.path().join("stage");

        let result =
            PackageValidator::validate_and_extract(&zip_path, &[String::from("MyAddon")], &stage);

        assert!(result.is_ok());
        assert!(stage.join("MyAddon").join("main.lua").exists());
    }

    #[test]
    fn rejects_extra_nesting() {
        let zip_path = create_zip(&[("release/MyAddon/main.lua", b"print(1)")]);
        let temp = tempdir().expect("tempdir");
        let stage = temp.path().join("stage");

        let result =
            PackageValidator::validate_and_extract(&zip_path, &[String::from("MyAddon")], &stage);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        let zip_path = create_zip(&[("MyAddon/../../evil.lua", b"print(1)")]);
        let temp = tempdir().expect("tempdir");
        let stage = temp.path().join("stage");

        let result =
            PackageValidator::validate_and_extract(&zip_path, &[String::from("MyAddon")], &stage);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_declared_folder() {
        let zip_path = create_zip(&[("MyAddon/main.lua", b"print(1)")]);
        let temp = tempdir().expect("tempdir");
        let stage = temp.path().join("stage");

        let result = PackageValidator::validate_and_extract(
            &zip_path,
            &[String::from("MyAddon"), String::from("MyAddon_Lib")],
            &stage,
        );

        assert!(result.is_err());
    }
}
