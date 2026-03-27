use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use reqwest::Client;
use tauri::{AppHandle, Manager};

use crate::{
    app_config,
    error::InstallerError,
    services::{
        catalog_service::CatalogService, github_release_service::GitHubReleaseService,
        log_service::LogService, settings_store::SettingsStore,
    },
};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub backups_dir: PathBuf,
    pub state_file: PathBuf,
    pub catalog_cache_file: PathBuf,
}

#[derive(Clone)]
pub struct AppRuntime {
    pub paths: AppPaths,
    pub logger: Arc<LogService>,
    pub http_client: Client,
    pub catalog_url: String,
}

impl AppRuntime {
    pub fn new(app: &AppHandle) -> Result<Self, InstallerError> {
        let config_dir = app.path().app_config_dir().map_err(|err| {
            InstallerError::validation(
                "path_resolution",
                format!("Could not resolve app config directory: {err}"),
            )
        })?;
        let data_dir = app.path().app_data_dir().map_err(|err| {
            InstallerError::validation(
                "path_resolution",
                format!("Could not resolve app data directory: {err}"),
            )
        })?;
        let cache_dir = app.path().app_cache_dir().map_err(|err| {
            InstallerError::validation(
                "path_resolution",
                format!("Could not resolve app cache directory: {err}"),
            )
        })?;

        let logs_dir = data_dir.join("logs");
        let backups_dir = data_dir.join("backups");
        let state_file = config_dir.join("state.json");
        let catalog_cache_file = cache_dir.join("catalog-cache.json");

        for path in [&config_dir, &data_dir, &cache_dir, &logs_dir, &backups_dir] {
            fs::create_dir_all(path).map_err(|err| {
                InstallerError::io(
                    "path_setup",
                    format!("Could not create '{}'.", path.display()),
                    err,
                )
            })?;
        }

        let logger = Arc::new(LogService::new(&logs_dir)?);
        let http_client = Client::builder()
            .user_agent(format!(
                "AscensionAddonInstaller/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|err| {
                InstallerError::network(
                    "http_client",
                    "Could not initialize the GitHub client.",
                    err.to_string(),
                )
            })?;

        Ok(Self {
            paths: AppPaths {
                cache_dir,
                logs_dir,
                backups_dir,
                state_file,
                catalog_cache_file,
            },
            logger,
            http_client,
            catalog_url: app_config::catalog_url(),
        })
    }

    pub fn settings_store(&self) -> SettingsStore {
        SettingsStore::new(self.paths.state_file.clone())
    }

    pub fn catalog_service(&self) -> CatalogService {
        CatalogService::new(
            self.catalog_url.clone(),
            self.paths.catalog_cache_file.clone(),
        )
    }

    pub fn github_service(&self) -> GitHubReleaseService {
        GitHubReleaseService::new(self.http_client.clone())
    }

    pub fn open_logs_folder(&self) -> Result<(), InstallerError> {
        let status = Command::new("explorer")
            .arg(self.paths.logs_dir.as_os_str())
            .status()
            .map_err(|err| {
                InstallerError::io("open_logs", "Could not open the log folder.", err)
            })?;

        if !status.success() {
            return Err(InstallerError::validation(
                "open_logs",
                "Could not open the log folder.",
            ));
        }

        Ok(())
    }

    pub fn clear_backups(&self) -> Result<(), InstallerError> {
        clear_directory(&self.paths.backups_dir)
    }
}

pub fn clear_directory(path: &Path) -> Result<(), InstallerError> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|err| {
            InstallerError::io(
                "cleanup",
                format!("Could not clear '{}'.", path.display()),
                err,
            )
        })?;
    }
    fs::create_dir_all(path).map_err(|err| {
        InstallerError::io(
            "cleanup",
            format!("Could not recreate '{}'.", path.display()),
            err,
        )
    })
}
