#![deny(warnings, clippy::all)]

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

mod rel_fs;

pub use rel_fs::RelFs;

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Failed to locate {path:?}")]
    NotFound { path: PathBuf },

    #[error("Failed to access {path:?}")]
    AccessDenied { path: PathBuf },

    #[error("The path {path:?} is outside acceptable bounds")]
    PathOutsideBounds { path: PathBuf },

    /// Used for other errors that is not defined already. Do not depend on this
    /// for anything other than logging. If you need to check an error that is
    /// reported as other, please request the error to be added instead.
    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn to_io_error(&self) -> std::io::Error {
        match self {
            Self::NotFound { .. } => ErrorKind::NotFound.into(),
            Self::AccessDenied { .. } => ErrorKind::PermissionDenied.into(),
            Self::PathOutsideBounds { .. } => ErrorKind::InvalidInput.into(),
            Self::Other(message) => std::io::Error::new(ErrorKind::Other, message.as_str()),
        }
    }

    pub fn from_io(path: PathBuf, e: std::io::Error) -> Self {
        match e.kind() {
            ErrorKind::NotFound => Self::NotFound { path },
            ErrorKind::PermissionDenied => Self::AccessDenied { path },
            _ => Self::Other(e.to_string()),
        }
    }
}

pub trait LogixVfs: std::fmt::Debug + Send + Sync {
    type RoFile: std::io::Read;

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, Error>;
    fn open_file(&self, path: &Path) -> Result<Self::RoFile, Error>;
}
