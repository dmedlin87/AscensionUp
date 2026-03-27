use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use sysinfo::System;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    app_config::TARGET_NAME,
    domain::{CatalogAddon, InstalledAddonState, LocalState},
    error::InstallerError,
    runtime::{clear_directory, AppRuntime},
    services::{
        github_release_service::{compare_versions, ResolvedAddonRelease},
        package_validator::PackageValidator,
    },
};

pub struct AddonInstaller;

impl AddonInstaller {
    pub async fn install_or_update(
        runtime: &AppRuntime,
        addon_id: &str,
        allow_while_game_running: bool,
    ) -> Result<Option<String>, InstallerError> {
        let mut state = runtime.settings_store().load()?;
        let addon_path = configured_addon_path(&state)?;
        ensure_game_not_blocking(runtime, &state, allow_while_game_running)?;

        let catalog_resolution = runtime
            .catalog_service()
            .load_catalog(&runtime.http_client, &runtime.logger, &mut state)
            .await;
        let catalog = catalog_resolution.catalog.ok_or_else(|| {
            InstallerError::validation(
                "catalog_unavailable",
                "The catalog could not be loaded, so installs and updates are unavailable.",
            )
        })?;

        let addon = catalog
            .addons
            .iter()
            .find(|entry| {
                entry.addon_id == addon_id
                    && entry.targets.iter().any(|target| target == TARGET_NAME)
            })
            .cloned()
            .ok_or_else(|| {
                InstallerError::validation(
                    "catalog_missing_addon",
                    "That addon is not available for the selected target.",
                )
            })?;

        let previous_version = state
            .installed_addons
            .get(addon_id)
            .map(|installed| installed.version.clone());
        let release = runtime
            .github_service()
            .fetch_addon_release_metadata(&addon, &runtime.logger)
            .await?;

        Self::install_resolved_release(runtime, &mut state, &addon, &release, &addon_path).await?;

        runtime.settings_store().save(&state)?;

        let notice = match previous_version {
            Some(version) if version != release.manifest.version => Some(format!(
                "Updated {} from {} to {}.",
                addon.display_name, version, release.manifest.version
            )),
            Some(_) => Some(format!(
                "Reinstalled {} {}.",
                addon.display_name, release.manifest.version
            )),
            None => Some(format!(
                "Installed {} {}.",
                addon.display_name, release.manifest.version
            )),
        };

        Ok(notice)
    }

    pub async fn update_all(
        runtime: &AppRuntime,
        allow_while_game_running: bool,
    ) -> Result<Option<String>, InstallerError> {
        let mut state = runtime.settings_store().load()?;
        let addon_path = configured_addon_path(&state)?;
        ensure_game_not_blocking(runtime, &state, allow_while_game_running)?;

        let catalog_resolution = runtime
            .catalog_service()
            .load_catalog(&runtime.http_client, &runtime.logger, &mut state)
            .await;
        let catalog = catalog_resolution.catalog.ok_or_else(|| {
            InstallerError::validation(
                "catalog_unavailable",
                "The catalog could not be loaded, so updates are unavailable.",
            )
        })?;

        let installed_ids: Vec<String> = state.installed_addons.keys().cloned().collect();
        let mut updated = Vec::new();
        let mut failures = Vec::new();

        for addon_id in installed_ids {
            let Some(addon) = catalog
                .addons
                .iter()
                .find(|entry| {
                    entry.addon_id == addon_id
                        && entry.targets.iter().any(|target| target == TARGET_NAME)
                })
                .cloned()
            else {
                failures.push(format!("{addon_id}: missing from catalog"));
                continue;
            };

            let installed_version = state
                .installed_addons
                .get(&addon_id)
                .map(|installed| installed.version.clone())
                .unwrap_or_default();

            match runtime
                .github_service()
                .fetch_addon_release_metadata(&addon, &runtime.logger)
                .await
            {
                Ok(release) => {
                    match compare_versions(&installed_version, &release.manifest.version) {
                        Ok(std::cmp::Ordering::Less) => {
                            if let Err(error) = Self::install_resolved_release(
                                runtime,
                                &mut state,
                                &addon,
                                &release,
                                &addon_path,
                            )
                            .await
                            {
                                failures.push(format!("{}: {}", addon.display_name, error));
                            } else {
                                updated.push(addon.display_name.clone());
                            }
                        }
                        Ok(_) => {}
                        Err(error) => failures.push(format!("{}: {}", addon.display_name, error)),
                    }
                }
                Err(error) => failures.push(format!("{}: {}", addon.display_name, error)),
            }
        }

        runtime.settings_store().save(&state)?;

        let notice = if updated.is_empty() && failures.is_empty() {
            Some("All managed addons are already up to date.".to_string())
        } else if failures.is_empty() {
            Some(format!("Updated {}.", updated.join(", ")))
        } else if updated.is_empty() {
            Some(format!("No addons were updated. {}", failures.join(" | ")))
        } else {
            Some(format!(
                "Updated {}. Some addons could not be updated: {}",
                updated.join(", "),
                failures.join(" | ")
            ))
        };

        Ok(notice)
    }

    pub fn rollback(
        runtime: &AppRuntime,
        addon_id: &str,
        allow_while_game_running: bool,
    ) -> Result<Option<String>, InstallerError> {
        let mut state = runtime.settings_store().load()?;
        let addon_path = configured_addon_path(&state)?;
        ensure_game_not_blocking(runtime, &state, allow_while_game_running)?;

        let installed = state
            .installed_addons
            .get(addon_id)
            .cloned()
            .ok_or_else(|| {
                InstallerError::validation(
                    "rollback_missing",
                    "This addon is not installed by the app.",
                )
            })?;

        let backup_path = installed.backup_path.clone().ok_or_else(|| {
            InstallerError::validation("rollback_missing", "No rollback version is available.")
        })?;
        let backup_version = installed.backup_version.clone().ok_or_else(|| {
            InstallerError::validation("rollback_missing", "No rollback version is available.")
        })?;

        let backup_root = PathBuf::from(&backup_path);
        if !backup_root.is_dir() {
            return Err(InstallerError::validation(
                "rollback_missing",
                "The rollback backup for this addon is missing.",
            ));
        }

        let stage_root = addon_path.join(format!(
            ".ascensionup-rollback-{}-{}",
            addon_id,
            Uuid::new_v4()
        ));
        clear_directory(&stage_root)?;
        for folder in &installed.folders {
            copy_dir_all(&backup_root.join(folder), &stage_root.join(folder))?;
        }

        swap_managed_folders(&addon_path, &installed.folders, &stage_root)?;

        if backup_root.exists() {
            fs::remove_dir_all(&backup_root).map_err(|err| {
                InstallerError::io(
                    "rollback_cleanup",
                    format!("Could not remove '{}'.", backup_root.display()),
                    err,
                )
            })?;
        }

        state.installed_addons.insert(
            addon_id.to_string(),
            InstalledAddonState {
                version: backup_version.clone(),
                folders: installed.folders.clone(),
                installed_at: now_iso(),
                backup_version: None,
                backup_path: None,
                source_repo: installed.source_repo,
                display_name: installed.display_name,
            },
        );
        runtime.settings_store().save(&state)?;

        Ok(Some(format!(
            "Rolled back {} to {}.",
            addon_id, backup_version
        )))
    }

    pub fn uninstall(
        runtime: &AppRuntime,
        addon_id: &str,
        allow_while_game_running: bool,
    ) -> Result<Option<String>, InstallerError> {
        let mut state = runtime.settings_store().load()?;
        let addon_path = configured_addon_path(&state)?;
        ensure_game_not_blocking(runtime, &state, allow_while_game_running)?;

        let installed = state
            .installed_addons
            .get(addon_id)
            .cloned()
            .ok_or_else(|| {
                InstallerError::validation(
                    "uninstall_missing",
                    "This addon is not installed by the app.",
                )
            })?;

        remove_managed_folders(&addon_path, &installed.folders)?;
        cleanup_backup_dir(runtime, addon_id, installed.backup_path.as_deref())?;
        state.installed_addons.remove(addon_id);
        runtime.settings_store().save(&state)?;

        Ok(Some(format!(
            "Uninstalled {}.",
            installed
                .display_name
                .unwrap_or_else(|| addon_id.to_string())
        )))
    }

    pub fn detect_game_running(state: &LocalState) -> bool {
        let Some(game_path) = state.game_path.as_ref() else {
            return false;
        };

        let game_root = PathBuf::from(game_path);
        let saved_executable = state
            .game_executable_path
            .as_ref()
            .map(PathBuf::from)
            .and_then(|path| path.file_name().map(|name| name.to_os_string()));

        let system = System::new_all();

        system.processes().values().any(|process| {
            let path_matches = process.exe().is_some_and(|exe| exe.starts_with(&game_root));
            let name_matches = saved_executable
                .as_ref()
                .is_some_and(|saved_name| process.name() == saved_name);
            path_matches || name_matches
        })
    }

    async fn install_resolved_release(
        runtime: &AppRuntime,
        state: &mut LocalState,
        addon: &CatalogAddon,
        release: &ResolvedAddonRelease,
        addon_path: &Path,
    ) -> Result<(), InstallerError> {
        PackageValidator::validate_manifest(addon, &release.manifest)?;
        ensure_no_folder_overlap(state, &addon.addon_id, &release.manifest.folders)?;

        let download_dir = runtime.paths.cache_dir.join("downloads");
        fs::create_dir_all(&download_dir).map_err(|err| {
            InstallerError::io(
                "install_setup",
                format!("Could not create '{}'.", download_dir.display()),
                err,
            )
        })?;

        let package_path = download_dir.join(format!(
            "{}-{}.zip",
            addon.addon_id, release.manifest.version
        ));
        runtime
            .github_service()
            .download_to_file(&release.asset_download_url, &package_path, &runtime.logger)
            .await?;

        if let Some(checksum) = &release.manifest.sha256 {
            PackageValidator::verify_checksum(&package_path, checksum)?;
        }

        let stage_root = addon_path.join(format!(
            ".ascensionup-stage-{}-{}",
            addon.addon_id,
            Uuid::new_v4()
        ));
        PackageValidator::validate_and_extract(
            &package_path,
            &release.manifest.folders,
            &stage_root,
        )?;

        let backup_root = runtime.paths.backups_dir.join(&addon.addon_id);
        let previous = state.installed_addons.get(&addon.addon_id).cloned();
        let existing_folders = existing_managed_folders(addon_path, &release.manifest.folders);

        if existing_folders.is_empty() {
            if backup_root.exists() {
                fs::remove_dir_all(&backup_root).map_err(|err| {
                    InstallerError::io(
                        "backup_cleanup",
                        format!("Could not remove '{}'.", backup_root.display()),
                        err,
                    )
                })?;
            }
        } else {
            clear_directory(&backup_root)?;
            for folder in &release.manifest.folders {
                let source = addon_path.join(folder);
                if source.exists() {
                    copy_dir_all(&source, &backup_root.join(folder))?;
                }
            }
        }

        swap_managed_folders(addon_path, &release.manifest.folders, &stage_root)?;

        state.installed_addons.insert(
            addon.addon_id.clone(),
            InstalledAddonState {
                version: release.manifest.version.clone(),
                folders: release.manifest.folders.clone(),
                installed_at: now_iso(),
                backup_version: if existing_folders.is_empty() {
                    None
                } else {
                    previous.map(|installed| installed.version)
                },
                backup_path: if existing_folders.is_empty() {
                    None
                } else {
                    Some(backup_root.display().to_string())
                },
                source_repo: format!("{}/{}", addon.owner, addon.repo),
                display_name: Some(addon.display_name.clone()),
            },
        );

        Ok(())
    }
}

fn configured_addon_path(state: &LocalState) -> Result<PathBuf, InstallerError> {
    let addon_path = state
        .addon_path
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| {
            InstallerError::validation(
                "path_missing",
                "Choose and confirm an Ascension addon folder first.",
            )
        })?;

    if !addon_path.is_dir() {
        return Err(InstallerError::validation(
            "path_missing",
            "The saved addon folder is no longer valid. Choose the game folder again.",
        ));
    }

    Ok(addon_path)
}

fn ensure_game_not_blocking(
    runtime: &AppRuntime,
    state: &LocalState,
    allow_while_game_running: bool,
) -> Result<(), InstallerError> {
    if AddonInstaller::detect_game_running(state) && !allow_while_game_running {
        runtime.logger.warn(
            "game_process",
            "Ascension appears to be running and the operation requires confirmation.",
        );
        return Err(InstallerError::validation(
            "game_running",
            "Ascension appears to be running. Close it or confirm that you want to continue.",
        ));
    }

    Ok(())
}

fn ensure_no_folder_overlap(
    state: &LocalState,
    addon_id: &str,
    new_folders: &[String],
) -> Result<(), InstallerError> {
    let new_set: BTreeSet<String> = new_folders.iter().cloned().collect();

    for (installed_id, installed) in &state.installed_addons {
        if installed_id == addon_id {
            continue;
        }

        let overlap: Vec<String> = installed
            .folders
            .iter()
            .filter(|folder| new_set.contains(*folder))
            .cloned()
            .collect();
        if !overlap.is_empty() {
            return Err(InstallerError::validation(
                "folder_overlap",
                format!(
                    "Managed folder ownership overlaps with '{}': {}",
                    installed_id,
                    overlap.join(", ")
                ),
            ));
        }
    }

    Ok(())
}

fn existing_managed_folders(addon_path: &Path, folders: &[String]) -> Vec<String> {
    folders
        .iter()
        .filter(|folder| addon_path.join(folder).exists())
        .cloned()
        .collect()
}

fn remove_managed_folders(addon_path: &Path, folders: &[String]) -> Result<(), InstallerError> {
    for folder in folders {
        let live_folder = addon_path.join(folder);
        if !live_folder.exists() {
            continue;
        }

        fs::remove_dir_all(&live_folder).map_err(|err| {
            InstallerError::io(
                "uninstall_remove",
                "Some files could not be removed because the game may still be using them.",
                err,
            )
        })?;
    }

    Ok(())
}

fn cleanup_backup_dir(
    runtime: &AppRuntime,
    addon_id: &str,
    recorded_backup_path: Option<&str>,
) -> Result<(), InstallerError> {
    let expected_backup_root = runtime.paths.backups_dir.join(addon_id);
    if expected_backup_root.exists() {
        fs::remove_dir_all(&expected_backup_root).map_err(|err| {
            InstallerError::io(
                "uninstall_cleanup",
                format!("Could not remove '{}'.", expected_backup_root.display()),
                err,
            )
        })?;
    }

    if let Some(raw_backup_path) = recorded_backup_path {
        let backup_root = PathBuf::from(raw_backup_path);
        if backup_root != expected_backup_root && backup_root.exists() {
            fs::remove_dir_all(&backup_root).map_err(|err| {
                InstallerError::io(
                    "uninstall_cleanup",
                    format!("Could not remove '{}'.", backup_root.display()),
                    err,
                )
            })?;
        }
    }

    Ok(())
}

fn swap_managed_folders(
    addon_path: &Path,
    folders: &[String],
    incoming_root: &Path,
) -> Result<(), InstallerError> {
    let removed_root = addon_path.join(format!(".ascensionup-replaced-{}", Uuid::new_v4()));
    fs::create_dir_all(&removed_root).map_err(|err| {
        InstallerError::io(
            "swap_folders",
            format!("Could not create '{}'.", removed_root.display()),
            err,
        )
    })?;

    let mut removed = Vec::new();
    let mut moved_in = Vec::new();

    for folder in folders {
        let live_folder = addon_path.join(folder);
        if live_folder.exists() {
            let removed_folder = removed_root.join(folder);
            fs::rename(&live_folder, &removed_folder).map_err(|err| {
                InstallerError::io(
                    "swap_folders",
                    "Some files could not be replaced because the game may still be using them.",
                    err,
                )
            })?;
            removed.push((folder.clone(), removed_folder));
        }
    }

    for folder in folders {
        let staged_folder = incoming_root.join(folder);
        if !staged_folder.exists() {
            continue;
        }

        let live_folder = addon_path.join(folder);
        if let Err(err) = fs::rename(&staged_folder, &live_folder) {
            for moved in &moved_in {
                let _ = fs::rename(addon_path.join(moved), incoming_root.join(moved));
            }
            for (folder_name, removed_folder) in &removed {
                let _ = fs::rename(removed_folder, addon_path.join(folder_name));
            }
            return Err(InstallerError::io(
                "swap_folders",
                "Some files could not be replaced because the game may still be using them.",
                err,
            ));
        }
        moved_in.push(folder.clone());
    }

    if removed_root.exists() {
        fs::remove_dir_all(&removed_root).map_err(|err| {
            InstallerError::io(
                "swap_cleanup",
                format!("Could not remove '{}'.", removed_root.display()),
                err,
            )
        })?;
    }

    if incoming_root.exists() {
        fs::remove_dir_all(incoming_root).map_err(|err| {
            InstallerError::io(
                "swap_cleanup",
                format!("Could not remove '{}'.", incoming_root.display()),
                err,
            )
        })?;
    }

    Ok(())
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), InstallerError> {
    if !source.exists() {
        return Ok(());
    }

    fs::create_dir_all(destination).map_err(|err| {
        InstallerError::io(
            "copy_dir",
            format!("Could not create '{}'.", destination.display()),
            err,
        )
    })?;

    for entry in WalkDir::new(source) {
        let entry = entry.map_err(|err| {
            InstallerError::validation_with_details(
                "copy_dir",
                format!("Could not walk '{}'.", source.display()),
                err,
            )
        })?;
        let relative = entry.path().strip_prefix(source).map_err(|err| {
            InstallerError::validation_with_details(
                "copy_dir",
                format!(
                    "Could not derive a relative path for '{}'.",
                    entry.path().display()
                ),
                err,
            )
        })?;
        let target = destination.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|err| {
                InstallerError::io(
                    "copy_dir",
                    format!("Could not create '{}'.", target.display()),
                    err,
                )
            })?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    InstallerError::io(
                        "copy_dir",
                        format!("Could not create '{}'.", parent.display()),
                        err,
                    )
                })?;
            }
            fs::copy(entry.path(), &target).map_err(|err| {
                InstallerError::io(
                    "copy_dir",
                    format!(
                        "Could not copy '{}' to '{}'.",
                        entry.path().display(),
                        target.display()
                    ),
                    err,
                )
            })?;
        }
    }

    Ok(())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}
