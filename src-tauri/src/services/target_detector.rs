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

        let (game_root, executable_path) = if selected_metadata.is_file() {
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

            (
                canonicalize_lossy(parent),
                Some(canonicalize_lossy(selected_path).display().to_string()),
            )
        } else if selected_metadata.is_dir() {
            (canonicalize_lossy(selected_path), None)
        } else {
            return Err(InstallerError::validation(
                "target_invalid",
                "Select an Ascension folder or executable.",
            ));
        };

        let candidate_paths = vec![
            CandidateAddonPath {
                path: game_root.join("Interface").join("AddOns").display().to_string(),
                exists: game_root.join("Interface").join("AddOns").is_dir(),
                label: "Interface\\AddOns".to_string(),
            },
            CandidateAddonPath {
                path: game_root
                    .join("Resources")
                    .join("Client")
                    .join("Interface")
                    .join("Addons")
                    .display()
                    .to_string(),
                exists: game_root
                    .join("Resources")
                    .join("Client")
                    .join("Interface")
                    .join("Addons")
                    .is_dir(),
                label: "Resources\\Client\\Interface\\Addons".to_string(),
            },
        ];

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

        let valid_candidates: Vec<&CandidateAddonPath> =
            candidate_paths.iter().filter(|candidate| candidate.exists).collect();

        let verification = if valid_candidates.is_empty() {
            PathVerification::Invalid
        } else if ascension_hints.is_empty() {
            PathVerification::Unverified
        } else {
            PathVerification::Verified
        };

        let proposed_addon_path = if valid_candidates.len() == 1 {
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
            normalized_game_path: game_root.display().to_string(),
            game_executable_path: executable_path,
            verification,
            candidate_addon_paths: candidate_paths,
            proposed_addon_path,
            message,
            ascension_hints,
        })
    }
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::TargetDetector;
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
                .join("Addons"),
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
}
