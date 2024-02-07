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

    #[error("The path {path:?} is not a directory")]
    NotADirectory { path: PathBuf },

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
            Self::NotADirectory { .. } => {
                // TODO(2024.02): Once rust-lang/#86442 is stabilized, this can use ErrorKind::NotADirectory
                std::io::Error::new(ErrorKind::Other, "Not a directory")
            }
            Self::Other(message) => std::io::Error::new(ErrorKind::Other, message.as_str()),
        }
    }

    pub fn from_io(path: PathBuf, e: std::io::Error) -> Self {
        match e.kind() {
            ErrorKind::NotFound => Self::NotFound { path },
            ErrorKind::PermissionDenied => Self::AccessDenied { path },
            _ => {
                let msg = e.to_string();
                // TODO(2024.02): Once rust-lang/#86442 is stabilized, this work-around can be removed
                match msg.as_str().split_once(" (os error") {
                    Some(("Not a directory", _)) => Self::NotADirectory { path },
                    _ => Self::Other(msg),
                }
            }
        }
    }
}

pub trait LogixVfs: std::fmt::Debug + Send + Sync {
    type RoFile: std::io::Read;
    type ReadDir: Iterator<Item = Result<PathBuf, Error>>;

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, Error>;
    fn open_file(&self, path: &Path) -> Result<Self::RoFile, Error>;
    fn read_dir(&self, path: &Path) -> Result<Self::ReadDir, Error>;
}
