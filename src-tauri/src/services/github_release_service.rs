use std::cmp::Ordering;

use reqwest::Client;
use semver::Version;
use serde::Deserialize;

use crate::{
    app_config,
    domain::{AddonManifest, CatalogAddon, InstallerUpdateStatus},
    error::InstallerError,
    services::log_service::LogService,
};

#[derive(Debug, Clone)]
pub struct GitHubReleaseService {
    client: Client,
}

#[derive(Debug, Clone)]
pub struct ResolvedAddonRelease {
    pub manifest: AddonManifest,
    pub asset_download_url: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

impl GitHubReleaseService {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn fetch_addon_release_metadata(
        &self,
        addon: &CatalogAddon,
        logger: &LogService,
    ) -> Result<ResolvedAddonRelease, InstallerError> {
        let release = self.latest_release(&addon.owner, &addon.repo, logger).await?;
        let manifest_asset = release
            .assets
            .iter()
            .find(|asset| asset.name == addon.manifest_asset_name)
            .ok_or_else(|| {
                InstallerError::validation(
                    "manifest_asset_missing",
                    format!(
                        "The latest release for '{}' does not contain '{}'.",
                        addon.display_name, addon.manifest_asset_name
                    ),
                )
            })?;

        let manifest_body = self.download_text(&manifest_asset.browser_download_url).await?;
        let manifest: AddonManifest = serde_json::from_str(&manifest_body).map_err(|err| {
            InstallerError::validation_with_details(
                "manifest_parse",
                format!("The manifest for '{}' is not valid JSON.", addon.display_name),
                err,
            )
        })?;

        let package_asset = release
            .assets
            .iter()
            .find(|asset| asset.name == manifest.asset_name)
            .ok_or_else(|| {
                InstallerError::validation(
                    "package_asset_missing",
                    format!(
                        "The release for '{}' does not contain '{}'.",
                        addon.display_name, manifest.asset_name
                    ),
                )
            })?;

        Ok(ResolvedAddonRelease {
            manifest,
            asset_download_url: package_asset.browser_download_url.clone(),
            published_at: release.published_at,
        })
    }

    pub async fn download_to_file(
        &self,
        url: &str,
        destination: &std::path::Path,
        logger: &LogService,
    ) -> Result<(), InstallerError> {
        logger.info("github", format!("Downloading {}", url));
        let response = self.client.get(url).send().await.map_err(|err| {
            InstallerError::network("asset_download", "Could not download the release asset.", err.to_string())
        })?;

        if !response.status().is_success() {
            return Err(InstallerError::network(
                "asset_download",
                format!("Could not download the release asset (HTTP {}).", response.status()),
                response.status().to_string(),
            ));
        }

        let bytes = response.bytes().await.map_err(|err| {
            InstallerError::network("asset_download", "Could not read the release asset.", err.to_string())
        })?;

        std::fs::write(destination, bytes).map_err(|err| {
            InstallerError::io(
                "asset_download",
                format!("Could not write '{}'.", destination.display()),
                err,
            )
        })
    }

    pub async fn check_installer_update(
        &self,
        logger: &LogService,
    ) -> Result<InstallerUpdateStatus, InstallerError> {
        let release = self
            .latest_release(
                &app_config::installer_repo_owner(),
                &app_config::installer_repo_name(),
                logger,
            )
            .await?;

        let latest_version = release.tag_name.trim_start_matches('v').to_string();
        let available = compare_versions(env!("CARGO_PKG_VERSION"), &latest_version)?
            == Ordering::Less;
        let download_url = release
            .assets
            .iter()
            .find(|asset| asset.name == app_config::INSTALLER_ASSET_NAME)
            .map(|asset| asset.browser_download_url.clone())
            .or_else(|| Some(app_config::installer_download_url()));

        Ok(InstallerUpdateStatus {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            latest_version: Some(latest_version),
            download_url,
            release_page_url: app_config::installer_release_page_url(),
            published_at: release.published_at,
            available,
            message: if available {
                Some("A newer installer version is available.".to_string())
            } else {
                None
            },
        })
    }

    async fn latest_release(
        &self,
        owner: &str,
        repo: &str,
        logger: &LogService,
    ) -> Result<GitHubRelease, InstallerError> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
        logger.info("github", format!("Fetching {}", url));

        let response = self.client.get(&url).send().await.map_err(|err| {
            InstallerError::network("github_release", "Could not reach GitHub Releases.", err.to_string())
        })?;

        if !response.status().is_success() {
            return Err(InstallerError::network(
                "github_release",
                format!("Could not load the latest release for {owner}/{repo} (HTTP {}).", response.status()),
                response.status().to_string(),
            ));
        }

        response.json::<GitHubRelease>().await.map_err(|err| {
            InstallerError::network(
                "github_release",
                "Could not parse the GitHub release response.",
                err.to_string(),
            )
        })
    }

    async fn download_text(&self, url: &str) -> Result<String, InstallerError> {
        let response = self.client.get(url).send().await.map_err(|err| {
            InstallerError::network("manifest_download", "Could not download the manifest.", err.to_string())
        })?;

        if !response.status().is_success() {
            return Err(InstallerError::network(
                "manifest_download",
                format!("Could not download the manifest (HTTP {}).", response.status()),
                response.status().to_string(),
            ));
        }

        response.text().await.map_err(|err| {
            InstallerError::network(
                "manifest_download",
                "Could not read the manifest response.",
                err.to_string(),
            )
        })
    }
}

pub fn compare_versions(current: &str, latest: &str) -> Result<Ordering, InstallerError> {
    let current_version = Version::parse(current).map_err(|err| {
        InstallerError::validation_with_details(
            "installer_version",
            "The current installer version is not valid semver.",
            err,
        )
    })?;
    let latest_version = Version::parse(latest).map_err(|err| {
        InstallerError::validation_with_details(
            "installer_version",
            "The latest installer version is not valid semver.",
            err,
        )
    })?;

    Ok(current_version.cmp(&latest_version))
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::compare_versions;

    #[test]
    fn compares_semver_versions() {
        assert_eq!(
            compare_versions("1.0.0", "1.1.0").expect("compare"),
            Ordering::Less
        );
        assert_eq!(
            compare_versions("1.2.0", "1.1.0").expect("compare"),
            Ordering::Greater
        );
    }
}
