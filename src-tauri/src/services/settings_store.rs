use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{app_config::resolve_target_name, domain::LocalState, error::InstallerError};

#[derive(Debug, Clone)]
pub struct SettingsStore {
    state_file: PathBuf,
}

impl SettingsStore {
    pub fn new(state_file: PathBuf) -> Self {
        Self { state_file }
    }

    pub fn load(&self) -> Result<LocalState, InstallerError> {
        if !self.state_file.exists() {
            return Ok(LocalState::default());
        }

        let raw = fs::read_to_string(&self.state_file).map_err(|err| {
            InstallerError::io(
                "state_load",
                format!("Could not read '{}'.", self.state_file.display()),
                err,
            )
        })?;

        let mut state: LocalState = serde_json::from_str(&raw).map_err(|err| {
            InstallerError::validation_with_details(
                "state_parse",
                "Could not parse the local state file.",
                err,
            )
        })?;

        let current_target = state.selected_target.clone();
        state.selected_target = resolve_target_name(
            Some(current_target.as_str()),
            &[
                state.game_path.as_deref(),
                state.game_executable_path.as_deref(),
                state.addon_path.as_deref(),
            ],
        );
        state.activate_selected_target_profile();

        Ok(state)
    }

    pub fn save(&self, state: &LocalState) -> Result<(), InstallerError> {
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                InstallerError::io(
                    "state_save",
                    format!("Could not create '{}'.", parent.display()),
                    err,
                )
            })?;
        }

        let mut state_to_save = state.clone();
        state_to_save.remember_selected_target_profile();

        let serialized = serde_json::to_string_pretty(&state_to_save).map_err(|err| {
            InstallerError::validation_with_details(
                "state_serialize",
                "Could not serialize local state.",
                err,
            )
        })?;

        atomic_write(&self.state_file, serialized.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::SettingsStore;
    use crate::domain::{LocalState, TargetPathState};

    #[test]
    fn load_rehydrates_the_selected_target_profile() {
        let temp = tempdir().expect("tempdir");
        let state_file = temp.path().join("state.json");

        let mut state = LocalState::default();
        state.selected_target = "CoA".to_string();
        state.target_profiles.insert(
            "Bronzebeard".to_string(),
            TargetPathState {
                game_path: Some(r"C:\Games\Ascension".to_string()),
                game_executable_path: Some(r"C:\Games\Ascension\Ascension.exe".to_string()),
                addon_path: Some(r"C:\Games\Ascension\Interface\AddOns".to_string()),
            },
        );
        state.target_profiles.insert(
            "CoA".to_string(),
            TargetPathState {
                game_path: Some(r"C:\Games\Ascension PTR".to_string()),
                game_executable_path: Some(r"C:\Games\Ascension PTR\Ascension.exe".to_string()),
                addon_path: Some(r"C:\Games\Ascension PTR\Resources\ascension_ptr\Interface\AddOns".to_string()),
            },
        );

        fs::write(
            &state_file,
            serde_json::to_string_pretty(&state).expect("serialize"),
        )
        .expect("write state");

        let store = SettingsStore::new(state_file);
        let loaded = store.load().expect("load");

        assert_eq!(loaded.selected_target, "CoA");
        assert_eq!(loaded.game_path.as_deref(), Some(r"C:\Games\Ascension PTR"));
        assert_eq!(
            loaded.addon_path.as_deref(),
            Some(r"C:\Games\Ascension PTR\Resources\ascension_ptr\Interface\AddOns")
        );
        assert_eq!(
            loaded.game_executable_path.as_deref(),
            Some(r"C:\Games\Ascension PTR\Ascension.exe")
        );
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), InstallerError> {
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, contents).map_err(|err| {
        InstallerError::io(
            "state_save",
            format!("Could not write '{}'.", temp_path.display()),
            err,
        )
    })?;

    if path.exists() {
        fs::remove_file(path).map_err(|err| {
            InstallerError::io(
                "state_save",
                format!("Could not replace '{}'.", path.display()),
                err,
            )
        })?;
    }

    fs::rename(&temp_path, path).map_err(|err| {
        InstallerError::io(
            "state_save",
            format!("Could not finalize '{}'.", path.display()),
            err,
        )
    })
}
