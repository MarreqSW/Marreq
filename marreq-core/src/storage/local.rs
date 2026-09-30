// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Content-addressed blobs in a local directory:
//!
//! ```text
//! <dir>/tmp/upload-<random>      incoming uploads
//! <dir>/ab/cd/abcd…(64 hex)      stored files
//! ```

use super::{BlobStore, StagedBlob};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub struct LocalFsStore {
    root: PathBuf,
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn invalid_hash() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid blob hash")
}

impl LocalFsStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }

    fn blob_path(&self, sha256: &str) -> io::Result<PathBuf> {
        if !is_sha256(sha256) {
            return Err(invalid_hash());
        }
        Ok(self
            .root
            .join(&sha256[0..2])
            .join(&sha256[2..4])
            .join(sha256))
    }

    /// Create the directories, check they are writable and remove uploads
    /// left behind by a crash. Called once at startup.
    pub fn prepare(&self) -> io::Result<()> {
        let tmp = self.tmp_dir();
        fs::create_dir_all(&tmp)?;
        for entry in fs::read_dir(&tmp)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let _ = fs::remove_file(entry.path());
            }
        }
        let probe = tmp.join(".write-test");
        fs::write(&probe, b"ok")?;
        fs::remove_file(probe)
    }
}

impl BlobStore for LocalFsStore {
    fn new_upload_path(&self) -> io::Result<PathBuf> {
        let tmp = self.tmp_dir();
        fs::create_dir_all(&tmp)?;
        let suffix: u64 = rand::random();
        Ok(tmp.join(format!("upload-{suffix:016x}")))
    }

    fn stage(&self, tmp: &Path) -> io::Result<StagedBlob> {
        let mut file = fs::File::open(tmp)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 64 * 1024];
        let mut size = 0u64;
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            size += n as u64;
        }
        file.sync_all()?;
        let sha256 = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Ok(StagedBlob {
            sha256,
            size,
            tmp_path: tmp.to_path_buf(),
        })
    }

    fn commit(&self, staged: StagedBlob) -> io::Result<()> {
        let dest = self.blob_path(&staged.sha256)?;
        if dest.exists() {
            let _ = fs::remove_file(&staged.tmp_path);
            return Ok(());
        }
        let parent = dest.parent().expect("blob path has a parent");
        fs::create_dir_all(parent)?;
        fs::rename(&staged.tmp_path, &dest)?;
        // Persist the new directory entry too.
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    }

    fn discard(&self, staged: StagedBlob) {
        let _ = fs::remove_file(staged.tmp_path);
    }

    fn open(&self, sha256: &str) -> io::Result<fs::File> {
        fs::File::open(self.blob_path(sha256)?)
    }

    fn delete(&self, sha256: &str) -> io::Result<()> {
        match fs::remove_file(self.blob_path(sha256)?) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let suffix: u64 = rand::random();
        let dir = std::env::temp_dir().join(format!("marreq-blob-test-{name}-{suffix:016x}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn upload(store: &LocalFsStore, bytes: &[u8]) -> StagedBlob {
        let path = store.new_upload_path().unwrap();
        fs::write(&path, bytes).unwrap();
        store.stage(&path).unwrap()
    }

    const HELLO_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    #[test]
    fn stores_by_hash_in_two_level_directories() {
        let root = temp_root("layout");
        let store = LocalFsStore::new(root.clone());
        let staged = upload(&store, b"hello");
        assert_eq!(staged.sha256, HELLO_SHA);
        assert_eq!(staged.size, 5);
        store.commit(staged).unwrap();

        let path = root.join("2c").join("f2").join(HELLO_SHA);
        assert_eq!(fs::read(&path).unwrap(), b"hello");
        let mut contents = String::new();
        store
            .open(HELLO_SHA)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert_eq!(contents, "hello");
        assert_eq!(fs::read_dir(root.join("tmp")).unwrap().count(), 0);

        store.delete(HELLO_SHA).unwrap();
        assert!(!path.exists());
        // Deleting a missing blob is not an error.
        store.delete(HELLO_SHA).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn same_content_is_stored_once() {
        let root = temp_root("dedup");
        let store = LocalFsStore::new(root.clone());
        store.commit(upload(&store, b"hello")).unwrap();
        let second = upload(&store, b"hello");
        let tmp = second.tmp_path.clone();
        store.commit(second).unwrap();
        assert!(!tmp.exists(), "duplicate upload's temp file is removed");
        assert_eq!(fs::read_dir(root.join("2c").join("f2")).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn discard_removes_the_upload() {
        let root = temp_root("discard");
        let store = LocalFsStore::new(root.clone());
        let staged = upload(&store, b"hello");
        let tmp = staged.tmp_path.clone();
        store.discard(staged);
        assert!(!tmp.exists());
        assert!(store.open(HELLO_SHA).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prepare_cleans_leftover_uploads() {
        let root = temp_root("prepare");
        let store = LocalFsStore::new(root.clone());
        let leftover = store.new_upload_path().unwrap();
        fs::write(&leftover, b"partial").unwrap();
        store.prepare().unwrap();
        assert!(!leftover.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_hashes_that_could_escape_the_directory() {
        let store = LocalFsStore::new(PathBuf::from("/nonexistent"));
        for bad in ["../../etc/passwd", "", &"A".repeat(64), &"g".repeat(64)] {
            assert_eq!(
                store.open(bad).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }
}
