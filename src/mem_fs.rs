use std::{
    collections::BTreeMap,
    ffi::OsString,
    fmt,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{utils::PathUtil, Error, LogixVfs, LogixVfsDirEntry};

#[derive(Clone, PartialEq)]
enum FileData {
    Static(&'static [u8]),
    Arc(Arc<[u8]>),
}

impl fmt::Debug for FileData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Static(buf) => f.debug_struct("Static").field("size", &buf.len()).finish(),
            Self::Arc(buf) => f.debug_struct("Arc").field("size", &buf.len()).finish(),
        }
    }
}

#[derive(Default, Debug)]
enum Entry {
    #[default]
    Empty,
    File(FileData),
    Dir(BTreeMap<OsString, Entry>),
}

#[derive(Default, Debug)]
pub struct MemFs {
    root: Entry,
}

impl MemFs {
    fn resolve_node_mut(&mut self, path: PathBuf, create_path: bool) -> Result<&mut Entry, Error> {
        use std::path::Component;

        let mut cur = &mut self.root;

        for (i, component) in path.components().enumerate() {
            match component {
                Component::RootDir => (),
                Component::Prefix(_) | Component::CurDir | Component::ParentDir => {
                    debug_assert!(false, "Should be unreachable ({path:?})");
                    return Err(Error::Other(format!(
                        "Internal error: path {path:?} is not canonicalized",
                    )));
                }
                Component::Normal(name) => 'retry_cur: loop {
                    match cur {
                        Entry::Empty => {
                            if create_path {
                                *cur = Entry::Dir([(name.to_owned(), Entry::Empty)].into());
                                continue 'retry_cur;
                            } else {
                                return Err(Error::NotFound { path });
                            }
                        }
                        Entry::File(_) => {
                            let dir: PathBuf = path.components().take(i).collect();
                            return Err(Error::Other(format!(
                                "Cannot create directory {dir:?} as it is a file for {path:?}"
                            )));
                        }
                        Entry::Dir(map) => cur = map.entry(name.to_owned()).or_default(),
                    }
                    break;
                },
            }
        }

        Ok(cur)
    }

    fn resolve_node(&self, path: PathBuf) -> Result<(PathBuf, &Entry), Error> {
        use std::path::Component;

        let mut cur = &self.root;

        for (i, component) in path.components().enumerate() {
            match component {
                Component::RootDir => (),
                Component::Prefix(_) | Component::CurDir | Component::ParentDir => {
                    debug_assert!(false, "Should be unreachable ({path:?})");
                    return Err(Error::Other(format!(
                        "Internal error: path {path:?} is not canonicalized",
                    )));
                }
                Component::Normal(name) => match cur {
                    Entry::Empty => return Err(Error::NotFound { path }),
                    Entry::File(_) => {
                        let dir: PathBuf = path.components().take(i).collect();
                        return Err(Error::NotADirectory { path: dir });
                    }
                    Entry::Dir(map) => {
                        if let Some(entry) = map.get(name) {
                            cur = entry;
                        } else {
                            return Err(Error::NotFound { path });
                        }
                    }
                },
            }
        }

        Ok((path, cur))
    }

    fn resolve_path(&self, path: impl AsRef<Path>) -> Result<PathBuf, Error> {
        PathUtil {
            root: "/".as_ref(),
            cur_dir: "/".as_ref(),
        }
        .resolve_path(false, path.as_ref())
    }

    pub fn set_static_file(
        &mut self,
        path: impl AsRef<Path>,
        data: &'static [u8],
        create_dir: bool,
    ) -> Result<(), Error> {
        let path = path.as_ref();
        let node = self.resolve_node_mut(self.resolve_path(path)?, create_dir)?;
        match node {
            Entry::Empty | Entry::File(_) => {
                *node = Entry::File(FileData::Static(data));
                Ok(())
            }
            Entry::Dir(_) => Err(Error::Other(format!(
                "Can't overwrite directory with a file at {path:?}"
            ))),
        }
    }

    pub fn set_file(
        &mut self,
        path: impl AsRef<Path>,
        data: impl Into<Arc<[u8]>>,
        create_dir: bool,
    ) -> Result<(), Error> {
        let path = path.as_ref();
        let node = self.resolve_node_mut(self.resolve_path(path)?, create_dir)?;
        match node {
            Entry::Empty | Entry::File(_) => {
                *node = Entry::File(FileData::Arc(data.into()));
                Ok(())
            }
            Entry::Dir(_) => Err(Error::Other(format!(
                "Can't overwrite directory with a file at {path:?}"
            ))),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MemFileData(FileData);

impl AsRef<[u8]> for MemFileData {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            FileData::Static(buf) => buf,
            FileData::Arc(buf) => buf,
        }
    }
}

pub type MemFile = Cursor<MemFileData>;

enum DirEntryType {
    File,
    Dir,
}

pub struct DirEntry {
    path: PathBuf,
    ty: DirEntryType,
}

impl LogixVfsDirEntry for DirEntry {
    fn path(&self) -> &Path {
        &self.path
    }

    fn is_dir(&self) -> bool {
        match self.ty {
            DirEntryType::File => false,
            DirEntryType::Dir => true,
        }
    }

    fn is_file(&self) -> bool {
        match self.ty {
            DirEntryType::File => true,
            DirEntryType::Dir => false,
        }
    }

    fn is_symlink(&self) -> bool {
        false
    }
}

pub struct ReadDir {
    it: std::vec::IntoIter<DirEntry>,
}

impl ReadDir {
    fn new(base: &Path, map: &BTreeMap<OsString, Entry>) -> Self {
        let list: Vec<_> = map
            .iter()
            .filter_map(|(k, v)| {
                let ty = match v {
                    Entry::Empty => return None,
                    Entry::File(_) => DirEntryType::File,
                    Entry::Dir(_) => DirEntryType::Dir,
                };

                Some(DirEntry {
                    path: base.join(k),
                    ty,
                })
            })
            .collect();
        ReadDir {
            it: list.into_iter(),
        }
    }
}

impl Iterator for ReadDir {
    type Item = Result<DirEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.it.next().map(Ok)
    }
}

impl LogixVfs for MemFs {
    type RoFile = Cursor<MemFileData>;
    type DirEntry = DirEntry;
    type ReadDir = ReadDir;

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, crate::Error> {
        self.resolve_path(path)
    }

    fn open_file(&self, path: &Path) -> Result<Self::RoFile, crate::Error> {
        match self.resolve_node(self.resolve_path(path)?)? {
            (_, Entry::Empty) => Err(Error::NotFound {
                path: path.to_path_buf(),
            }),
            (_, Entry::File(data)) => Ok(Cursor::new(MemFileData(data.clone()))),
            (_, Entry::Dir(_)) => Err(Error::Other(format!("The path {path:?} is not a file"))),
        }
    }

    fn read_dir(&self, path: &Path) -> Result<Self::ReadDir, crate::Error> {
        match self.resolve_node(self.resolve_path(path)?)? {
            (_, Entry::Empty) => Err(Error::NotFound {
                path: path.to_path_buf(),
            }),
            (path, Entry::File(_)) => Err(Error::NotADirectory { path }),
            (path, Entry::Dir(map)) => Ok(ReadDir::new(&path, map)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let mut fs = MemFs::default();
        let hello_rs = b"fn hello() -> i32 {{\n42}}\n".as_slice();
        let world_rs = b"fn world() -> i32 {{\n1337}}\n".as_slice();

        assert_eq!(
            fs.set_static_file("/src/hello.rs", hello_rs, false)
                .unwrap_err(),
            Error::NotFound {
                path: "/src/hello.rs".into()
            }
        );

        fs.set_static_file("/src/hello.rs", hello_rs, true).unwrap();
        fs.set_static_file("/src/world.rs", world_rs, true).unwrap();

        assert_eq!(
            fs.set_static_file("/src/hello.rs/world.rs", hello_rs, false)
                .unwrap_err(),
            Error::Other(
                "Cannot create directory \"/src/hello.rs\" as it is a file for \"/src/hello.rs/world.rs\"".into()
            )
        );

        assert_eq!(
            fs.open_file("/src/hello.rs".as_ref()).unwrap().get_ref().0,
            FileData::Static(hello_rs)
        );

        assert_eq!(
            fs.open_file("/src".as_ref()).unwrap_err(),
            Error::Other("The path \"/src\" is not a file".to_owned())
        );

        assert_eq!(
            fs.open_file("/src/hello.rs/world.rs".as_ref()).unwrap_err(),
            Error::NotADirectory {
                path: "/src/hello.rs".into()
            }
        );

        {
            let mut it = fs.read_dir("/".as_ref()).unwrap();
            let entry = it.next().unwrap().unwrap();
            assert!(it.next().is_none());

            assert_eq!(entry.path(), Path::new("/src"),);
            assert!(entry.is_dir());
            assert!(!entry.is_file());
        }

        {
            let mut it = fs.read_dir("/src".as_ref()).unwrap();
            let file1 = it.next().unwrap().unwrap();
            let file2 = it.next().unwrap().unwrap();
            assert!(it.next().is_none());

            assert_eq!(file1.path(), Path::new("/src/hello.rs"),);
            assert_eq!(file2.path(), Path::new("/src/world.rs"),);

            assert!(!file1.is_dir());
            assert!(!file2.is_dir());

            assert!(file1.is_file());
            assert!(file2.is_file());
        }
    }
}
