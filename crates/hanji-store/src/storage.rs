//! Where documents are kept: a small key–value trait, so the operations stay
//! free of I/O (DESIGN.md §10.4) and one build serves a server, a CLI and a
//! wasm client. [`MemStorage`] keeps everything in memory; [`FsStorage`]
//! (native builds) keeps one file per key under a directory.
//!
//! Keys are `/`-separated paths of document ids (checked by the caller)
//! and fixed names, never user paths.

use std::collections::BTreeMap;

pub trait Storage {
    /// The value at `key`, or `None` when there is none.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String>;
    /// Store `data` at `key`, replacing what was there.
    fn put(&mut self, key: &str, data: &[u8]) -> Result<(), String>;
    /// The keys that start with `prefix`, sorted.
    fn keys(&self, prefix: &str) -> Result<Vec<String>, String>;
}

#[derive(Clone, Debug, Default)]
pub struct MemStorage(BTreeMap<String, Vec<u8>>);

impl MemStorage {
    pub fn new() -> MemStorage {
        MemStorage::default()
    }
}

impl Storage for MemStorage {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(self.0.get(key).cloned())
    }

    fn put(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        self.0.insert(key.to_string(), data.to_vec());
        Ok(())
    }

    fn keys(&self, prefix: &str) -> Result<Vec<String>, String> {
        Ok(self
            .0
            .range(prefix.to_string()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, _)| k.clone())
            .collect())
    }
}

#[cfg(not(target_family = "wasm"))]
pub use fs::FsStorage;

#[cfg(not(target_family = "wasm"))]
mod fs {
    use std::path::{Path, PathBuf};

    use super::Storage;

    /// One file per key under `root`. A value is written to a temporary
    /// file and renamed into place, so a reader sees the old or the new
    /// value, never half of one. One writer at a time (no locking yet).
    #[derive(Clone, Debug)]
    pub struct FsStorage {
        root: PathBuf,
    }

    impl FsStorage {
        pub fn new(root: impl Into<PathBuf>) -> FsStorage {
            FsStorage { root: root.into() }
        }

        pub fn root(&self) -> &Path {
            &self.root
        }

        fn path(&self, key: &str) -> PathBuf {
            key.split('/').fold(self.root.clone(), |p, seg| p.join(seg))
        }
    }

    fn err(what: &str, p: &Path, e: std::io::Error) -> String {
        format!("{what} {}: {e}", p.display())
    }

    impl Storage for FsStorage {
        fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
            let p = self.path(key);
            match std::fs::read(&p) {
                Ok(d) => Ok(Some(d)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(err("cannot read", &p, e)),
            }
        }

        fn put(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
            let p = self.path(key);
            if let Some(dir) = p.parent() {
                std::fs::create_dir_all(dir).map_err(|e| err("cannot create", dir, e))?;
            }
            let tmp = p.with_extension("tmp");
            std::fs::write(&tmp, data).map_err(|e| err("cannot write", &tmp, e))?;
            std::fs::rename(&tmp, &p).map_err(|e| err("cannot write", &p, e))
        }

        fn keys(&self, prefix: &str) -> Result<Vec<String>, String> {
            let mut out = vec![];
            let mut stack = vec![(self.root.clone(), String::new())];
            while let Some((dir, rel)) = stack.pop() {
                let rd = match std::fs::read_dir(&dir) {
                    Ok(rd) => rd,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(e) => return Err(err("cannot list", &dir, e)),
                };
                for ent in rd {
                    let ent = ent.map_err(|e| err("cannot list", &dir, e))?;
                    let name = ent.file_name().to_string_lossy().into_owned();
                    let key = if rel.is_empty() { name } else { format!("{rel}/{name}") };
                    if ent.path().is_dir() {
                        stack.push((ent.path(), key));
                    } else if key.starts_with(prefix) && !key.ends_with(".tmp") {
                        out.push(key);
                    }
                }
            }
            out.sort();
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exercise(s: &mut dyn Storage) {
        assert_eq!(s.get("docs/a/doc.json").unwrap(), None);
        s.put("docs/a/doc.json", b"1").unwrap();
        s.put("docs/a/r1.txt", b"t").unwrap();
        s.put("docs/b/doc.json", b"2").unwrap();
        s.put("docs/a/doc.json", b"3").unwrap();
        assert_eq!(s.get("docs/a/doc.json").unwrap().as_deref(), Some(&b"3"[..]));
        assert_eq!(s.keys("docs/a/").unwrap(), ["docs/a/doc.json", "docs/a/r1.txt"]);
        assert_eq!(s.keys("docs/").unwrap().len(), 3);
    }

    #[test]
    fn memory() {
        exercise(&mut MemStorage::new());
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn filesystem() {
        let dir = std::env::temp_dir().join(format!("hanji-store-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        exercise(&mut FsStorage::new(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
