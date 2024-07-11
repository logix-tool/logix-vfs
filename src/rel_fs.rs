use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::{utils::PathUtil, Error, LogixVfs};

#[derive(Debug)]
pub struct RelFs {
    root: PathBuf,
    cur_dir: PathBuf,
}

impl RelFs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cur_dir: PathBuf::new(),
        }
    }

    pub fn chdir(&mut self, path: impl AsRef<Path>) -> Result<&Path, Error> {
        self.cur_dir = self.resolve_path(true, path)?;
        Ok(&self.cur_dir)
    }

    fn resolve_path(&self, relative: bool, path: impl AsRef<Path>) -> Result<PathBuf, Error> {
        PathUtil {
            root: &self.root,
            cur_dir: &self.cur_dir,
        }
        .resolve_path(relative, path.as_ref())
    }
}

pub struct ReadDir {
    path: PathBuf,
    prefix: PathBuf,
    it: std::fs::ReadDir,
}

impl Iterator for ReadDir {
    type Item = Result<PathBuf, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.it.next()? {
            Ok(entry) => {
                let full_path = entry.path();
                full_path
                    .strip_prefix(&self.prefix)
                    .map_err(|e| {
                        // NOTE(2024.02): This should not happen, at least I don't know how to trigger it
                        Error::Other(format!(
                            "Failed to strip prefix {:?} off {full_path:?}: {e}",
                            self.prefix
                        ))
                    })
                    .map(|p| p.to_path_buf())
            }
            Err(e) => Err(Error::from_io(self.path.clone(), e)),
        })
    }
}

impl LogixVfs for RelFs {
    type RoFile = File;
    type DirEntry = PathBuf;
    type ReadDir = ReadDir;

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, Error> {
        self.resolve_path(true, path)
    }

    fn open_file(&self, path: &Path) -> Result<Self::RoFile, Error> {
        let full_path = self.resolve_path(false, path)?;
        File::open(full_path).map_err(|e| Error::from_io(path.to_path_buf(), e))
    }

    fn read_dir(&self, path: &Path) -> Result<Self::ReadDir, Error> {
        let full_path = self.resolve_path(false, path)?;
        let it = full_path
            .read_dir()
            .map_err(|e| Error::from_io(path.to_path_buf(), e))?;
        Ok(ReadDir {
            path: path.to_path_buf(),
            prefix: full_path,
            it,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestCase<'a> = &'a [(Option<(&'a str, &'a str)>, &'a [&'a str], &'a str, &'a str)];

    static PATHS_TO_TEST: TestCase = &[
        (
            None,
            &[
                ".config/awesome-app/config.toml",
                ".config/./awesome-app/./config.toml",
            ],
            "/home/zeldor/.config/awesome-app/config.toml",
            ".config/awesome-app/config.toml",
        ),
        (
            None,
            &[".config/./awesome-app/../config.toml"],
            "/home/zeldor/.config/config.toml",
            ".config/config.toml",
        ),
        (
            Some((".config", ".config")),
            &["awesome-app"],
            "/home/zeldor/.config/awesome-app",
            ".config/awesome-app",
        ),
        (
            None,
            &["../awesome-app"],
            "/home/zeldor/awesome-app",
            "awesome-app",
        ),
        (
            Some(("awesome-app", ".config/awesome-app")),
            &[
                "config.toml",
                "./config.toml",
                "/.config/awesome-app/config.toml",
            ],
            "/home/zeldor/.config/awesome-app/config.toml",
            ".config/awesome-app/config.toml",
        ),
        (
            None,
            &["../config.toml"],
            "/home/zeldor/.config/config.toml",
            ".config/config.toml",
        ),
        (None, &["/.bashrc"], "/home/zeldor/.bashrc", ".bashrc"),
    ];

    #[test]
    fn basics() {
        let mut fs = RelFs::new("/home/zeldor");

        for &(chdir, paths, if_full, if_relative) in PATHS_TO_TEST {
            if let Some((chdir, rel_after)) = chdir {
                assert_eq!(fs.chdir(chdir), Ok(Path::new(rel_after)), "{chdir:?}");
            }
            for path in paths {
                assert_eq!(
                    fs.resolve_path(false, path),
                    Ok(PathBuf::from(if_full)),
                    "{path:?}"
                );
                assert_eq!(
                    fs.resolve_path(true, path),
                    Ok(PathBuf::from(if_relative)),
                    "{path:?}"
                );
            }
        }
    }

    #[test]
    fn errors() {
        let mut fs = RelFs::new("src");

        assert_eq!(
            fs.canonicalize_path("../test".as_ref()),
            Err(Error::PathOutsideBounds {
                path: "../test".into()
            })
        );

        assert_eq!(
            fs.canonicalize_path("test/../test/../../test".as_ref()),
            Err(Error::PathOutsideBounds {
                path: "test/../test/../../test".into()
            })
        );

        assert_eq!(fs.open_file("lib.rs".as_ref()).err(), None);
        assert_eq!(
            fs.open_file("not-lib.rs".as_ref()).err(),
            Some(Error::NotFound {
                path: "not-lib.rs".into()
            })
        );
        assert_eq!(
            fs.open_file("../outside.txt".as_ref()).err(),
            Some(Error::PathOutsideBounds {
                path: "../outside.txt".into()
            })
        );

        assert_eq!(
            fs.chdir("../outside").err(),
            Some(Error::PathOutsideBounds {
                path: "../outside".into()
            })
        );

        assert_eq!(
            fs.read_dir("../outside".as_ref()).err(),
            Some(Error::PathOutsideBounds {
                path: "../outside".into()
            })
        );

        assert_eq!(
            fs.read_dir("lib.rs".as_ref()).err(),
            Some(Error::NotADirectory {
                path: "lib.rs".into()
            })
        );
        assert_eq!(
            fs.read_dir("not-lib.rs".as_ref()).err(),
            Some(Error::NotFound {
                path: "not-lib.rs".into()
            })
        );
    }
}
