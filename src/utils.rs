use std::path::{Component, Path, PathBuf};

use crate::{Error, LogixVfsDirEntry};

pub(crate) struct PathUtil<'a> {
    pub cur_dir: &'a Path,
    pub root: &'a Path,
}

impl<'a> PathUtil<'a> {
    pub fn resolve_path(&self, relative: bool, path: &Path) -> Result<PathBuf, Error> {
        let mut ret = if relative {
            self.cur_dir.to_path_buf()
        } else {
            self.root.join(self.cur_dir)
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
                        return Err(Error::PathOutsideBounds {
                            path: path.to_path_buf(),
                        });
                    }
                    level -= 1;
                    ret.pop();
                }
                Component::Prefix(prefix) => {
                    // NOTE(2024.02): Should never happen on platforms other than Windows
                    return Err(Error::Other(format!(
                        "Unknown prefix {:?}",
                        prefix.as_os_str()
                    )));
                }
            }
        }

        Ok(ret)
    }
}

impl LogixVfsDirEntry for PathBuf {
    fn path(&self) -> &Path {
        self
    }

    fn is_dir(&self) -> bool {
        Path::is_dir(self)
    }

    fn is_file(&self) -> bool {
        Path::is_file(self)
    }

    fn is_symlink(&self) -> bool {
        Path::is_symlink(self)
    }
}
