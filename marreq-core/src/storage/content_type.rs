// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Allowlist of attachment file types, checked against the file's own bytes.
//!
//! The extension picks the expected type; the magic bytes (via `infer`) must
//! agree. Text formats have no magic bytes, so they must look like UTF-8 text
//! instead. HTML, SVG, scripts and executables are never accepted. Files are
//! always served as downloads, so the stored content type is informational.

/// Bytes of the file start that the checks look at (OOXML detection needs
/// the first zip entries).
pub const SNIFF_BYTES: usize = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct AllowedType {
    pub label: &'static str,
    pub extensions: &'static [&'static str],
    /// Content type stored and sent on download.
    pub content_type: &'static str,
    /// Types `infer` may report for a genuine file of this kind.
    sniffed: &'static [&'static str],
    text: bool,
}

const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const PPTX: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
const ODT: &str = "application/vnd.oasis.opendocument.text";
const ODS: &str = "application/vnd.oasis.opendocument.spreadsheet";
const ODP: &str = "application/vnd.oasis.opendocument.presentation";
const ZIP: &str = "application/zip";

const fn binary(
    label: &'static str,
    extensions: &'static [&'static str],
    content_type: &'static str,
    sniffed: &'static [&'static str],
) -> AllowedType {
    AllowedType {
        label,
        extensions,
        content_type,
        sniffed,
        text: false,
    }
}

const fn text(
    label: &'static str,
    extensions: &'static [&'static str],
    content_type: &'static str,
    sniffed: &'static [&'static str],
) -> AllowedType {
    AllowedType {
        label,
        extensions,
        content_type,
        sniffed,
        text: true,
    }
}

pub const ALLOWED_TYPES: &[AllowedType] = &[
    binary("PDF", &["pdf"], "application/pdf", &["application/pdf"]),
    binary("PNG image", &["png"], "image/png", &["image/png"]),
    binary(
        "JPEG image",
        &["jpg", "jpeg"],
        "image/jpeg",
        &["image/jpeg"],
    ),
    binary("GIF image", &["gif"], "image/gif", &["image/gif"]),
    binary("WebP image", &["webp"], "image/webp", &["image/webp"]),
    // Office files are zip archives; older `infer` versions only see the zip.
    binary("Word document", &["docx"], DOCX, &[DOCX, ZIP]),
    binary("Excel workbook", &["xlsx"], XLSX, &[XLSX, ZIP]),
    binary("PowerPoint presentation", &["pptx"], PPTX, &[PPTX, ZIP]),
    binary("OpenDocument text", &["odt"], ODT, &[ODT, ZIP]),
    binary("OpenDocument spreadsheet", &["ods"], ODS, &[ODS, ZIP]),
    binary("OpenDocument presentation", &["odp"], ODP, &[ODP, ZIP]),
    binary(
        "ZIP archive",
        &["zip"],
        ZIP,
        &[ZIP, DOCX, XLSX, PPTX, ODT, ODS, ODP],
    ),
    text("Text", &["txt", "log", "md"], "text/plain", &[]),
    text("CSV", &["csv"], "text/csv", &[]),
    text("JSON", &["json"], "application/json", &[]),
    text("XML", &["xml"], "application/xml", &["text/xml"]),
];

/// Lower-case extensions accepted, for error messages and the API.
pub fn allowed_extensions() -> Vec<&'static str> {
    ALLOWED_TYPES
        .iter()
        .flat_map(|t| t.extensions.iter().copied())
        .collect()
}

fn extension(filename: &str) -> Option<String> {
    let (stem, ext) = filename.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty()).then(|| ext.to_ascii_lowercase())
}

fn looks_like_utf8_text(head: &[u8]) -> bool {
    if head.contains(&0) {
        return false;
    }
    match std::str::from_utf8(head) {
        Ok(_) => true,
        // `head` may cut a multi-byte character in half at the end.
        Err(e) => e.error_len().is_none() && head.len() - e.valid_up_to() < 4,
    }
}

/// Check an upload's name and first bytes against the allowlist.
pub fn detect(filename: &str, head: &[u8]) -> Result<&'static AllowedType, String> {
    let allowed = || allowed_extensions().join(", ");
    let Some(ext) = extension(filename) else {
        return Err(format!(
            "the file name needs an extension; allowed types: {}",
            allowed()
        ));
    };
    let Some(kind) = ALLOWED_TYPES
        .iter()
        .find(|t| t.extensions.contains(&ext.as_str()))
    else {
        return Err(format!(
            ".{ext} files are not allowed; allowed types: {}",
            allowed()
        ));
    };
    let sniffed = infer::get(head).map(|t| t.mime_type());
    let matches = if kind.text {
        sniffed.is_none_or(|m| kind.sniffed.contains(&m)) && looks_like_utf8_text(head)
    } else {
        sniffed.is_some_and(|m| kind.sniffed.contains(&m))
    };
    if matches {
        Ok(kind)
    } else {
        Err(format!(
            "the file content does not match its .{ext} extension ({} expected)",
            kind.label
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDF: &[u8] = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n1 0 obj\n";
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0";
    const JPEG: &[u8] = b"\xff\xd8\xff\xe0\0\x10JFIF\0\x01\x01\0\0\x01\0\x01\0\0";
    const ELF: &[u8] = b"\x7fELF\x02\x01\x01\0\0\0\0\0\0\0\0\0\x03\0>\0\x01\0\0\0";

    /// A minimal zip whose first entry is `name` (enough for magic-byte checks).
    fn zip_with_first_entry(name: &str) -> Vec<u8> {
        let mut out = b"PK\x03\x04\x14\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec();
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&[0u8; 64]);
        out
    }

    #[test]
    fn accepts_allowed_binary_types() {
        assert_eq!(
            detect("spec.pdf", PDF).unwrap().content_type,
            "application/pdf"
        );
        assert_eq!(detect("Plot.PNG", PNG).unwrap().content_type, "image/png");
        assert_eq!(
            detect("photo.jpg", JPEG).unwrap().content_type,
            "image/jpeg"
        );
        assert_eq!(
            detect("photo.jpeg", JPEG).unwrap().content_type,
            "image/jpeg"
        );
        let zip = zip_with_first_entry("data.bin");
        assert_eq!(detect("bundle.zip", &zip).unwrap().content_type, ZIP);
    }

    #[test]
    fn accepts_office_files_that_sniff_as_zip_or_ooxml() {
        let docx = zip_with_first_entry("[Content_Types].xml");
        assert_eq!(detect("report.docx", &docx).unwrap().content_type, DOCX);
        let plain_zip = zip_with_first_entry("content.xml");
        assert_eq!(
            detect("report.docx", &plain_zip).unwrap().content_type,
            DOCX
        );
        assert_eq!(detect("sheet.ods", &plain_zip).unwrap().content_type, ODS);
    }

    #[test]
    fn accepts_a_real_xlsx_workbook() {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        workbook
            .add_worksheet()
            .write_string(0, 0, "REQ-1")
            .unwrap();
        let bytes = workbook.save_to_buffer().unwrap();
        let head = &bytes[..bytes.len().min(SNIFF_BYTES)];
        assert_eq!(detect("matrix.xlsx", head).unwrap().content_type, XLSX);
        assert!(
            detect("matrix.docx", head).is_err(),
            "a workbook is not a Word file"
        );
    }

    #[test]
    fn accepts_utf8_text_formats() {
        assert_eq!(
            detect("notes.txt", "Größe: 5 µm\n".as_bytes())
                .unwrap()
                .content_type,
            "text/plain"
        );
        assert_eq!(
            detect("data.csv", b"a,b\n1,2\n").unwrap().content_type,
            "text/csv"
        );
        assert_eq!(
            detect("cfg.json", b"{\"a\": 1}").unwrap().content_type,
            "application/json"
        );
        assert_eq!(
            detect("model.xml", b"<?xml version=\"1.0\"?><model/>")
                .unwrap()
                .content_type,
            "application/xml"
        );
        // A multi-byte character cut at the end of the sniffed window is fine.
        let cut = &"é".as_bytes()[..1];
        let mut head = b"abc".to_vec();
        head.extend_from_slice(cut);
        assert!(detect("notes.txt", &head).is_ok());
    }

    #[test]
    fn rejects_disallowed_extensions() {
        for name in [
            "page.html",
            "logo.svg",
            "run.sh",
            "tool.exe",
            "noext",
            ".txt",
        ] {
            let err = detect(name, b"hello").unwrap_err();
            assert!(err.contains("allowed types"), "{name}: {err}");
        }
    }

    #[test]
    fn rejects_content_that_does_not_match_the_extension() {
        assert!(
            detect("spec.pdf", ELF)
                .unwrap_err()
                .contains("does not match")
        );
        assert!(detect("spec.pdf", PNG).is_err());
        assert!(detect("photo.png", b"just text").is_err());
        assert!(detect("notes.txt", ELF).is_err());
        assert!(detect("notes.txt", b"bin\0ary").is_err());
        assert!(detect("notes.txt", b"\xff\xfe\xfa invalid").is_err());
        assert!(detect("notes.txt", PDF).is_err());
        assert!(detect("index.txt", b"<!DOCTYPE html><html><body>x</body></html>").is_err());
        assert!(detect("report.docx", PDF).is_err());
    }
}
