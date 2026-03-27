use std::{fs, path::PathBuf};

use reqwest::Client;

use crate::{
    app_config::{MANIFEST_ASSET_NAME, TARGET_NAME},
    domain::{Catalog, CatalogResolution, CatalogStatus, LocalState},
    error::InstallerError,
    services::{log_service::LogService, package_validator::PackageValidator},
};

#[derive(Debug, Clone)]
pub struct CatalogService {
    catalog_url: String,
    cache_file: PathBuf,
}

impl CatalogService {
    pub fn new(catalog_url: String, cache_file: PathBuf) -> Self {
        Self { catalog_url, cache_file }
    }

    pub async fn load_catalog(
        &self,
        client: &Client,
        logger: &LogService,
        state: &mut LocalState,
    ) -> CatalogResolution {
        match self.fetch_remote_catalog(client, logger).await {
            Ok(catalog) => {
                state.last_catalog_refresh_at = Some(now_iso());
                state.cached_catalog_version = Some(catalog.schema_version.to_string());
                if let Err(err) = self.save_cache(&catalog) {
                    logger.warn("catalog", format!("Could not update cache: {}", err));
                }
                CatalogResolution {
                    catalog: Some(catalog),
                    status: CatalogStatus::Live,
                    message: None,
                }
            }
            Err(remote_error) => match self.load_cached_catalog() {
                Ok(catalog) => {
                    logger.warn(
                        "catalog",
                        format!("Using cached catalog because the remote fetch failed: {}", remote_error),
                    );
                    CatalogResolution {
                        catalog: Some(catalog),
                        status: CatalogStatus::Cached,
                        message: Some(
                            "The live catalog could not be loaded. Showing the last cached catalog instead."
                                .to_string(),
                        ),
                    }
                }
                Err(cache_error) => {
                    logger.error(
                        "catalog",
                        format!(
                            "Catalog unavailable. Remote error: {}. Cache error: {}",
                            remote_error, cache_error
                        ),
                    );
                    CatalogResolution {
                        catalog: None,
                        status: CatalogStatus::Unavailable,
                        message: Some(
                            "The catalog could not be loaded. Managed addons from local state remain visible."
                                .to_string(),
                        ),
                    }
                }
            },
        }
    }

    async fn fetch_remote_catalog(
        &self,
        client: &Client,
        logger: &LogService,
    ) -> Result<Catalog, InstallerError> {
        logger.info("catalog", format!("Fetching {}", self.catalog_url));
        let response = client
            .get(&self.catalog_url)
            .send()
            .await
            .map_err(|err| InstallerError::network("catalog_fetch", "Could not reach the remote catalog.", err.to_string()))?;

        if !response.status().is_success() {
            return Err(InstallerError::network(
                "catalog_fetch",
                format!("Could not load the remote catalog (HTTP {}).", response.status()),
                response.status().to_string(),
            ));
        }

        let body = response.text().await.map_err(|err| {
            InstallerError::network(
                "catalog_fetch",
                "Could not read the remote catalog response.",
                err.to_string(),
            )
        })?;

        let catalog: Catalog = serde_json::from_str(&body).map_err(|err| {
            InstallerError::validation_with_details(
                "catalog_parse",
                "The remote catalog is not valid JSON.",
                err,
            )
        })?;

        Self::validate_catalog(&catalog)?;

        Ok(catalog)
    }

    fn load_cached_catalog(&self) -> Result<Catalog, InstallerError> {
        let raw = fs::read_to_string(&self.cache_file).map_err(|err| {
            InstallerError::io(
                "catalog_cache",
                format!("Could not read '{}'.", self.cache_file.display()),
                err,
            )
        })?;

        let catalog: Catalog = serde_json::from_str(&raw).map_err(|err| {
            InstallerError::validation_with_details(
                "catalog_cache",
                "The cached catalog is not valid JSON.",
                err,
            )
        })?;

        Self::validate_catalog(&catalog)?;

        Ok(catalog)
    }

    fn save_cache(&self, catalog: &Catalog) -> Result<(), InstallerError> {
        if let Some(parent) = self.cache_file.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                InstallerError::io(
                    "catalog_cache",
                    format!("Could not create '{}'.", parent.display()),
                    err,
                )
            })?;
        }

        let serialized = serde_json::to_string_pretty(catalog).map_err(|err| {
            InstallerError::validation_with_details(
                "catalog_cache",
                "Could not serialize the catalog cache.",
                err,
            )
        })?;

        fs::write(&self.cache_file, serialized).map_err(|err| {
            InstallerError::io(
                "catalog_cache",
                format!("Could not write '{}'.", self.cache_file.display()),
                err,
            )
        })
    }

    pub fn validate_catalog(catalog: &Catalog) -> Result<(), InstallerError> {
        if catalog.schema_version == 0 {
            return Err(InstallerError::validation(
                "catalog_schema",
                "The catalog schemaVersion must be greater than zero.",
            ));
        }

        if !catalog.targets.iter().any(|target| target == TARGET_NAME) {
            return Err(InstallerError::validation(
                "catalog_target",
                "The catalog does not support Bronzebeard.",
            ));
        }

        PackageValidator::validate_semver(
            &catalog.min_installer_version,
            "The catalog minInstallerVersion is not valid semver.",
        )?;

        for addon in &catalog.addons {
            if addon.manifest_strategy != "release-asset" {
                return Err(InstallerError::validation(
                    "catalog_manifest_strategy",
                    format!(
                        "Addon '{}' must use the 'release-asset' manifest strategy.",
                        addon.display_name
                    ),
                ));
            }

            if addon.manifest_asset_name != MANIFEST_ASSET_NAME {
                return Err(InstallerError::validation(
                    "catalog_manifest_asset",
                    format!(
                        "Addon '{}' must use '{}'.",
                        addon.display_name, MANIFEST_ASSET_NAME
                    ),
                ));
            }

            if addon.targets.iter().any(|target| target == TARGET_NAME) {
                PackageValidator::validate_folder_names(&addon.folders)?;
            }
        }

        Ok(())
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{Catalog, CatalogAddon},
        services::catalog_service::CatalogService,
    };

    #[test]
    fn validates_bronzebeard_catalog() {
        let catalog = Catalog {
            schema_version: 1,
            targets: vec!["Bronzebeard".to_string()],
            addons: vec![CatalogAddon {
                addon_id: "my-addon".to_string(),
                display_name: "My Addon".to_string(),
                description: None,
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                targets: vec!["Bronzebeard".to_string()],
                folders: vec!["MyAddon".to_string()],
                manifest_strategy: "release-asset".to_string(),
                manifest_asset_name: "addon-manifest.json".to_string(),
                asset_name_pattern: "MyAddon-v{version}.zip".to_string(),
                icon_url: None,
            }],
            min_installer_version: "1.0.0".to_string(),
        };

        assert!(CatalogService::validate_catalog(&catalog).is_ok());
    }
}
