// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! ReqIFZ export and import (issue #343): ReqIF plus the requirements'
//! attachment files in one ZIP archive.
//!
//! Both directions block (ZIP and file I/O); call them from `spawn_blocking`.

use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::app::{AppState, DieselCachedRepo};
use crate::models::User;
use crate::reqif::archive::{
    ArchiveError, ArchiveFile, ArchiveLimits, ReqifArchive, is_precompressed, write_reqifz,
};
use crate::reqif::import::{ImportConfig, ImportResult, parse_reqif};
use crate::services::ReqIFService;
use crate::services::attachment_service::{
    AttachmentEntity, AttachmentError, AttachmentService, NewUpload, format_mib, sanitize_filename,
};
use crate::services::reqif_service::ExportInput;
use crate::storage::{AttachmentStorage, content_type};

/// The warning the parser adds for any embedded file; replaced by specific ones here.
const EMBEDDED_WARNING: &str =
    "embedded/file content was present in the ReqIF document and was not imported";

fn private_temp_path(prefix: &str, extension: &str) -> io::Result<PathBuf> {
    let suffix: u64 = rand::random();
    let path = std::env::temp_dir().join(format!("{prefix}-{suffix:016x}.{extension}"));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(&path)?;
    Ok(path)
}

/// Write `input` as a ReqIFZ archive to a private temp file and return its
/// path (the caller streams and removes it). The document is named
/// `<document_name>`; each requirement attachment goes to
/// [`ExportInput::archive_path`] and is linked from its statement. A file
/// missing from the store is left out (and not linked).
pub fn export_archive(
    input: &ExportInput,
    storage: &AttachmentStorage,
    document_name: &str,
) -> io::Result<PathBuf> {
    let mut files = Vec::new();
    let mut linked = HashSet::new();
    for a in input
        .attachments
        .iter()
        .filter(|a| a.entity_type == "requirement")
    {
        match storage.store().open(&a.sha256) {
            Ok(file) => {
                linked.insert(a.id);
                files.push(ArchiveFile {
                    entry: ExportInput::archive_path(a),
                    file,
                    compress: !is_precompressed(&a.content_type),
                });
            }
            Err(e) => eprintln!(
                "[marreq] reqifz export: attachment {} ({}) is missing from storage: {e}",
                a.id, a.original_filename
            ),
        }
    }
    let xml = input.to_xml(Some(&linked));
    let path = private_temp_path("marreq-reqifz", "reqifz")?;
    if let Err(e) = write_reqifz(&path, document_name, xml.as_bytes(), files) {
        let _ = std::fs::remove_file(&path);
        return Err(e);
    }
    Ok(path)
}

/// Result of a ReqIFZ import: the documents' results merged, plus the files.
#[derive(Debug)]
pub struct ReqifzImport {
    pub result: ImportResult,
    pub documents: Vec<String>,
    pub imported_attachment_count: usize,
}

fn merge(into: &mut ImportResult, from: ImportResult, prefix: &str) {
    into.imported_count += from.imported_count;
    into.created_link_count += from.created_link_count;
    into.imported_requirement_ids
        .extend(from.imported_requirement_ids);
    into.errors
        .extend(from.errors.into_iter().map(|e| format!("{prefix}{e}")));
    into.warnings
        .extend(from.warnings.into_iter().map(|w| format!("{prefix}{w}")));
}

fn read_head(path: &Path) -> io::Result<Vec<u8>> {
    let mut head = Vec::with_capacity(content_type::SNIFF_BYTES);
    std::fs::File::open(path)?
        .take(content_type::SNIFF_BYTES as u64)
        .read_to_end(&mut head)?;
    Ok(head)
}

/// Why one referenced file was not attached (becomes a warning).
fn attach_one(
    service: &AttachmentService<'_>,
    storage: &AttachmentStorage,
    archive: &mut ReqifArchive,
    actor: &User,
    project_id: i32,
    requirement_id: i32,
    entry: &str,
) -> Result<(), String> {
    let filename = sanitize_filename(entry.rsplit('/').next().unwrap_or(entry))
        .ok_or_else(|| format!("{entry} has no usable file name"))?;
    let max = storage.config.max_file_bytes;
    let tmp = storage
        .store()
        .new_upload_path()
        .map_err(|e| format!("attachment storage is unavailable: {e}"))?;
    let size = match archive.extract_to(entry, &tmp, max) {
        Ok(size) => size,
        Err(ArchiveError::TooLarge { .. }) => {
            return Err(format!(
                "{filename} is larger than the {} per-file limit",
                format_mib(max as i64)
            ));
        }
        Err(e) => return Err(e.to_string()),
    };
    let cleanup = |e: String| {
        let _ = std::fs::remove_file(&tmp);
        e
    };
    if size == 0 {
        return Err(cleanup(format!("{filename} is empty")));
    }
    let head = read_head(&tmp).map_err(|e| cleanup(e.to_string()))?;
    let kind =
        content_type::detect(&filename, &head).map_err(|e| cleanup(format!("{filename}: {e}")))?;
    let staged = storage
        .store()
        .stage(&tmp)
        .map_err(|e| cleanup(e.to_string()))?;
    service
        .create(
            actor,
            NewUpload {
                project_id,
                entity: AttachmentEntity::Requirement,
                entity_id: requirement_id,
                filename: filename.clone(),
                content_type: kind.content_type,
            },
            staged,
        )
        .map(|_| ())
        .map_err(|e| match e {
            AttachmentError::TooLarge(msg)
            | AttachmentError::UnsupportedType(msg)
            | AttachmentError::BadInput(msg)
            | AttachmentError::Storage(msg) => format!("{filename}: {msg}"),
            AttachmentError::Repo(e) => format!("{filename}: {e}"),
        })
}

/// Import every `.reqif` document of the archive at `archive_path` into the
/// project, then attach the files their XHTML objects reference. Files that
/// fail a check (missing, unsafe path, size, type, quota) are skipped with a
/// warning; the requirements stay imported.
pub fn import_archive(
    state: &AppState<DieselCachedRepo>,
    storage: &Arc<AttachmentStorage>,
    actor: &User,
    config: &ImportConfig,
    archive_path: &Path,
) -> Result<ReqifzImport, String> {
    let mut archive =
        ReqifArchive::open(archive_path, ArchiveLimits::default()).map_err(|e| e.to_string())?;
    let documents = archive.documents().to_vec();
    let several = documents.len() > 1;
    let reqif = ReqIFService::new(state);
    let attachments = AttachmentService::new(state, storage);
    let mut merged = ImportResult {
        success: true,
        message: String::new(),
        imported_count: 0,
        created_link_count: 0,
        errors: Vec::new(),
        warnings: Vec::new(),
        imported_requirement_ids: Vec::new(),
        object_requirement_ids: HashMap::new(),
    };
    let mut reserved = HashSet::new();
    let mut imported_attachment_count = 0usize;

    for document in &documents {
        let prefix = if several {
            format!("{document}: ")
        } else {
            String::new()
        };
        let parsed = match archive
            .read_document(document)
            .map_err(|e| e.to_string())
            .and_then(|bytes| parse_reqif(&bytes))
        {
            Ok(parsed) => parsed,
            Err(e) => {
                merged.errors.push(format!("{prefix}{e}"));
                continue;
            }
        };
        let mut result = reqif.import_parsed(&parsed, config, actor, &mut reserved)?;
        let reference_count: usize = parsed.objects.iter().map(|o| o.object_refs.len()).sum();
        result.warnings.retain(|w| w != EMBEDDED_WARNING);
        if parsed.attachment_count > reference_count {
            result.warnings.push(format!(
                "{} embedded file(s) not given as XHTML objects were not imported",
                parsed.attachment_count - reference_count
            ));
        }
        for object in &parsed.objects {
            let Some(&requirement_id) = result.object_requirement_ids.get(&object.id) else {
                continue;
            };
            let label = object
                .long_name
                .clone()
                .unwrap_or_else(|| object.id.clone());
            let mut attached = HashSet::new();
            for reference in &object.object_refs {
                let Some(entry) = archive.resolve(document, &reference.data) else {
                    result.warnings.push(format!(
                        "{label}: {} is not in the archive; not attached",
                        reference.data
                    ));
                    continue;
                };
                if !attached.insert(entry.clone()) {
                    continue;
                }
                match attach_one(
                    &attachments,
                    storage,
                    &mut archive,
                    actor,
                    config.project_id,
                    requirement_id,
                    &entry,
                ) {
                    Ok(()) => imported_attachment_count += 1,
                    Err(why) => result
                        .warnings
                        .push(format!("{label}: {why}; not attached")),
                }
            }
        }
        merge(&mut merged, result, &prefix);
    }

    merged.success = merged.errors.is_empty();
    merged.message = if merged.success {
        format!(
            "Successfully imported {} requirements ({} links, {} files)",
            merged.imported_count, merged.created_link_count, imported_attachment_count
        )
    } else {
        format!(
            "Imported {} requirements and {} files with {} errors",
            merged.imported_count,
            imported_attachment_count,
            merged.errors.len()
        )
    };
    Ok(ReqifzImport {
        result: merged,
        documents,
        imported_attachment_count,
    })
}
