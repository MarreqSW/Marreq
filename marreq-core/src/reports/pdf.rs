// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! PDF output: the document model is handed to Typst as JSON and laid out by
//! the fixed library `typst/marreq.typ`, compiled in-process. Fonts are
//! embedded in the binary; Typst has no access to the file system or network.

use std::sync::OnceLock;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};
use typst_layout::PagedDocument;

use super::model::Document;

const LIBRARY: &str = include_str!("typst/marreq.typ");
/// The Marreq logo (gear and check mark), on a transparent background.
pub(crate) const LOGO_PNG: &[u8] = include_bytes!("assets/logo.png");
const MAIN: &str = "#import \"marreq.typ\": render\n#render(json(\"data.json\"))\n";

const FONT_FILES: [&[u8]; 5] = [
    include_bytes!("fonts/Inter-Regular.otf"),
    include_bytes!("fonts/Inter-Bold.otf"),
    include_bytes!("fonts/Inter-Italic.otf"),
    include_bytes!("fonts/Inter-BoldItalic.otf"),
    include_bytes!("fonts/DejaVuSansMono.ttf"),
];

struct Shared {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| {
        let fonts: Vec<Font> = FONT_FILES
            .iter()
            .flat_map(|data| Font::iter(Bytes::new(*data)))
            .collect();
        Shared {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(FontBook::from_fonts(&fonts)),
            fonts,
        }
    })
}

fn file_id(name: &str) -> FileId {
    FileId::new(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(name).expect("static path"),
    ))
}

/// One compilation: the main file, the layout library and the data.
struct ReportWorld {
    main: Source,
    library_source: Source,
    data_id: FileId,
    data: Bytes,
    logo_id: FileId,
    today: Option<Datetime>,
}

impl World for ReportWorld {
    fn library(&self) -> &LazyHash<Library> {
        &shared().library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &shared().book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else if id == self.library_source.id() {
            Ok(self.library_source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.data_id {
            Ok(self.data.clone())
        } else if id == self.logo_id {
            Ok(Bytes::new(LOGO_PNG))
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        shared().fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        self.today
    }
}

/// Render `doc` as PDF; `pdf_a` produces PDF/A-2b. CPU-bound: call it from
/// `spawn_blocking` on request paths.
pub fn render(doc: &Document, pdf_a: bool) -> Result<Vec<u8>, String> {
    let data = serde_json::to_vec(doc).map_err(|e| format!("report data: {e}"))?;
    let today = {
        let mut parts = doc
            .meta
            .date
            .split('-')
            .filter_map(|p| p.parse::<i32>().ok());
        match (parts.next(), parts.next(), parts.next()) {
            (Some(y), Some(m), Some(d)) => Datetime::from_ymd(y, m as u8, d as u8),
            _ => None,
        }
    };
    let world = ReportWorld {
        main: Source::new(file_id("main.typ"), MAIN.to_string()),
        library_source: Source::new(file_id("marreq.typ"), LIBRARY.to_string()),
        data_id: file_id("data.json"),
        data: Bytes::new(data),
        logo_id: file_id("logo.png"),
        today,
    };
    let compiled = typst::compile::<PagedDocument>(&world);
    let document = compiled
        .output
        .map_err(|errors| format!("report layout failed: {}", describe(&world, &errors)))?;
    let mut options = typst_pdf::PdfOptions {
        ident: typst::foundations::Smart::Custom(doc.meta.doc_id.clone()),
        creator: typst::foundations::Smart::Custom(Some("Marreq".into())),
        ..Default::default()
    };
    if pdf_a {
        options.standards = typst_pdf::PdfStandards::new(&[typst_pdf::PdfStandard::A_2b])
            .map_err(|e| format!("PDF/A: {}", e.message()))?;
    }
    typst_pdf::pdf(&document, &options).map_err(|errors| {
        format!(
            "PDF export failed: {}",
            errors
                .iter()
                .map(|d| d.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })
}

/// Diagnostics with the offending line of `marreq.typ` (for logs and tests).
fn describe(world: &ReportWorld, errors: &[typst::diag::SourceDiagnostic]) -> String {
    let line_of = |span: typst::syntax::DiagSpan| -> Option<String> {
        let id = span.id()?;
        let range = world.range(span)?;
        let source = world.source(id).ok()?;
        let text = source.text();
        let start = text[..range.start].rfind('\n').map_or(0, |i| i + 1);
        let end = text[range.start..]
            .find('\n')
            .map_or(text.len(), |i| range.start + i);
        let line = text[..range.start].matches('\n').count() + 1;
        Some(format!("line {line}: {}", text[start..end].trim()))
    };
    errors
        .iter()
        .map(|d| {
            let mut s = d.message.to_string();
            if let Some(at) = line_of(d.span) {
                s.push_str(&format!(" ({at})"));
            }
            for t in d.trace.iter().take(3) {
                if let Some(at) = line_of(t.span.into()) {
                    s.push_str(&format!(" <- {at}"));
                }
            }
            s
        })
        .collect::<Vec<_>>()
        .join("; ")
}
