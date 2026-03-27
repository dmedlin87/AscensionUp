use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use chrono::Utc;

use crate::error::InstallerError;

#[derive(Debug)]
pub struct LogService {
    logs_dir: PathBuf,
    file: Mutex<File>,
}

impl LogService {
    pub fn new(logs_dir: &Path) -> Result<Self, InstallerError> {
        fs::create_dir_all(logs_dir).map_err(|err| {
            InstallerError::io(
                "log_setup",
                format!("Could not create '{}'.", logs_dir.display()),
                err,
            )
        })?;

        let file_name = format!("session-{}.log", Utc::now().format("%Y%m%d-%H%M%S"));
        let file_path = logs_dir.join(file_name);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|err| {
                InstallerError::io(
                    "log_setup",
                    format!("Could not open '{}'.", file_path.display()),
                    err,
                )
            })?;

        Ok(Self {
            logs_dir: logs_dir.to_path_buf(),
            file: Mutex::new(file),
        })
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    pub fn info(&self, scope: &str, message: impl AsRef<str>) {
        self.write("INFO", scope, message.as_ref());
    }

    pub fn warn(&self, scope: &str, message: impl AsRef<str>) {
        self.write("WARN", scope, message.as_ref());
    }

    pub fn error(&self, scope: &str, message: impl AsRef<str>) {
        self.write("ERROR", scope, message.as_ref());
    }

    fn write(&self, level: &str, scope: &str, message: &str) {
        let line = format!(
            "{} [{}] [{}] {}\n",
            Utc::now().to_rfc3339(),
            level,
            scope,
            message
        );

        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());
        }
    }
}
