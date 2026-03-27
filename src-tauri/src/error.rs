use std::io;

use thiserror::Error;

use crate::domain::ErrorPayload;

#[derive(Debug, Error)]
pub enum InstallerError {
    #[error("{message}")]
    Validation {
        code: &'static str,
        message: String,
        details: Option<String>,
    },
    #[error("{message}")]
    Io {
        code: &'static str,
        message: String,
        details: Option<String>,
    },
    #[error("{message}")]
    Network {
        code: &'static str,
        message: String,
        details: Option<String>,
    },
}

impl InstallerError {
    pub fn validation(code: &'static str, message: impl Into<String>) -> Self {
        Self::Validation {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn validation_with_details(
        code: &'static str,
        message: impl Into<String>,
        details: impl std::fmt::Display,
    ) -> Self {
        Self::Validation {
            code,
            message: message.into(),
            details: Some(details.to_string()),
        }
    }

    pub fn io(code: &'static str, message: impl Into<String>, err: io::Error) -> Self {
        Self::Io {
            code,
            message: message.into(),
            details: Some(err.to_string()),
        }
    }

    pub fn network(
        code: &'static str,
        message: impl Into<String>,
        details: impl std::fmt::Display,
    ) -> Self {
        Self::Network {
            code,
            message: message.into(),
            details: Some(details.to_string()),
        }
    }

    pub fn payload(&self) -> ErrorPayload {
        match self {
            Self::Validation {
                code,
                message,
                details,
            }
            | Self::Io {
                code,
                message,
                details,
            }
            | Self::Network {
                code,
                message,
                details,
            } => ErrorPayload {
                code: (*code).to_string(),
                message: message.clone(),
                details: details.clone(),
            },
        }
    }
}
