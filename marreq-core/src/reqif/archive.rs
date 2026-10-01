// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! ReqIFZ archives (issue #343): a ZIP file holding one or more `.reqif`
//! documents plus the files their XHTML `<object data=…>` elements refer to.
//!
//! Reading is defensive: every entry name is checked before anything is
//! extracted (no absolute paths, no `..`, no symlinks, no duplicates), and
//! sizes are capped while reading, not taken from the headers.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Limits applied when opening and reading an archive.
#[derive(Debug, Clone, Copy)]
pub struct ArchiveLimits {
    pub max_entries: usize,
    /// Sum of the declared uncompressed sizes.
    pub max_total_uncompressed: u64,
    /// Highest uncompressed/compressed ratio allowed for entries above `ratio_min_size`.
    pub max_ratio: u64,
    pub ratio_min_size: u64,
    /// Largest `.reqif` document read into memory.
    pub max_document_bytes: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_total_uncompressed: 1024 * 1024 * 1024,
            max_ratio: 100,
            ratio_min_size: 1024 * 1024,
            max_document_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArchiveError {
    #[error("the file is not a valid ZIP archive: {0}")]
    Invalid(String),
    #[error("the archive contains an unsafe path: {0}")]
    UnsafePath(String),
    #[error("the archive contains an encrypted entry: {0}")]
    Encrypted(String),
    #[error("the archive contains a symbolic link: {0}")]
    Symlink(String),
    #[error("the archive contains the same path twice: {0}")]
    Duplicate(String),
    #[error("the archive has more than {0} entries")]
    TooManyEntries(usize),
    #[error("the archive expands to more than {0} MB")]
    TooLargeTotal(u64),
    #[error("the archive entry {0} is compressed suspiciously well (possible zip bomb)")]
    Bomb(String),
    #[error("the archive contains no .reqif document")]
    NoDocument,
    #[error("{0} is not in the archive")]
    NotFound(String),
    #[error("{name} is larger than {max_bytes} bytes")]
    TooLarge { name: String, max_bytes: u64 },
    #[error("could not read the archive: {0}")]
    Io(String),
}

impl From<zip::result::ZipError> for ArchiveError {
    fn from(e: zip::result::ZipError) -> Self {
        ArchiveError::Invalid(e.to_string())
    }
}

impl From<io::Error> for ArchiveError {
    fn from(e: io::Error) -> Self {
        ArchiveError::Io(e.to_string())
    }
}

/// Normalise an entry name to `a/b/c`, rejecting anything that could point
/// outside the archive. `None` means "unsafe".
fn normalize_entry_name(raw: &str) -> Option<String> {
    let unified = raw.replace('\\', "/");
    if unified.starts_with('/') {
        return None;
    }
    let mut parts = Vec::new();
    for part in unified.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            p if p.contains(':') || p.chars().any(char::is_control) => return None,
            p => parts.push(p),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Resolve `relative` against `base_dir` (both `/`-separated, already
/// normalised). `None` when `..` climbs out of the archive.
fn join_relative(base_dir: &str, relative: &str) -> Option<String> {
    let mut parts: Vec<&str> = base_dir.split('/').filter(|p| !p.is_empty()).collect();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn is_reqif_document(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".reqif") && !lower.starts_with("__macosx/")
}

/// An opened, validated ReqIFZ archive.
pub struct ReqifArchive {
    zip: ZipArchive<File>,
    limits: ArchiveLimits,
    /// normalised name -> entry index
    entries: HashMap<String, usize>,
    /// lower-cased normalised name -> normalised name
    folded: HashMap<String, String>,
    documents: Vec<String>,
}

impl ReqifArchive {
    pub fn open(path: &Path, limits: ArchiveLimits) -> Result<Self, ArchiveError> {
        let mut zip = ZipArchive::new(File::open(path)?)?;
        if zip.len() > limits.max_entries {
            return Err(ArchiveError::TooManyEntries(limits.max_entries));
        }
        let mut entries = HashMap::new();
        let mut folded = HashMap::new();
        let mut documents = Vec::new();
        let mut total: u64 = 0;
        for index in 0..zip.len() {
            let file = zip.by_index_raw(index)?;
            let raw = file.name().to_string();
            if file.is_dir() {
                if normalize_entry_name(&raw).is_none() && !raw.trim_matches('/').is_empty() {
                    return Err(ArchiveError::UnsafePath(raw));
                }
                continue;
            }
            let name = normalize_entry_name(&raw)
                .filter(|_| file.enclosed_name().is_some())
                .ok_or_else(|| ArchiveError::UnsafePath(raw.clone()))?;
            if file.is_symlink() {
                return Err(ArchiveError::Symlink(raw));
            }
            if file.encrypted() {
                return Err(ArchiveError::Encrypted(raw));
            }
            let size = file.size();
            if size > limits.ratio_min_size
                && size > file.compressed_size().saturating_mul(limits.max_ratio)
            {
                return Err(ArchiveError::Bomb(raw));
            }
            total = total.saturating_add(size);
            if total > limits.max_total_uncompressed {
                return Err(ArchiveError::TooLargeTotal(
                    limits.max_total_uncompressed / (1024 * 1024),
                ));
            }
            if folded.insert(name.to_lowercase(), name.clone()).is_some() {
                return Err(ArchiveError::Duplicate(raw));
            }
            if is_reqif_document(&name) {
                documents.push(name.clone());
            }
            entries.insert(name, index);
        }
        if documents.is_empty() {
            return Err(ArchiveError::NoDocument);
        }
        Ok(Self {
            zip,
            limits,
            entries,
            folded,
            documents,
        })
    }

    /// `.reqif` documents in archive order.
    pub fn documents(&self) -> &[String] {
        &self.documents
    }

    fn index_of(&self, name: &str) -> Result<usize, ArchiveError> {
        self.entries
            .get(name)
            .copied()
            .ok_or_else(|| ArchiveError::NotFound(name.to_string()))
    }

    /// Read a document into memory (at most `max_document_bytes`).
    pub fn read_document(&mut self, name: &str) -> Result<Vec<u8>, ArchiveError> {
        let index = self.index_of(name)?;
        let max = self.limits.max_document_bytes;
        let mut out = Vec::new();
        self.zip
            .by_index(index)?
            .take(max + 1)
            .read_to_end(&mut out)?;
        if out.len() as u64 > max {
            return Err(ArchiveError::TooLarge {
                name: name.to_string(),
                max_bytes: max,
            });
        }
        Ok(out)
    }

    /// Extract one entry to `dest` (created; must not exist). Fails, and
    /// removes `dest`, when the entry is larger than `max_bytes`.
    pub fn extract_to(
        &mut self,
        name: &str,
        dest: &Path,
        max_bytes: u64,
    ) -> Result<u64, ArchiveError> {
        let index = self.index_of(name)?;
        let result = (|| {
            let mut out = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dest)?;
            let mut entry = self.zip.by_index(index)?;
            let copied = io::copy(&mut (&mut entry).take(max_bytes + 1), &mut out)?;
            if copied > max_bytes {
                return Err(ArchiveError::TooLarge {
                    name: name.to_string(),
                    max_bytes,
                });
            }
            out.flush()?;
            Ok(copied)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(dest);
        }
        result
    }

    /// The entry an XHTML `data` attribute in `document` points to. External
    /// URLs and paths that escape the archive give `None`; the match falls
    /// back to a case-insensitive comparison (archives made on Windows).
    pub fn resolve(&self, document: &str, data: &str) -> Option<String> {
        let data = data.split(['#', '?']).next().unwrap_or("").trim();
        let has_scheme = data
            .split_once(':')
            .is_some_and(|(scheme, _)| !scheme.is_empty() && !scheme.contains('/'));
        if data.is_empty() || has_scheme || data.starts_with('/') || data.starts_with("//") {
            return None;
        }
        let decoded = urlencoding::decode(data).ok()?.replace('\\', "/");
        let base = document.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
        let target = join_relative(base, &decoded)?;
        if self.entries.contains_key(&target) {
            return Some(target);
        }
        self.folded.get(&target.to_lowercase()).cloned()
    }
}

/// One file to place in a ReqIFZ archive.
pub struct ArchiveFile {
    /// Entry name, e.g. `files/12/report.pdf`.
    pub entry: String,
    pub file: File,
    /// Deflate it; already-compressed formats (images, PDF, Office) are stored.
    pub compress: bool,
}

/// Content types whose files are already compressed.
pub fn is_precompressed(content_type: &str) -> bool {
    content_type.starts_with("image/")
        || matches!(content_type, "application/pdf" | "application/zip")
        || content_type.starts_with("application/vnd.openxmlformats")
        || content_type.starts_with("application/vnd.oasis.opendocument")
}

/// Write a ReqIFZ archive: the document first, then the files.
pub fn write_reqifz(
    dest: &Path,
    document_name: &str,
    xml: &[u8],
    files: Vec<ArchiveFile>,
) -> io::Result<()> {
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut zip = ZipWriter::new(File::create(dest)?);
    zip.start_file(document_name, deflated)
        .map_err(io::Error::other)?;
    zip.write_all(xml)?;
    for mut f in files {
        zip.start_file(&f.entry, if f.compress { deflated } else { stored })
            .map_err(io::Error::other)?;
        io::copy(&mut f.file, &mut zip)?;
    }
    zip.finish().map_err(io::Error::other)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let suffix: u64 = rand::random();
            let dir = std::env::temp_dir().join(format!("marreq-reqifz-{name}-{suffix:016x}"));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Build a zip from (name, bytes) pairs with the given method.
    fn zip_of(path: &Path, entries: &[(&str, &[u8])], method: CompressionMethod) {
        let mut zip = ZipWriter::new(File::create(path).unwrap());
        let opts = SimpleFileOptions::default().compression_method(method);
        for (name, bytes) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }

    fn open(path: &Path) -> Result<ReqifArchive, ArchiveError> {
        ReqifArchive::open(path, ArchiveLimits::default())
    }

    #[test]
    fn opens_documents_and_resolves_relative_references() {
        let dir = TempDir::new("open");
        let path = dir.path("a.reqifz");
        zip_of(
            &path,
            &[
                ("export/spec.reqif", b"<REQ-IF/>"),
                ("export/files/plot one.png", b"png"),
                ("export/files/Report.PDF", b"pdf"),
                ("other.reqif", b"<REQ-IF/>"),
                ("__MACOSX/export/._spec.reqif", b"junk"),
            ],
            CompressionMethod::Deflated,
        );
        let mut archive = open(&path).unwrap();
        assert_eq!(archive.documents(), ["export/spec.reqif", "other.reqif"]);
        assert_eq!(
            archive.read_document("export/spec.reqif").unwrap(),
            b"<REQ-IF/>"
        );

        let doc = "export/spec.reqif";
        assert_eq!(
            archive.resolve(doc, "files/plot%20one.png").as_deref(),
            Some("export/files/plot one.png")
        );
        assert_eq!(
            archive
                .resolve(doc, "./files/plot one.png#page=1")
                .as_deref(),
            Some("export/files/plot one.png")
        );
        assert_eq!(
            archive.resolve(doc, "files/report.pdf").as_deref(),
            Some("export/files/Report.PDF"),
            "case-insensitive fallback"
        );
        assert_eq!(
            archive
                .resolve("other.reqif", "export/files/Report.PDF")
                .as_deref(),
            Some("export/files/Report.PDF")
        );
        assert_eq!(
            archive
                .resolve(doc, "../export/files/Report.PDF")
                .as_deref(),
            Some("export/files/Report.PDF")
        );
        for external in [
            "https://example.com/a.png",
            "file:///etc/passwd",
            "data:image/png;base64,AAAA",
            "/etc/passwd",
            "../../etc/passwd",
            "files/missing.png",
            "",
        ] {
            assert_eq!(archive.resolve(doc, external), None, "{external}");
        }
    }

    #[test]
    fn rejects_unsafe_entry_names() {
        let dir = TempDir::new("unsafe");
        for bad in [
            "../evil.reqif",
            "/abs/evil.reqif",
            "a/../../evil.reqif",
            "C:/evil.reqif",
            "a\\..\\..\\x.reqif",
        ] {
            let path = dir.path("bad.reqifz");
            let _ = std::fs::remove_file(&path);
            zip_of(
                &path,
                &[("ok.reqif", b"<REQ-IF/>"), (bad, b"x")],
                CompressionMethod::Stored,
            );
            assert!(
                matches!(open(&path), Err(ArchiveError::UnsafePath(_))),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_symlinks_duplicates_and_archives_without_documents() {
        let dir = TempDir::new("reject");
        let path = dir.path("link.reqifz");
        {
            let mut zip = ZipWriter::new(File::create(&path).unwrap());
            let opts = SimpleFileOptions::default();
            zip.start_file("doc.reqif", opts).unwrap();
            zip.write_all(b"<REQ-IF/>").unwrap();
            zip.add_symlink("files/link", "/etc/passwd", opts).unwrap();
            zip.finish().unwrap();
        }
        assert!(matches!(open(&path), Err(ArchiveError::Symlink(_))));

        let path = dir.path("dup.reqifz");
        zip_of(
            &path,
            &[
                ("doc.reqif", b"1"),
                ("files/A.png", b"1"),
                ("files/a.png", b"2"),
            ],
            CompressionMethod::Stored,
        );
        assert!(matches!(open(&path), Err(ArchiveError::Duplicate(_))));

        let path = dir.path("none.reqifz");
        zip_of(&path, &[("readme.txt", b"hi")], CompressionMethod::Stored);
        assert_eq!(open(&path).err(), Some(ArchiveError::NoDocument));

        let path = dir.path("not-zip.reqifz");
        std::fs::write(&path, b"PK\x03\x04 not really").unwrap();
        assert!(matches!(open(&path), Err(ArchiveError::Invalid(_))));
    }

    #[test]
    fn enforces_entry_count_total_size_and_compression_ratio() {
        let dir = TempDir::new("limits");
        let path = dir.path("many.reqifz");
        let names: Vec<String> = (0..5).map(|i| format!("f{i}.reqif")).collect();
        let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x"[..])).collect();
        zip_of(&path, &entries, CompressionMethod::Stored);
        let small = ArchiveLimits {
            max_entries: 4,
            ..ArchiveLimits::default()
        };
        assert_eq!(
            ReqifArchive::open(&path, small).err(),
            Some(ArchiveError::TooManyEntries(4))
        );

        let path = dir.path("bomb.reqifz");
        let zeros = vec![0u8; 4 * 1024 * 1024];
        zip_of(
            &path,
            &[("doc.reqif", b"<REQ-IF/>"), ("files/zeros.bin", &zeros)],
            CompressionMethod::Deflated,
        );
        assert!(matches!(open(&path), Err(ArchiveError::Bomb(_))));

        let tight = ArchiveLimits {
            max_total_uncompressed: 3 * 1024 * 1024,
            max_ratio: u64::MAX,
            ..ArchiveLimits::default()
        };
        assert!(matches!(
            ReqifArchive::open(&path, tight),
            Err(ArchiveError::TooLargeTotal(3))
        ));
    }

    #[test]
    fn extraction_and_document_reads_are_capped() {
        let dir = TempDir::new("extract");
        let path = dir.path("a.reqifz");
        zip_of(
            &path,
            &[
                ("doc.reqif", b"<REQ-IF>0123456789</REQ-IF>"),
                ("files/a.bin", &[7u8; 2000]),
            ],
            CompressionMethod::Deflated,
        );
        let mut archive = ReqifArchive::open(
            &path,
            ArchiveLimits {
                max_document_bytes: 10,
                ..ArchiveLimits::default()
            },
        )
        .unwrap();
        assert!(matches!(
            archive.read_document("doc.reqif"),
            Err(ArchiveError::TooLarge { .. })
        ));

        let dest = dir.path("out.bin");
        assert_eq!(
            archive.extract_to("files/a.bin", &dest, 2000).unwrap(),
            2000
        );
        assert_eq!(std::fs::read(&dest).unwrap(), vec![7u8; 2000]);

        let dest2 = dir.path("out2.bin");
        assert!(matches!(
            archive.extract_to("files/a.bin", &dest2, 1999),
            Err(ArchiveError::TooLarge { .. })
        ));
        assert!(!dest2.exists(), "a partial file is removed");
        assert!(matches!(
            archive.extract_to("files/nope", &dir.path("x"), 10),
            Err(ArchiveError::NotFound(_))
        ));
    }

    #[test]
    fn writer_round_trips_through_the_reader() {
        let dir = TempDir::new("write");
        let png = dir.path("plot.png");
        std::fs::write(&png, b"\x89PNG fake").unwrap();
        let csv = dir.path("data.csv");
        std::fs::write(&csv, b"a,b\n1,2\n").unwrap();
        let path = dir.path("out.reqifz");
        write_reqifz(
            &path,
            "lunar-lander.reqif",
            b"<REQ-IF/>",
            vec![
                ArchiveFile {
                    entry: "files/1/plot.png".into(),
                    file: File::open(&png).unwrap(),
                    compress: !is_precompressed("image/png"),
                },
                ArchiveFile {
                    entry: "files/2/data.csv".into(),
                    file: File::open(&csv).unwrap(),
                    compress: !is_precompressed("text/csv"),
                },
            ],
        )
        .unwrap();
        let mut archive = open(&path).unwrap();
        assert_eq!(archive.documents(), ["lunar-lander.reqif"]);
        assert_eq!(
            archive
                .resolve("lunar-lander.reqif", "files/1/plot.png")
                .as_deref(),
            Some("files/1/plot.png")
        );
        let out = dir.path("plot-out.png");
        archive.extract_to("files/1/plot.png", &out, 1024).unwrap();
        assert_eq!(std::fs::read(out).unwrap(), b"\x89PNG fake");
        let mut raw = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        assert_eq!(
            raw.by_name("files/1/plot.png").unwrap().compression(),
            CompressionMethod::Stored
        );
        assert_eq!(
            raw.by_name("files/2/data.csv").unwrap().compression(),
            CompressionMethod::Deflated
        );
    }

    #[test]
    fn precompressed_types() {
        assert!(is_precompressed("image/png"));
        assert!(is_precompressed("application/pdf"));
        assert!(is_precompressed(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ));
        assert!(!is_precompressed("text/csv"));
        assert!(!is_precompressed("application/xml"));
    }
}
