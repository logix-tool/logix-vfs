use std::path::Path;

mod rel_fs;

pub use rel_fs::RelFs;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Not found")]
    NotFound,

    #[error("Access denied")]
    AccessDenied,
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match e.kind() {
            ErrorKind::NotFound => Self::NotFound,
            ErrorKind::PermissionDenied => Self::AccessDenied,
            _ => todo!("{e:?}"),
        }
    }
}

pub trait LogixVfs {
    type RoFile: std::io::Read;

    fn open_file(&self, path: &Path) -> Result<Self::RoFile, Error>;
}
