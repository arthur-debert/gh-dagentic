use anyhow::Result;
use std::path::Path;

pub trait Filesystem {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    fn write_file(&self, path: &Path, contents: &[u8]) -> Result<()>;
    fn file_exists(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
}

pub struct RealFs;

impl Filesystem for RealFs {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        Ok(std::fs::read(path)?)
    }

    fn write_file(&self, path: &Path, contents: &[u8]) -> Result<()> {
        Ok(std::fs::write(path, contents)?)
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        Ok(std::fs::create_dir_all(path)?)
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    pub struct FakeFs {
        pub files: RefCell<HashMap<PathBuf, Vec<u8>>>,
        pub dirs: RefCell<Vec<PathBuf>>,
    }

    impl FakeFs {
        pub fn new() -> Self {
            Self {
                files: RefCell::new(HashMap::new()),
                dirs: RefCell::new(Vec::new()),
            }
        }

        pub fn with_file(self, path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) -> Self {
            self.files.borrow_mut().insert(path.into(), contents.into());
            self
        }
    }

    impl Filesystem for FakeFs {
        fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
            self.files
                .borrow()
                .get(path)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("file not found: {}", path.display()))
        }

        fn write_file(&self, path: &Path, contents: &[u8]) -> Result<()> {
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), contents.to_vec());
            Ok(())
        }

        fn file_exists(&self, path: &Path) -> bool {
            self.files.borrow().contains_key(path)
        }

        fn create_dir_all(&self, path: &Path) -> Result<()> {
            self.dirs.borrow_mut().push(path.to_path_buf());
            Ok(())
        }
    }
}
