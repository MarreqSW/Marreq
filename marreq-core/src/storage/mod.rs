// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! File storage for attachments (issue #241).
//!
//! Files are content-addressed: each one is stored once under its SHA-256 and
//! database rows reference the hash. [`BlobStore`] hides where the bytes live;
//! v1 ships [`local::LocalFsStore`] (a directory, usually a Docker volume).

pub mod content_type;
pub mod local;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

pub use local::LocalFsStore;

const MIB: u64 = 1024 * 1024;
pub const DEFAULT_ATTACHMENTS_DIR: &str = "/var/lib/marreq/attachments";
pub const DEFAULT_MAX_FILE_MB: u64 = 10;
pub const DEFAULT_PROJECT_QUOTA_MB: u64 = 500;

/// Attachment settings, read once at startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentsConfig {
    /// `MARREQ_ATTACHMENTS_DIR`.
    pub dir: PathBuf,
    /// `MARREQ_ATTACHMENT_MAX_MB`, in bytes.
    pub max_file_bytes: u64,
    /// `MARREQ_PROJECT_STORAGE_QUOTA_MB`, in bytes. Instance admins can
    /// override it per project.
    pub default_project_quota_bytes: u64,
}

impl Default for AttachmentsConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(DEFAULT_ATTACHMENTS_DIR),
            max_file_bytes: DEFAULT_MAX_FILE_MB * MIB,
            default_project_quota_bytes: DEFAULT_PROJECT_QUOTA_MB * MIB,
        }
    }
}

impl AttachmentsConfig {
    /// Read from the environment, pushing a message to `issues` for each bad value.
    pub fn from_env(issues: &mut Vec<String>) -> Self {
        Self::from_lookup(|name| std::env::var(name).ok(), issues)
    }

    fn from_lookup(get: impl Fn(&str) -> Option<String>, issues: &mut Vec<String>) -> Self {
        let defaults = Self::default();
        let dir = get("MARREQ_ATTACHMENTS_DIR")
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or(defaults.dir);
        let mut megabytes = |name: &str, default: u64| match get(name) {
            None => default * MIB,
            Some(raw) => match raw.trim().parse::<u64>() {
                Ok(mb) if mb > 0 && mb <= 1024 * 1024 => mb * MIB,
                _ => {
                    issues.push(format!(
                        "{name} must be a whole number of megabytes between 1 and 1048576"
                    ));
                    default * MIB
                }
            },
        };
        let max_file_bytes = megabytes("MARREQ_ATTACHMENT_MAX_MB", DEFAULT_MAX_FILE_MB);
        let default_project_quota_bytes =
            megabytes("MARREQ_PROJECT_STORAGE_QUOTA_MB", DEFAULT_PROJECT_QUOTA_MB);
        Self {
            dir,
            max_file_bytes,
            default_project_quota_bytes,
        }
    }

    /// Request body limit Rocket needs so a maximum-size file fits
    /// (multipart framing and the other form fields get 1 MiB).
    pub fn request_limit_bytes(&self) -> u64 {
        self.max_file_bytes + MIB
    }
}

/// A hashed upload waiting to be committed into the store.
#[derive(Debug)]
pub struct StagedBlob {
    pub sha256: String,
    pub size: u64,
    pub(crate) tmp_path: PathBuf,
}

/// Where attachment bytes live. Operations are blocking; call them from
/// `spawn_blocking` on request paths.
pub trait BlobStore: Send + Sync {
    /// A fresh path in the store's scratch area for an incoming upload. It is
    /// on the same filesystem as the blobs, so committing is a rename.
    fn new_upload_path(&self) -> io::Result<PathBuf>;
    /// Hash (and fsync) an upload written to `tmp`. Takes ownership of the file.
    fn stage(&self, tmp: &Path) -> io::Result<StagedBlob>;
    /// Move a staged upload into place; a no-op (besides removing the temp
    /// file) when the same content is already stored.
    fn commit(&self, staged: StagedBlob) -> io::Result<()>;
    /// Drop a staged upload without storing it.
    fn discard(&self, staged: StagedBlob);
    fn open(&self, sha256: &str) -> io::Result<std::fs::File>;
    fn delete(&self, sha256: &str) -> io::Result<()>;
}

/// Rocket managed state: the blob store plus limits.
pub struct AttachmentStorage {
    store: Arc<dyn BlobStore>,
    pub config: AttachmentsConfig,
    /// Serialises "commit a blob" against "delete an unused blob", so a file
    /// that a concurrent upload just referenced is never removed.
    blob_lock: Mutex<()>,
}

impl AttachmentStorage {
    pub fn new(store: Arc<dyn BlobStore>, config: AttachmentsConfig) -> Self {
        Self {
            store,
            config,
            blob_lock: Mutex::new(()),
        }
    }

    /// Local-disk storage at `config.dir`. Nothing touches the disk until the
    /// first upload; call [`LocalFsStore::prepare`] at startup to check it.
    pub fn local(config: AttachmentsConfig) -> Self {
        Self::new(Arc::new(LocalFsStore::new(config.dir.clone())), config)
    }

    pub fn store(&self) -> &dyn BlobStore {
        self.store.as_ref()
    }

    pub fn commit(&self, staged: StagedBlob) -> io::Result<()> {
        let _guard = self.blob_lock.lock().unwrap_or_else(|e| e.into_inner());
        self.store.commit(staged)
    }

    /// Delete the blob unless `in_use` (checked under the lock) says a row
    /// still references it.
    pub fn delete_if_unused(
        &self,
        sha256: &str,
        in_use: impl FnOnce() -> bool,
    ) -> io::Result<bool> {
        let _guard = self.blob_lock.lock().unwrap_or_else(|e| e.into_inner());
        if in_use() {
            return Ok(false);
        }
        self.store.delete(sha256)?;
        Ok(true)
    }
}

static INSTALLED: OnceLock<Arc<AttachmentStorage>> = OnceLock::new();

/// Make the storage reachable from services that run outside a request with
/// Rocket state (e.g. removing files when a requirement is deleted). Later
/// calls keep the first instance.
pub fn install(storage: Arc<AttachmentStorage>) -> Arc<AttachmentStorage> {
    Arc::clone(INSTALLED.get_or_init(|| storage))
}

pub fn installed() -> Option<&'static Arc<AttachmentStorage>> {
    INSTALLED.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn parse(vars: &[(&str, &str)]) -> (AttachmentsConfig, Vec<String>) {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let mut issues = Vec::new();
        let cfg = AttachmentsConfig::from_lookup(|k| map.get(k).cloned(), &mut issues);
        (cfg, issues)
    }

    #[test]
    fn defaults_are_10_mb_per_file_and_500_mb_per_project() {
        let (cfg, issues) = parse(&[]);
        assert!(issues.is_empty());
        assert_eq!(cfg, AttachmentsConfig::default());
        assert_eq!(cfg.max_file_bytes, 10 * MIB);
        assert_eq!(cfg.default_project_quota_bytes, 500 * MIB);
        assert_eq!(cfg.dir, PathBuf::from("/var/lib/marreq/attachments"));
        assert_eq!(cfg.request_limit_bytes(), 11 * MIB);
    }

    #[test]
    fn reads_overrides() {
        let (cfg, issues) = parse(&[
            ("MARREQ_ATTACHMENTS_DIR", "/data/files"),
            ("MARREQ_ATTACHMENT_MAX_MB", "25"),
            ("MARREQ_PROJECT_STORAGE_QUOTA_MB", " 2048 "),
        ]);
        assert!(issues.is_empty());
        assert_eq!(cfg.dir, PathBuf::from("/data/files"));
        assert_eq!(cfg.max_file_bytes, 25 * MIB);
        assert_eq!(cfg.default_project_quota_bytes, 2048 * MIB);
    }

    #[test]
    fn reports_invalid_values() {
        let (cfg, issues) = parse(&[
            ("MARREQ_ATTACHMENT_MAX_MB", "0"),
            ("MARREQ_PROJECT_STORAGE_QUOTA_MB", "lots"),
        ]);
        assert_eq!(issues.len(), 2);
        assert!(issues[0].contains("MARREQ_ATTACHMENT_MAX_MB"));
        assert!(issues[1].contains("MARREQ_PROJECT_STORAGE_QUOTA_MB"));
        assert_eq!(cfg.max_file_bytes, 10 * MIB);
    }
}
