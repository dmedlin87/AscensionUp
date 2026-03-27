use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::app_config::TARGET_NAME;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandEnvelope<T>
where
    T: Serialize,
{
    pub data: Option<T>,
    pub error: Option<ErrorPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalState {
    pub selected_target: String,
    pub game_path: Option<String>,
    pub game_executable_path: Option<String>,
    pub addon_path: Option<String>,
    #[serde(default)]
    pub installed_addons: BTreeMap<String, InstalledAddonState>,
    pub last_catalog_refresh_at: Option<String>,
    pub cached_catalog_version: Option<String>,
}

impl Default for LocalState {
    fn default() -> Self {
        Self {
            selected_target: TARGET_NAME.to_string(),
            game_path: None,
            game_executable_path: None,
            addon_path: None,
            installed_addons: BTreeMap::new(),
            last_catalog_refresh_at: None,
            cached_catalog_version: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledAddonState {
    pub version: String,
    pub folders: Vec<String>,
    pub installed_at: String,
    pub backup_version: Option<String>,
    pub backup_path: Option<String>,
    pub source_repo: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub schema_version: u32,
    pub targets: Vec<String>,
    pub addons: Vec<CatalogAddon>,
    pub min_installer_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogAddon {
    pub addon_id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub owner: String,
    pub repo: String,
    pub targets: Vec<String>,
    pub folders: Vec<String>,
    pub manifest_strategy: String,
    pub manifest_asset_name: String,
    pub asset_name_pattern: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonManifest {
    pub schema_version: u32,
    pub addon_id: String,
    pub display_name: String,
    pub version: String,
    pub target_support: Vec<String>,
    pub folders: Vec<String>,
    pub asset_name: String,
    pub sha256: Option<String>,
    pub min_installer_version: String,
    pub release_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PathVerification {
    Verified,
    Unverified,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateAddonPath {
    pub path: String,
    pub exists: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathInspection {
    pub normalized_game_path: String,
    pub game_executable_path: Option<String>,
    pub verification: PathVerification,
    pub candidate_addon_paths: Vec<CandidateAddonPath>,
    pub proposed_addon_path: Option<String>,
    pub message: String,
    pub ascension_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CatalogStatus {
    Live,
    Cached,
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct CatalogResolution {
    pub catalog: Option<Catalog>,
    pub status: CatalogStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AddonStatus {
    NotInstalled,
    Installed,
    UpdateAvailable,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonRow {
    pub addon_id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub repo_attribution: String,
    pub repo_url: String,
    pub managed_folders: Vec<String>,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub latest_published_at: Option<String>,
    pub last_installed_at: Option<String>,
    pub release_notes: Option<String>,
    pub status: AddonStatus,
    pub error_message: Option<String>,
    pub disabled_reason: Option<String>,
    pub can_install: bool,
    pub can_update: bool,
    pub can_uninstall: bool,
    pub can_rollback: bool,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub installer_version: String,
    pub selected_target: String,
    pub game_path: Option<String>,
    pub game_executable_path: Option<String>,
    pub addon_path: Option<String>,
    pub path_verification: PathVerification,
    pub path_message: Option<String>,
    pub needs_setup: bool,
    pub catalog_status: CatalogStatus,
    pub catalog_message: Option<String>,
    pub catalog_url: String,
    pub last_catalog_refresh_at: Option<String>,
    pub addon_rows: Vec<AddonRow>,
    pub log_directory: String,
    pub game_running: bool,
    pub installer_release_page_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub snapshot: AppSnapshot,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallerUpdateStatus {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub download_url: Option<String>,
    pub release_page_url: String,
    pub published_at: Option<String>,
    pub available: bool,
    pub message: Option<String>,
}
