use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{app_config::TARGET_NAME, domain::LocalState, error::InstallerError};

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

        if state.selected_target.is_empty() {
            state.selected_target = TARGET_NAME.to_string();
        }

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

        let serialized = serde_json::to_string_pretty(state).map_err(|err| {
            InstallerError::validation_with_details(
                "state_serialize",
                "Could not serialize local state.",
                err,
            )
        })?;

        atomic_write(&self.state_file, serialized.as_bytes())
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
