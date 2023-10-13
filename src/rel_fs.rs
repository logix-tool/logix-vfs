use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::LogixVfs;

#[derive(Debug, PartialEq)]
pub enum Error {
    OutsideRoot,
}

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
        use std::path::Component;

        let path = path.as_ref();
        let mut ret = if relative {
            self.cur_dir.clone()
        } else {
            self.root.join(&self.cur_dir)
        };

        let mut level = self.cur_dir.components().count();

        for cur in path.components() {
            match cur {
                Component::Normal(name) => {
                    level += 1;
                    ret.push(name);
                }
                Component::RootDir => {
                    while level > 0 {
                        level -= 1;
                        ret.pop();
                    }
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    if level == 0 {
                        return Err(Error::OutsideRoot);
                    }
                    level -= 1;
                    ret.pop();
                }
                _ => todo!("{path:?} => {cur:?}"),
            }
        }

        Ok(ret)
    }
}

impl LogixVfs for RelFs {
    type RoFile = File;

    fn open_file(&self, path: &Path) -> Result<Self::RoFile, crate::Error> {
        match self.resolve_path(false, path) {
            Ok(full_path) => Ok(File::open(full_path)?),
            Err(e) => todo!("{e:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let mut fs = RelFs::new("/home/zeldor");

        assert_eq!(
            fs.resolve_path(false, ".config/awesome-app/config.toml"),
            Ok(PathBuf::from(
                "/home/zeldor/.config/awesome-app/config.toml"
            ))
        );

        assert_eq!(
            fs.resolve_path(true, ".config/awesome-app/config.toml"),
            Ok(PathBuf::from(".config/awesome-app/config.toml"))
        );

        assert_eq!(
            fs.resolve_path(false, ".config/./awesome-app/./config.toml"),
            Ok(PathBuf::from(
                "/home/zeldor/.config/awesome-app/config.toml"
            ))
        );

        assert_eq!(
            fs.resolve_path(true, ".config/./awesome-app/./config.toml"),
            Ok(PathBuf::from(".config/awesome-app/config.toml"))
        );

        assert_eq!(
            fs.resolve_path(false, ".config/./awesome-app/../config.toml"),
            Ok(PathBuf::from("/home/zeldor/.config/config.toml"))
        );

        assert_eq!(
            fs.resolve_path(true, ".config/./awesome-app/../config.toml"),
            Ok(PathBuf::from(".config/config.toml"))
        );

        assert_eq!(fs.chdir(".config"), Ok(Path::new(".config")));

        assert_eq!(
            fs.chdir("awesome-app"),
            Ok(Path::new(".config/awesome-app"))
        );

        assert_eq!(
            fs.resolve_path(false, "config.toml"),
            Ok(PathBuf::from(
                "/home/zeldor/.config/awesome-app/config.toml"
            ))
        );

        assert_eq!(
            fs.resolve_path(true, "config.toml"),
            Ok(PathBuf::from(".config/awesome-app/config.toml"))
        );

        assert_eq!(
            fs.resolve_path(false, "./config.toml"),
            Ok(PathBuf::from(
                "/home/zeldor/.config/awesome-app/config.toml"
            ))
        );

        assert_eq!(
            fs.resolve_path(true, "./config.toml"),
            Ok(PathBuf::from(".config/awesome-app/config.toml"))
        );

        assert_eq!(
            fs.resolve_path(false, "../config.toml"),
            Ok(PathBuf::from("/home/zeldor/.config/config.toml"))
        );

        assert_eq!(
            fs.resolve_path(true, "../config.toml"),
            Ok(PathBuf::from(".config/config.toml"))
        );

        assert_eq!(
            fs.resolve_path(false, "/.bashrc"),
            Ok(PathBuf::from("/home/zeldor/.bashrc"))
        );

        assert_eq!(
            fs.resolve_path(true, "/.bashrc"),
            Ok(PathBuf::from(".bashrc"))
        );
    }
}
