#![deny(warnings, clippy::all)]

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

mod rel_fs;

pub use rel_fs::RelFs;

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Not found")]
    NotFound,

    #[error("Access denied")]
    AccessDenied,

    #[error("The path is outside acceptable bounds")]
    PathOutsideBounds,
}

impl Error {
    pub fn to_io_error(&self) -> std::io::Error {
        match self {
            Self::NotFound => ErrorKind::NotFound.into(),
            Self::AccessDenied => ErrorKind::PermissionDenied.into(),
            Self::PathOutsideBounds => ErrorKind::InvalidInput.into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            ErrorKind::NotFound => Self::NotFound,
            ErrorKind::PermissionDenied => Self::AccessDenied,
            _ => todo!("{e:?}"),
        }
    }
}

pub trait LogixVfs: std::fmt::Debug + Send + Sync {
    type RoFile: std::io::Read;

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, Error>;
    fn open_file(&self, path: &Path) -> Result<Self::RoFile, Error>;
}
