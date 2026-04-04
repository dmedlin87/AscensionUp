use std::{collections::BTreeSet, path::PathBuf};

use serde::Serialize;
use tauri::State;

use crate::{
    app_config,
    domain::{
        AddonRow, AddonStatus, AppSnapshot, CatalogResolution, CommandEnvelope,
        InstalledAddonState, OperationResult, PathInspection, PathVerification,
        TargetPathState,
    },
    error::InstallerError,
    runtime::AppRuntime,
    services::{
        addon_installer::AddonInstaller,
        github_release_service::{compare_versions, ResolvedAddonRelease},
        package_validator::PackageValidator,
        target_detector::{canonicalize_lossy, display_path, is_addon_directory, TargetDetector},
    },
};

fn ok<T>(data: T) -> CommandEnvelope<T>
where
    T: Serialize,
{
    CommandEnvelope {
        data: Some(data),
        error: None,
    }
}

fn err<T>(error: InstallerError) -> CommandEnvelope<T>
where
    T: Serialize,
{
    CommandEnvelope {
        data: None,
        error: Some(error.payload()),
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn bootstrapApp(
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<AppSnapshot>, String> {
    Ok(match snapshot_for_current_state(runtime.inner()).await {
        Ok(snapshot) => ok(snapshot),
        Err(error) => err(error),
    })
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn inspectGamePath(selected_path: String) -> CommandEnvelope<PathInspection> {
    match TargetDetector::inspect(&PathBuf::from(selected_path)) {
        Ok(inspection) => ok(inspection),
        Err(error) => err(error),
    }
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn confirmGamePath(
    game_path: String,
    addon_path: String,
    game_executable_path: Option<String>,
    selected_target: Option<String>,
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<AppSnapshot>, String> {
    let result = async {
        let inspection = TargetDetector::inspect(&PathBuf::from(
            game_executable_path.clone().unwrap_or_else(|| game_path.clone()),
        ))?;
        let requested_addon_path = canonicalize_lossy(&PathBuf::from(&addon_path));
        let requested_addon_text = display_path(&requested_addon_path);
        let chosen = inspection
            .candidate_addon_paths
            .iter()
            .find(|candidate| candidate.exists && candidate.path == requested_addon_text)
            .cloned()
            .or_else(|| {
                if requested_addon_path.is_dir() && is_addon_directory(&requested_addon_path) {
                    Some(crate::domain::CandidateAddonPath {
                        path: requested_addon_text.clone(),
                        exists: true,
                        label: "Selected AddOn Directory".to_string(),
                    })
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                InstallerError::validation(
                    "confirm_path",
                    "The selected addon directory is not one of the documented Ascension or CoA addon paths.",
                )
            })?;

        let mut state = runtime.settings_store().load()?;
        let current_target = state.selected_target.clone();
        let remembered_game_executable_path = inspection
            .game_executable_path
            .clone()
            .or(game_executable_path.clone());
        let replacing_existing = state
            .game_path
            .as_ref()
            .is_some_and(|saved| saved != &inspection.normalized_game_path);
        state.game_path = Some(inspection.normalized_game_path.clone());
        state.game_executable_path = remembered_game_executable_path.clone();
        state.addon_path = Some(chosen.path.clone());
        state.selected_target = app_config::resolve_target_name(
            selected_target.as_deref().or(Some(current_target.as_str())),
            &[
                Some(inspection.normalized_game_path.as_str()),
                inspection.game_executable_path.as_deref(),
                Some(chosen.path.as_str()),
            ],
        );
        let resolved_target = state.selected_target.clone();
        remember_detected_target_profiles(
            &mut state,
            &inspection,
            &resolved_target,
            remembered_game_executable_path.as_deref(),
            &chosen.path,
        );
        if replacing_existing {
            state.installed_addons.clear();
            runtime.clear_backups()?;
        }
        runtime.settings_store().save(&state)?;

        snapshot_for_current_state(runtime.inner()).await
    }
    .await;

    Ok(match result {
        Ok(snapshot) => ok(snapshot),
        Err(error) => err(error),
    })
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn refreshCatalog(
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<AppSnapshot>, String> {
    Ok(match snapshot_for_current_state(runtime.inner()).await {
        Ok(snapshot) => ok(snapshot),
        Err(error) => err(error),
    })
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn installAddon(
    addon_id: String,
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<OperationResult>, String> {
    let result = async {
        let notice = AddonInstaller::install_or_update(runtime.inner(), &addon_id).await?;
        let snapshot = snapshot_for_current_state(runtime.inner()).await?;
        Ok(OperationResult { snapshot, notice })
    }
    .await;

    Ok(match result {
        Ok(operation) => ok(operation),
        Err(error) => err(error),
    })
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn updateAddon(
    addon_id: String,
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<OperationResult>, String> {
    installAddon(addon_id, runtime).await
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn updateAllAddons(
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<OperationResult>, String> {
    let result = async {
        let notice = AddonInstaller::update_all(runtime.inner()).await?;
        let snapshot = snapshot_for_current_state(runtime.inner()).await?;
        Ok(OperationResult { snapshot, notice })
    }
    .await;

    Ok(match result {
        Ok(operation) => ok(operation),
        Err(error) => err(error),
    })
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn uninstallAddon(
    addon_id: String,
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<OperationResult>, String> {
    let result = async {
        let notice = AddonInstaller::uninstall(runtime.inner(), &addon_id)?;
        let snapshot = snapshot_for_current_state(runtime.inner()).await?;
        Ok(OperationResult { snapshot, notice })
    }
    .await;

    Ok(match result {
        Ok(operation) => ok(operation),
        Err(error) => err(error),
    })
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn rollbackAddon(
    addon_id: String,
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<OperationResult>, String> {
    let result = async {
        let notice = AddonInstaller::rollback(runtime.inner(), &addon_id)?;
        let snapshot = snapshot_for_current_state(runtime.inner()).await?;
        Ok(OperationResult { snapshot, notice })
    }
    .await;

    Ok(match result {
        Ok(operation) => ok(operation),
        Err(error) => err(error),
    })
}

#[allow(non_snake_case)]
#[tauri::command]
pub async fn checkInstallerUpdate(
    runtime: State<'_, AppRuntime>,
) -> Result<CommandEnvelope<crate::domain::InstallerUpdateStatus>, String> {
    Ok(
        match runtime
            .github_service()
            .check_installer_update(&runtime.logger)
            .await
        {
            Ok(status) => ok(status),
            Err(error) => err(error),
        },
    )
}

#[allow(non_snake_case)]
#[tauri::command]
pub fn openLogsFolder(runtime: State<'_, AppRuntime>) -> CommandEnvelope<bool> {
    match runtime.open_logs_folder() {
        Ok(()) => ok(true),
        Err(error) => err(error),
    }
}

async fn snapshot_for_current_state(runtime: &AppRuntime) -> Result<AppSnapshot, InstallerError> {
    let mut state = runtime.settings_store().load()?;
    let catalog_resolution = runtime
        .catalog_service()
        .load_catalog(&runtime.http_client, &runtime.logger, &mut state)
        .await;
    runtime.settings_store().save(&state)?;

    let (path_verification, path_message) = current_path_status(&state)?;
    let needs_setup = state
        .addon_path
        .as_ref()
        .map(PathBuf::from)
        .is_none_or(|path| !path.is_dir());
    let addon_rows = build_addon_rows(runtime, &state, &catalog_resolution).await;

    Ok(AppSnapshot {
        installer_version: env!("CARGO_PKG_VERSION").to_string(),
        selected_target: state.selected_target.clone(),
        game_path: state.game_path.clone(),
        game_executable_path: state.game_executable_path.clone(),
        addon_path: state.addon_path.clone(),
        path_verification,
        path_message,
        needs_setup,
        catalog_status: catalog_resolution.status,
        catalog_message: catalog_resolution.message,
        catalog_url: runtime.catalog_url.clone(),
        last_catalog_refresh_at: state.last_catalog_refresh_at.clone(),
        addon_rows,
        log_directory: runtime.logger.logs_dir().display().to_string(),
        game_running: false,
        installer_release_page_url: app_config::installer_release_page_url(),
    })
}

fn current_path_status(
    state: &crate::domain::LocalState,
) -> Result<(PathVerification, Option<String>), InstallerError> {
    let Some(game_path) = state.game_path.as_ref() else {
        return Ok((
            PathVerification::Invalid,
            Some("Choose an Ascension or CoA folder or executable to begin.".to_string()),
        ));
    };

    let selected = state
        .game_executable_path
        .clone()
        .unwrap_or_else(|| game_path.clone());
    let inspection = TargetDetector::inspect(&PathBuf::from(selected))?;

    Ok((inspection.verification, Some(inspection.message)))
}

fn remember_detected_target_profiles(
    state: &mut crate::domain::LocalState,
    inspection: &PathInspection,
    current_target: &str,
    game_executable_path: Option<&str>,
    confirmed_addon_path: &str,
) {
    let profile = TargetPathState {
        game_path: Some(inspection.normalized_game_path.clone()),
        game_executable_path: inspection
            .game_executable_path
            .clone()
            .or_else(|| game_executable_path.map(str::to_string)),
        addon_path: Some(confirmed_addon_path.to_string()),
    };
    state.remember_target_profile(current_target, profile);

    for candidate in &inspection.candidate_addon_paths {
        if !candidate.exists || candidate.path == confirmed_addon_path {
            continue;
        }

        if let Some(target) = companion_target_for_candidate(current_target, &candidate.path) {
            state.remember_target_profile(
                target,
                TargetPathState {
                    game_path: Some(inspection.normalized_game_path.clone()),
                    game_executable_path: inspection
                        .game_executable_path
                        .clone()
                        .or_else(|| game_executable_path.map(str::to_string)),
                    addon_path: Some(candidate.path.clone()),
                },
            );
        }
    }
}

fn companion_target_for_candidate(current_target: &str, candidate_path: &str) -> Option<&'static str> {
    let is_live_target = current_target.eq_ignore_ascii_case(app_config::TARGET_NAME);
    let is_coa_target = current_target.eq_ignore_ascii_case(app_config::COA_TARGET_NAME);

    if is_live_target {
        app_config::infer_target_name_from_path_hint(candidate_path)
            .filter(|target| *target == app_config::COA_TARGET_NAME)
    } else if is_coa_target {
        app_config::infer_target_name_from_path_hint(candidate_path)
            .filter(|target| *target == app_config::TARGET_NAME)
    } else {
        None
    }
}

async fn build_addon_rows(
    runtime: &AppRuntime,
    state: &crate::domain::LocalState,
    catalog_resolution: &CatalogResolution,
) -> Vec<AddonRow> {
    let Some(catalog) = catalog_resolution.catalog.as_ref() else {
        return rows_from_local_state_only(state);
    };

    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    let catalog_floor_error =
        PackageValidator::validate_minimum_installer_version(&catalog.min_installer_version)
            .err()
            .map(|error| error.to_string());

    for addon in catalog
        .addons
        .iter()
        .filter(|addon| app_config::contains_target(&addon.targets, &state.selected_target))
    {
        seen.insert(addon.addon_id.clone());
        let installed = state.installed_addons.get(&addon.addon_id);

        let mut row = base_row(addon, installed);
        row.disabled_reason = catalog_floor_error.clone();
        let needs_setup = state
            .addon_path
            .as_ref()
            .map(PathBuf::from)
            .is_none_or(|path| !path.is_dir());

        match runtime
            .github_service()
            .fetch_addon_release_metadata(addon, &runtime.logger)
            .await
        {
            Ok(release) => populate_release_metadata(
                &mut row,
                addon,
                installed,
                &release,
                needs_setup,
                &state.selected_target,
            ),
            Err(error) => {
                row.status = if installed.is_some() {
                    AddonStatus::Installed
                } else {
                    AddonStatus::Error
                };
                row.error_message = Some(error.to_string());
                row.can_uninstall = installed.is_some() && !needs_setup;
                row.can_rollback = installed
                    .and_then(|value| value.backup_path.as_ref())
                    .is_some();
            }
        }

        rows.push(row);
    }

    for (addon_id, installed) in &state.installed_addons {
        if seen.contains(addon_id) {
            continue;
        }

        rows.push(AddonRow {
            addon_id: addon_id.clone(),
            display_name: installed
                .display_name
                .clone()
                .unwrap_or_else(|| addon_id.clone()),
            description: Some(
                "Managed locally, but not present in the current catalog.".to_string(),
            ),
            repo_attribution: installed.source_repo.clone(),
            repo_url: format!("https://github.com/{}", installed.source_repo),
            managed_folders: installed.folders.clone(),
            installed_version: Some(installed.version.clone()),
            latest_version: None,
            latest_published_at: None,
            last_installed_at: Some(installed.installed_at.clone()),
            release_notes: None,
            status: AddonStatus::Installed,
            error_message: Some("This addon is no longer present in the catalog.".to_string()),
            disabled_reason: Some("Catalog entry unavailable.".to_string()),
            can_install: false,
            can_update: false,
            can_uninstall: state
                .addon_path
                .as_ref()
                .map(PathBuf::from)
                .is_some_and(|path| path.is_dir()),
            can_rollback: installed.backup_path.is_some(),
            icon_url: None,
        });
    }

    rows.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    rows
}

fn base_row(
    addon: &crate::domain::CatalogAddon,
    installed: Option<&InstalledAddonState>,
) -> AddonRow {
    AddonRow {
        addon_id: addon.addon_id.clone(),
        display_name: addon.display_name.clone(),
        description: addon.description.clone(),
        repo_attribution: format!("{}/{}", addon.owner, addon.repo),
        repo_url: format!("https://github.com/{}/{}", addon.owner, addon.repo),
        managed_folders: addon.folders.clone(),
        installed_version: installed.map(|value| value.version.clone()),
        latest_version: None,
        latest_published_at: None,
        last_installed_at: installed.map(|value| value.installed_at.clone()),
        release_notes: None,
        status: if installed.is_some() {
            AddonStatus::Installed
        } else {
            AddonStatus::NotInstalled
        },
        error_message: None,
        disabled_reason: None,
        can_install: false,
        can_update: false,
        can_uninstall: false,
        can_rollback: installed
            .and_then(|value| value.backup_path.as_ref())
            .is_some(),
        icon_url: addon.icon_url.clone(),
    }
}

fn populate_release_metadata(
    row: &mut AddonRow,
    addon: &crate::domain::CatalogAddon,
    installed: Option<&InstalledAddonState>,
    release: &ResolvedAddonRelease,
    needs_setup: bool,
    selected_target: &str,
) {
    row.latest_version = Some(release.manifest.version.clone());
    row.latest_published_at = release.published_at.clone();
    row.release_notes = release.manifest.release_notes.clone();

    if let Err(error) =
        PackageValidator::validate_manifest(addon, &release.manifest, selected_target)
    {
        row.status = AddonStatus::Error;
        row.error_message = Some(error.to_string());
        return;
    }

    let can_write = row.disabled_reason.is_none() && !needs_setup;
    let can_uninstall = installed.is_some() && !needs_setup;

    match installed {
        Some(installed) => {
            row.can_uninstall = can_uninstall;
            row.can_rollback = installed.backup_path.is_some();
            match compare_versions(&installed.version, &release.manifest.version) {
                Ok(std::cmp::Ordering::Less) => {
                    row.status = AddonStatus::UpdateAvailable;
                    row.can_update = can_write;
                }
                Ok(_) => {
                    row.status = AddonStatus::Installed;
                }
                Err(error) => {
                    row.status = AddonStatus::Error;
                    row.error_message = Some(error.to_string());
                }
            }
        }
        None => {
            row.status = AddonStatus::NotInstalled;
            row.can_install = can_write;
        }
    }
}

fn rows_from_local_state_only(state: &crate::domain::LocalState) -> Vec<AddonRow> {
    let mut rows: Vec<AddonRow> = state
        .installed_addons
        .iter()
        .map(|(addon_id, installed)| AddonRow {
            addon_id: addon_id.clone(),
            display_name: installed
                .display_name
                .clone()
                .unwrap_or_else(|| addon_id.clone()),
            description: Some("Managed locally. The catalog is currently unavailable.".to_string()),
            repo_attribution: installed.source_repo.clone(),
            repo_url: format!("https://github.com/{}", installed.source_repo),
            managed_folders: installed.folders.clone(),
            installed_version: Some(installed.version.clone()),
            latest_version: None,
            latest_published_at: None,
            last_installed_at: Some(installed.installed_at.clone()),
            release_notes: None,
            status: AddonStatus::Installed,
            error_message: None,
            disabled_reason: Some("Catalog unavailable.".to_string()),
            can_install: false,
            can_update: false,
            can_uninstall: state
                .addon_path
                .as_ref()
                .map(PathBuf::from)
                .is_some_and(|path| path.is_dir()),
            can_rollback: installed.backup_path.is_some(),
            icon_url: None,
        })
        .collect();

    rows.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    rows
}
