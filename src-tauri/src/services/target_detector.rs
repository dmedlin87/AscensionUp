use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::{CandidateAddonPath, PathInspection, PathVerification},
    error::InstallerError,
};

pub struct TargetDetector;

impl TargetDetector {
    pub fn inspect(selected_path: &Path) -> Result<PathInspection, InstallerError> {
        if !selected_path.exists() {
            return Err(InstallerError::validation(
                "target_missing",
                "The selected path does not exist.",
            ));
        }

        let selected_metadata = fs::metadata(selected_path).map_err(|err| {
            InstallerError::io(
                "target_inspect",
                format!("Could not inspect '{}'.", selected_path.display()),
                err,
            )
        })?;

        let (game_root, executable_path, direct_addon_path) = if selected_metadata.is_file() {
            let is_exe = selected_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));

            if !is_exe {
                return Err(InstallerError::validation(
                    "target_invalid",
                    "Select an Ascension folder or executable.",
                ));
            }

            let parent = selected_path.parent().ok_or_else(|| {
                InstallerError::validation(
                    "target_invalid",
                    "The selected executable does not have a parent directory.",
                )
            })?;

            let game_root = canonicalize_lossy(parent);
            (
                game_root,
                Some(display_path(&canonicalize_lossy(selected_path))),
                None,
            )
        } else if selected_metadata.is_dir() {
            let normalized = canonicalize_lossy(selected_path);
            if is_addon_directory(&normalized) {
                let game_root = derive_game_root_from_addon_dir(&normalized)
                    .unwrap_or_else(|| normalized.clone());
                (game_root, None, Some(normalized))
            } else if let Some(addon_dir) = addon_directory_from_child(&normalized) {
                let game_root = derive_game_root_from_addon_dir(&addon_dir)
                    .unwrap_or_else(|| addon_dir.clone());
                (game_root, None, Some(addon_dir))
            } else {
                (normalized, None, None)
            }
        } else {
            return Err(InstallerError::validation(
                "target_invalid",
                "Select an Ascension folder or executable.",
            ));
        };

        let mut candidate_paths = vec![
            candidate_path(
                game_root.join("Interface").join("AddOns"),
                "Interface\\AddOns",
            ),
            candidate_path(
                game_root
                    .join("Resources")
                    .join("Client")
                    .join("Interface")
                    .join("AddOns"),
                "Resources\\Client\\Interface\\AddOns",
            ),
            candidate_path(
                game_root
                    .join("resources")
                    .join("client")
                    .join("Interface")
                    .join("AddOns"),
                "resources\\client\\Interface\\AddOns",
            ),
        ];

        if let Some(addon_path) = direct_addon_path {
            let addon_text = display_path(&addon_path);
            if !candidate_paths
                .iter()
                .any(|candidate| candidate.path == addon_text)
            {
                candidate_paths.insert(
                    0,
                    CandidateAddonPath {
                        path: addon_text,
                        exists: addon_path.is_dir(),
                        label: "Selected AddOn Directory".to_string(),
                    },
                );
            }
        }

        let mut ascension_hints = Vec::new();
        let root_text = game_root.display().to_string().to_lowercase();
        if root_text.contains("ascension") {
            ascension_hints.push("The selected path contains 'Ascension'.".to_string());
        }
        if let Some(executable_path) = &executable_path {
            if executable_path.to_lowercase().contains("ascension") {
                ascension_hints.push("The selected executable contains 'Ascension'.".to_string());
            }
        }
        if game_root.join("Resources").join("Client").is_dir() {
            ascension_hints.push("The install contains Resources\\Client.".to_string());
        }

        let valid_candidates: Vec<&CandidateAddonPath> = candidate_paths
            .iter()
            .filter(|candidate| candidate.exists)
            .collect();

        let verification = if valid_candidates.is_empty() {
            PathVerification::Invalid
        } else if ascension_hints.is_empty() {
            PathVerification::Unverified
        } else {
            PathVerification::Verified
        };

        let proposed_addon_path = if let Some(candidate) = candidate_paths.iter().find(|c| {
            c.exists && c.label.eq_ignore_ascii_case("Resources\\Client\\Interface\\AddOns")
        }) {
            Some(candidate.path.clone())
        } else if valid_candidates.len() == 1 {
            Some(valid_candidates[0].path.clone())
        } else {
            None
        };

        let message = match verification {
            PathVerification::Invalid => "Could not find a valid Ascension addon folder.".to_string(),
            PathVerification::Verified if valid_candidates.len() == 1 => {
                "Found one valid addon directory.".to_string()
            }
            PathVerification::Verified => "Found multiple valid addon directories. Confirm which one to manage.".to_string(),
            PathVerification::Unverified if valid_candidates.len() == 1 => {
                "Found one addon directory, but this install could not be verified as Ascension. Confirm before saving.".to_string()
            }
            PathVerification::Unverified => {
                "Found multiple addon directories, but this install could not be verified as Ascension. Confirm before saving.".to_string()
            }
        };

        Ok(PathInspection {
            normalized_game_path: display_path(&game_root),
            game_executable_path: executable_path,
            verification,
            candidate_addon_paths: candidate_paths,
            proposed_addon_path,
            message,
            ascension_hints,
        })
    }
}

pub(crate) fn canonicalize_lossy(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn display_path(path: &Path) -> String {
    let raw = path.display().to_string();
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

fn candidate_path(path: PathBuf, label: &str) -> CandidateAddonPath {
    CandidateAddonPath {
        path: display_path(&path),
        exists: path.is_dir(),
        label: label.to_string(),
    }
}

pub(crate) fn is_addon_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|part| part.to_str()) else {
        return false;
    };
    if !name.eq_ignore_ascii_case("addons") {
        return false;
    }

    let Some(parent_name) = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|part| part.to_str())
    else {
        return false;
    };

    parent_name.eq_ignore_ascii_case("interface")
}

fn addon_directory_from_child(path: &Path) -> Option<PathBuf> {
    path.parent()
        .filter(|parent| is_addon_directory(parent))
        .map(PathBuf::from)
}

fn derive_game_root_from_addon_dir(addon_dir: &Path) -> Option<PathBuf> {
    let interface_dir = addon_dir.parent()?;
    let parent = interface_dir.parent()?;

    let path_parts: Vec<String> = addon_dir
        .iter()
        .map(|part| part.to_string_lossy().to_string())
        .collect();

    let resources_index = path_parts
        .iter()
        .position(|part| part.eq_ignore_ascii_case("resources"));
    let client_index = path_parts
        .iter()
        .position(|part| part.eq_ignore_ascii_case("client"));

    if resources_index
        .zip(client_index)
        .is_some_and(|(resources, client)| client == resources + 1)
    {
        parent.parent().map(PathBuf::from)
    } else {
        Some(parent.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{display_path, TargetDetector};
    use crate::domain::PathVerification;

    #[test]
    fn marks_verified_when_documented_path_and_hint_exist() {
        let temp = tempdir().expect("tempdir");
        let game_root = temp.path().join("Ascension");
        std::fs::create_dir_all(
            game_root
                .join("Resources")
                .join("Client")
                .join("Interface")
                .join("AddOns"),
        )
        .expect("create addons");

        let inspection = TargetDetector::inspect(&game_root).expect("inspection");

        assert_eq!(inspection.verification, PathVerification::Verified);
        assert!(inspection.proposed_addon_path.is_some());
    }

    #[test]
    fn marks_unverified_without_ascension_hints() {
        let temp = tempdir().expect("tempdir");
        let game_root = temp.path().join("Game");
        std::fs::create_dir_all(game_root.join("Interface").join("AddOns")).expect("create addons");

        let inspection = TargetDetector::inspect(&game_root).expect("inspection");

        assert_eq!(inspection.verification, PathVerification::Unverified);
    }

    #[test]
    fn marks_invalid_when_no_candidate_exists() {
        let temp = tempdir().expect("tempdir");
        let game_root = temp.path().join("Ascension");
        std::fs::create_dir_all(&game_root).expect("create");

        let inspection = TargetDetector::inspect(&game_root).expect("inspection");

        assert_eq!(inspection.verification, PathVerification::Invalid);
    }

    #[test]
    fn accepts_addon_directory_as_selected_path() {
        let temp = tempdir().expect("tempdir");
        let addon_dir = temp
            .path()
            .join("Ascension Launcher")
            .join("resources")
            .join("client")
            .join("Interface")
            .join("AddOns");
        std::fs::create_dir_all(&addon_dir).expect("create addons");

        let inspection = TargetDetector::inspect(&addon_dir).expect("inspection");

        assert_eq!(inspection.verification, PathVerification::Verified);
        let expected = display_path(&super::canonicalize_lossy(&addon_dir));
        assert_eq!(
            inspection.proposed_addon_path.as_deref(),
            Some(expected.as_str())
        );
    }

    #[test]
    fn accepts_specific_addon_folder_as_selected_path() {
        let temp = tempdir().expect("tempdir");
        let addon_dir = temp
            .path()
            .join("Ascension Launcher")
            .join("resources")
            .join("client")
            .join("Interface")
            .join("AddOns");
        let addon_folder = addon_dir.join("QuestBuddy");
        std::fs::create_dir_all(&addon_folder).expect("create addon");

        let inspection = TargetDetector::inspect(&addon_folder).expect("inspection");

        assert_eq!(inspection.verification, PathVerification::Verified);
        let expected = display_path(&super::canonicalize_lossy(&addon_dir));
        assert_eq!(
            inspection.proposed_addon_path.as_deref(),
            Some(expected.as_str())
        );
    }
}
