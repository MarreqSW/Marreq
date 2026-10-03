// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! ODT (OpenDocument text) output, written directly as ODF XML in a zip.
//! Headings carry outline levels (LibreOffice can rebuild the contents),
//! headers and footers show the page number and count, landscape blocks use a
//! landscape master page, and table headers repeat on every page.

use std::fmt::Write as _;
use std::io::{Cursor, Write as _};

use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

use super::model::{Align, Block, Cell, DocMeta, Document, Inline, RowKind, Table, Tone};

pub const MIME: &str = "application/vnd.oasis.opendocument.text";

/// Text width in cm for A4 portrait / landscape with the page margins below.
const PORTRAIT_WIDTH_CM: f32 = 17.0;
const LANDSCAPE_WIDTH_CM: f32 = 25.7;

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // XML 1.0 forbids most control characters.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

/// Text with tabs, newlines and runs of spaces as ODF elements.
fn text_runs(s: &str) -> String {
    let mut out = String::new();
    let mut spaces = 0usize;
    let flush = |out: &mut String, spaces: &mut usize| {
        if *spaces > 0 {
            out.push(' ');
            if *spaces > 1 {
                let _ = write!(out, "<text:s text:c=\"{}\"/>", *spaces - 1);
            }
            *spaces = 0;
        }
    };
    for c in s.chars() {
        match c {
            ' ' => spaces += 1,
            '\n' => {
                flush(&mut out, &mut spaces);
                out.push_str("<text:line-break/>");
            }
            '\t' => {
                flush(&mut out, &mut spaces);
                out.push_str("<text:tab/>");
            }
            c => {
                flush(&mut out, &mut spaces);
                out.push_str(&esc(&c.to_string()));
            }
        }
    }
    flush(&mut out, &mut spaces);
    out
}

fn tone_name(t: Tone) -> &'static str {
    match t {
        Tone::Ok => "ok",
        Tone::Warn => "warn",
        Tone::Bad => "bad",
        Tone::Open => "open",
        Tone::None => "none",
    }
}

const TONES: [(Tone, &str, &str); 5] = [
    (Tone::Ok, "#dcfce7", "#166534"),
    (Tone::Warn, "#fef3c7", "#92400e"),
    (Tone::Bad, "#fee2e2", "#991b1b"),
    (Tone::Open, "#e0e7ff", "#3730a3"),
    (Tone::None, "#f1f5f9", "#475569"),
];

/// Width / height of the logo image (254 × 169 px).
const LOGO_RATIO: f32 = 254.0 / 169.0;

/// The Marreq logo as an inline picture `height_cm` tall.
fn logo(name: &str, height_cm: f32) -> String {
    format!(
        "<draw:frame draw:name=\"{name}\" text:anchor-type=\"as-char\" svg:width=\"{:.2}cm\" svg:height=\"{height_cm:.2}cm\" draw:z-index=\"0\"><draw:image xlink:href=\"Pictures/logo.png\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"/></draw:frame>",
        height_cm * LOGO_RATIO
    )
}

fn inline(i: &Inline) -> String {
    match i {
        Inline::Text { text } => text_runs(text),
        Inline::Bold { text } => format!(
            "<text:span text:style-name=\"T_bold\">{}</text:span>",
            text_runs(text)
        ),
        Inline::Muted { text } => format!(
            "<text:span text:style-name=\"T_muted\">{}</text:span>",
            text_runs(text)
        ),
        Inline::Code { text } => format!(
            "<text:span text:style-name=\"T_code\">{}</text:span>",
            text_runs(text)
        ),
        Inline::Badge { tone, text } => format!(
            "<text:span text:style-name=\"T_badge_{}\">{}</text:span>",
            tone_name(*tone),
            text_runs(&format!("\u{a0}{text}\u{a0}"))
        ),
        Inline::LineBreak => "<text:line-break/>".into(),
    }
}

fn inlines(xs: &[Inline]) -> String {
    xs.iter().map(inline).collect()
}

struct Writer {
    body: String,
    /// Automatic table styles, collected while writing the body.
    auto_styles: String,
    tables: usize,
    landscape: bool,
}

impl Writer {
    fn p(&mut self, style: &str, content: &str) {
        let _ = write!(
            self.body,
            "<text:p text:style-name=\"{style}\">{content}</text:p>"
        );
    }

    fn table(&mut self, t: &Table) {
        self.tables += 1;
        let name = format!("Table{}", self.tables);
        let width = if self.landscape {
            LANDSCAPE_WIDTH_CM
        } else {
            PORTRAIT_WIDTH_CM
        };
        let total: f32 = t.columns.iter().map(|c| c.width).sum::<f32>().max(0.0001);
        let _ = write!(
            self.auto_styles,
            "<style:style style:name=\"{name}\" style:family=\"table\"><style:table-properties style:width=\"{width}cm\" table:align=\"margins\" fo:margin-top=\"0.1cm\" fo:margin-bottom=\"0.25cm\"/></style:style>"
        );
        for (i, c) in t.columns.iter().enumerate() {
            let w = width * c.width / total;
            let _ = write!(
                self.auto_styles,
                "<style:style style:name=\"{name}.C{i}\" style:family=\"table-column\"><style:table-column-properties style:column-width=\"{w:.2}cm\" style:rel-column-width=\"{}*\"/></style:style>",
                ((c.width / total) * 10000.0).round() as i64
            );
        }
        let (para, head_para) = if t.compact {
            ("Table_20_Contents_20_Small", "Table_20_Heading_20_Small")
        } else {
            ("Table_20_Contents", "Table_20_Heading")
        };
        let _ = write!(
            self.body,
            "<table:table table:name=\"{name}\" table:style-name=\"{name}\">"
        );
        for i in 0..t.columns.len() {
            let _ = write!(
                self.body,
                "<table:table-column table:style-name=\"{name}.C{i}\"/>"
            );
        }
        self.body
            .push_str("<table:table-header-rows><table:table-row>");
        for c in &t.columns {
            let style = match c.align {
                Align::Left => head_para.to_string(),
                Align::Center => format!("{head_para}_Center"),
                Align::Right => format!("{head_para}_Right"),
            };
            let _ = write!(
                self.body,
                "<table:table-cell table:style-name=\"Cell_head\" office:value-type=\"string\"><text:p text:style-name=\"{style}\">{}</text:p></table:table-cell>",
                esc(&c.title.to_uppercase())
            );
        }
        self.body
            .push_str("</table:table-row></table:table-header-rows>");
        let n = t.columns.len();
        for r in &t.rows {
            self.body.push_str("<table:table-row>");
            if r.kind == RowKind::Group {
                let label = match r.cells.first() {
                    Some(Cell::Content { inlines: xs }) => inlines(xs),
                    _ => String::new(),
                };
                let _ = write!(
                    self.body,
                    "<table:table-cell table:style-name=\"Cell_group\" table:number-columns-spanned=\"{n}\" office:value-type=\"string\"><text:p text:style-name=\"{para}\"><text:span text:style-name=\"T_group\">{label}</text:span></text:p></table:table-cell>"
                );
                for _ in 1..n {
                    self.body.push_str("<table:covered-table-cell/>");
                }
            } else {
                for (i, cell) in r.cells.iter().enumerate() {
                    let align = t.columns.get(i).map(|c| c.align).unwrap_or(Align::Left);
                    let style = match align {
                        Align::Left => para.to_string(),
                        Align::Center => format!("{para}_Center"),
                        Align::Right => format!("{para}_Right"),
                    };
                    let mut content = match cell {
                        Cell::Content { inlines: xs } => inlines(xs),
                        Cell::Bar { parts } => bar(parts),
                    };
                    if r.kind == RowKind::Total {
                        content =
                            format!("<text:span text:style-name=\"T_bold\">{content}</text:span>");
                    }
                    let cell_style = if r.kind == RowKind::Total {
                        "Cell_total"
                    } else {
                        "Cell_body"
                    };
                    let _ = write!(
                        self.body,
                        "<table:table-cell table:style-name=\"{cell_style}\" office:value-type=\"string\"><text:p text:style-name=\"{style}\">{content}</text:p></table:table-cell>"
                    );
                }
            }
            self.body.push_str("</table:table-row>");
        }
        self.body.push_str("</table:table>");
    }

    fn blocks(&mut self, blocks: &[Block], toc: &[(u8, String)]) {
        for b in blocks {
            match b {
                Block::Cover {
                    kicker,
                    title,
                    subtitle,
                    fields,
                    signatories,
                    note,
                } => {
                    self.p(
                        "Cover_20_Brand",
                        &format!(
                            "{} <text:span text:style-name=\"T_bold\">Marreq</text:span>",
                            logo("Logo", 1.0)
                        ),
                    );
                    self.p("Cover_20_Kicker", &esc(&kicker.to_uppercase()));
                    self.p("Title", &esc(title));
                    self.p("Subtitle", &esc(subtitle));
                    let mut t = Table::new(vec![
                        super::model::Column::new("Field", 0.3),
                        super::model::Column::new("Value", 0.7),
                    ]);
                    for f in fields {
                        t.row(vec![
                            Cell::of(vec![Inline::muted(&f.key)]),
                            Cell::of(vec![Inline::bold(&f.value)]),
                        ]);
                    }
                    self.table(&t);
                    if !signatories.is_empty() {
                        let mut t = Table::new(vec![
                            super::model::Column::new("", 0.24),
                            super::model::Column::new("Name / role", 0.36),
                            super::model::Column::new("Signature", 0.25),
                            super::model::Column::new("Date", 0.15),
                        ]);
                        for s in signatories {
                            t.row(vec![
                                Cell::of(vec![Inline::bold(&s.role)]),
                                Cell::text(&s.name),
                                Cell::text(""),
                                Cell::text(""),
                            ]);
                        }
                        self.table(&t);
                    }
                    if let Some(note) = note {
                        self.p("Standard", &text_runs(note));
                    }
                }
                Block::Toc { title } => {
                    self.body.push_str(
                        "<text:table-of-content text:name=\"Contents\" text:protected=\"true\"><text:table-of-content-source text:outline-level=\"2\" text:use-index-marks=\"false\">",
                    );
                    let _ = write!(
                        self.body,
                        "<text:index-title-template text:style-name=\"Contents_20_Heading\">{}</text:index-title-template>",
                        esc(title)
                    );
                    for level in 1..=2 {
                        let _ = write!(
                            self.body,
                            "<text:table-of-content-entry-template text:outline-level=\"{level}\" text:style-name=\"Contents_20_{level}\"><text:index-entry-chapter/><text:index-entry-text/><text:index-entry-tab-stop style:type=\"right\" style:leader-char=\".\"/><text:index-entry-page-number/></text:table-of-content-entry-template>"
                        );
                    }
                    self.body
                        .push_str("</text:table-of-content-source><text:index-body>");
                    let _ = write!(
                        self.body,
                        "<text:index-title text:name=\"Contents_Head\"><text:p text:style-name=\"Contents_20_Heading\">{}</text:p></text:index-title>",
                        esc(title)
                    );
                    // Pre-filled so the contents show at once; LibreOffice's
                    // "Update index" adds the page numbers.
                    for (level, text) in toc {
                        let _ = write!(
                            self.body,
                            "<text:p text:style-name=\"Contents_20_{}\">{}</text:p>",
                            level.clamp(&1, &2),
                            esc(text)
                        );
                    }
                    self.body
                        .push_str("</text:index-body></text:table-of-content>");
                }
                Block::Heading {
                    level,
                    number,
                    text,
                    outlined,
                } => {
                    let label = if number.is_empty() {
                        esc(text)
                    } else {
                        format!("{}<text:tab/>{}", esc(number), esc(text))
                    };
                    if *outlined {
                        let _ = write!(
                            self.body,
                            "<text:h text:style-name=\"Heading_20_{level}\" text:outline-level=\"{level}\">{label}</text:h>"
                        );
                    } else {
                        // Front-matter heading: same look, not in the contents.
                        self.p(&format!("Heading_20_{level}_20_Front"), &label);
                    }
                }
                Block::Paragraph { inlines: xs } => self.p("Text_20_body", &inlines(xs)),
                Block::Bullets { items } => {
                    self.body
                        .push_str("<text:list text:style-name=\"L_bullets\">");
                    for item in items {
                        let _ = write!(
                            self.body,
                            "<text:list-item><text:p text:style-name=\"List_20_Paragraph\">{}</text:p></text:list-item>",
                            inlines(item)
                        );
                    }
                    self.body.push_str("</text:list>");
                }
                Block::Table(t) => self.table(t),
                Block::Kpis { items } => {
                    let mut t = Table::new(
                        items
                            .iter()
                            .map(|_| super::model::Column::new("", 1.0).center())
                            .collect(),
                    );
                    t.columns
                        .iter_mut()
                        .zip(items)
                        .for_each(|(c, k)| c.title = k.label.clone());
                    t.row(
                        items
                            .iter()
                            .map(|k| Cell::of(vec![Inline::badge(k.tone, &k.value)]))
                            .collect(),
                    );
                    self.table(&t);
                }
                Block::Legend { items } => {
                    let content: String = items
                        .iter()
                        .map(|(tone, label)| format!("{}  ", inline(&Inline::badge(*tone, label))))
                        .collect();
                    self.p("Legend", &content);
                }
                Block::PageBreak => self.p("P_break", ""),
                Block::Landscape { blocks } => {
                    // A paragraph whose style names a master page starts a new
                    // page with that master.
                    self.p("P_landscape", "");
                    self.landscape = true;
                    self.blocks(blocks, toc);
                    self.landscape = false;
                    self.p("P_portrait", "");
                }
            }
        }
    }
}

fn bar(parts: &[super::model::BarPart]) -> String {
    let total: f64 = parts.iter().map(|p| p.value).sum();
    if total <= 0.0 {
        return inline(&Inline::muted("–"));
    }
    parts
        .iter()
        .filter(|p| p.value > 0.0)
        .map(|p| {
            inline(&Inline::badge(
                p.tone,
                format!("{}%", ((p.value / total) * 100.0).round() as i64),
            ))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn styles_xml(meta: &DocMeta) -> String {
    let (w, h) = if meta.paper == "us-letter" {
        ("21.59cm", "27.94cm")
    } else {
        ("21cm", "29.7cm")
    };
    let header = format!(
        "<text:p text:style-name=\"Header\">{} · {}<text:tab/>{}</text:p>",
        esc(&meta.project),
        esc(&meta.title),
        esc(&[meta.doc_id.as_str(), meta.issue_label.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" · "))
    );
    let footer_left = [meta.generated_by.as_str(), meta.classification.as_str()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" · ");
    let footer = format!(
        "<text:p text:style-name=\"Footer\">{}<text:tab/>Page <text:page-number text:select-page=\"current\">1</text:page-number> of <text:page-count>1</text:page-count></text:p>",
        esc(&footer_left)
    );
    let mut tone_styles = String::new();
    for (tone, bg, fg) in TONES {
        let _ = write!(
            tone_styles,
            "<style:style style:name=\"T_badge_{}\" style:family=\"text\"><style:text-properties fo:font-weight=\"bold\" fo:font-size=\"85%\" fo:color=\"{fg}\" fo:background-color=\"{bg}\"/></style:style>",
            tone_name(tone)
        );
    }
    let para = |name: &str, parent: &str, props: &str, text: &str| {
        format!(
            "<style:style style:name=\"{name}\" style:display-name=\"{}\" style:family=\"paragraph\" style:parent-style-name=\"{parent}\"><style:paragraph-properties {props}/><style:text-properties {text}/></style:style>",
            name.replace("_20_", " ")
        )
    };
    let mut paras = String::new();
    paras.push_str(&para(
        "Text_20_body",
        "Standard",
        "fo:margin-top=\"0cm\" fo:margin-bottom=\"0.2cm\"",
        "",
    ));
    paras.push_str(&para(
        "Title",
        "Standard",
        "fo:margin-top=\"0.2cm\" fo:margin-bottom=\"0.2cm\"",
        "fo:font-size=\"26pt\" fo:font-weight=\"bold\" fo:color=\"#0b1d4d\"",
    ));
    paras.push_str(&para(
        "Subtitle",
        "Standard",
        "fo:margin-bottom=\"0.8cm\"",
        "fo:font-size=\"11pt\" fo:color=\"#475569\"",
    ));
    paras.push_str(&para(
        "Cover_20_Brand",
        "Standard",
        "fo:margin-bottom=\"4cm\"",
        "fo:font-size=\"14pt\" fo:color=\"#0b1d4d\"",
    ));
    paras.push_str(&para(
        "Cover_20_Kicker",
        "Standard",
        "",
        "fo:font-size=\"8.5pt\" fo:color=\"#1d4ed8\" fo:letter-spacing=\"0.05cm\"",
    ));
    for (level, size, above) in [
        (1, "13pt", "0.5cm"),
        (2, "10.5pt", "0.35cm"),
        (3, "9.5pt", "0.3cm"),
    ] {
        let _ = write!(
            paras,
            "<style:style style:name=\"Heading_20_{level}\" style:display-name=\"Heading {level}\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:next-style-name=\"Text_20_body\" style:default-outline-level=\"{level}\"><style:paragraph-properties fo:margin-top=\"{above}\" fo:margin-bottom=\"0.2cm\" fo:keep-with-next=\"always\"><style:tab-stops><style:tab-stop style:position=\"1cm\"/></style:tab-stops></style:paragraph-properties><style:text-properties fo:font-size=\"{size}\" fo:font-weight=\"bold\" fo:color=\"#0b1d4d\"/></style:style>"
        );
        let _ = write!(
            paras,
            "<style:style style:name=\"Heading_20_{level}_20_Front\" style:display-name=\"Heading {level} Front\" style:family=\"paragraph\" style:parent-style-name=\"Heading_20_{level}\"/>"
        );
    }
    paras.push_str(&para(
        "Table_20_Contents",
        "Standard",
        "fo:margin-top=\"0cm\" fo:margin-bottom=\"0cm\"",
        "fo:font-size=\"8.5pt\"",
    ));
    paras.push_str(&para(
        "Table_20_Contents_20_Small",
        "Standard",
        "fo:margin-top=\"0cm\" fo:margin-bottom=\"0cm\"",
        "fo:font-size=\"7.5pt\"",
    ));
    paras.push_str(&para(
        "Table_20_Heading",
        "Standard",
        "fo:margin-top=\"0cm\" fo:margin-bottom=\"0cm\"",
        "fo:font-size=\"7pt\" fo:font-weight=\"bold\" fo:color=\"#ffffff\"",
    ));
    paras.push_str(&para(
        "Table_20_Heading_20_Small",
        "Table_20_Heading",
        "",
        "",
    ));
    for base in [
        "Table_20_Contents",
        "Table_20_Contents_20_Small",
        "Table_20_Heading",
        "Table_20_Heading_20_Small",
    ] {
        for (suffix, align) in [("_Center", "center"), ("_Right", "end")] {
            let _ = write!(
                paras,
                "<style:style style:name=\"{base}{suffix}\" style:family=\"paragraph\" style:parent-style-name=\"{base}\"><style:paragraph-properties fo:text-align=\"{align}\"/></style:style>"
            );
        }
    }
    paras.push_str(&para(
        "List_20_Paragraph",
        "Standard",
        "fo:margin-bottom=\"0.1cm\"",
        "",
    ));
    paras.push_str(&para(
        "Legend",
        "Standard",
        "fo:margin-bottom=\"0.2cm\"",
        "fo:font-size=\"7.5pt\"",
    ));
    paras.push_str(&para(
        "Contents_20_Heading",
        "Standard",
        "fo:margin-bottom=\"0.3cm\"",
        "fo:font-size=\"13pt\" fo:font-weight=\"bold\" fo:color=\"#0b1d4d\"",
    ));
    for level in 1..=2 {
        let indent = if level == 1 { "0cm" } else { "0.6cm" };
        let weight = if level == 1 { "bold" } else { "normal" };
        let _ = write!(
            paras,
            "<style:style style:name=\"Contents_20_{level}\" style:display-name=\"Contents {level}\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:class=\"index\"><style:paragraph-properties fo:margin-left=\"{indent}\" fo:margin-bottom=\"0.1cm\"><style:tab-stops><style:tab-stop style:position=\"17cm\" style:type=\"right\" style:leader-style=\"dotted\" style:leader-text=\".\"/></style:tab-stops></style:paragraph-properties><style:text-properties fo:font-weight=\"{weight}\"/></style:style>"
        );
    }
    paras.push_str(&para(
        "Header",
        "Standard",
        "",
        "fo:font-size=\"7.5pt\" fo:color=\"#64748b\"",
    ));
    paras.push_str(&para(
        "Footer",
        "Standard",
        "",
        "fo:font-size=\"7pt\" fo:color=\"#64748b\"",
    ));
    // Header and footer tab stops: right-aligned at the text width.
    let tab = |w: f32| {
        format!(
            "<style:paragraph-properties><style:tab-stops><style:tab-stop style:position=\"{w}cm\" style:type=\"right\"/></style:tab-stops></style:paragraph-properties>"
        )
    };
    let master_content = |w: f32, n: u8| {
        format!(
            "<style:header>{}</style:header><style:footer>{}</style:footer>",
            header.replacen(
                "<text:p text:style-name=\"Header\">",
                &format!(
                    "<text:p text:style-name=\"Header_{w}\">{} ",
                    logo(&format!("HeaderLogo{n}"), 0.32)
                ),
                1
            ),
            footer.replacen(
                "<text:p text:style-name=\"Footer\">",
                &format!("<text:p text:style-name=\"Footer_{w}\">"),
                1
            ),
        )
    };
    let mut header_footer_styles = String::new();
    for w in [PORTRAIT_WIDTH_CM, LANDSCAPE_WIDTH_CM] {
        for kind in ["Header", "Footer"] {
            let _ = write!(
                header_footer_styles,
                "<style:style style:name=\"{kind}_{w}\" style:family=\"paragraph\" style:parent-style-name=\"{kind}\">{}</style:style>",
                tab(w)
            );
        }
    }
    let layout = |name: &str, width: &str, height: &str, orientation: &str| {
        format!(
            "<style:page-layout style:name=\"{name}\"><style:page-layout-properties fo:page-width=\"{width}\" fo:page-height=\"{height}\" style:print-orientation=\"{orientation}\" fo:margin-top=\"1.2cm\" fo:margin-bottom=\"1.2cm\" fo:margin-left=\"2cm\" fo:margin-right=\"2cm\"/><style:header-style><style:header-footer-properties fo:min-height=\"0.6cm\" fo:margin-bottom=\"0.4cm\"/></style:header-style><style:footer-style><style:header-footer-properties fo:min-height=\"0.6cm\" fo:margin-top=\"0.4cm\"/></style:footer-style></style:page-layout>"
        )
    };
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" office:version="1.3">
<office:font-face-decls><style:font-face style:name="Inter" svg:font-family="Inter" style:font-family-generic="swiss"/><style:font-face style:name="DejaVu Sans Mono" svg:font-family="'DejaVu Sans Mono'" style:font-pitch="fixed"/></office:font-face-decls>
<office:styles>
<style:default-style style:family="paragraph"><style:paragraph-properties fo:hyphenation-ladder-count="no-limit"/><style:text-properties style:font-name="Inter" fo:font-size="9pt" fo:color="#0f172a" fo:language="en"/></style:default-style>
<style:style style:name="Standard" style:family="paragraph" style:class="text"/>
{paras}{header_footer_styles}
<style:style style:name="T_bold" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>
<style:style style:name="T_muted" style:family="text"><style:text-properties fo:color="#64748b"/></style:style>
<style:style style:name="T_code" style:family="text"><style:text-properties style:font-name="DejaVu Sans Mono" fo:font-size="88%"/></style:style>
<style:style style:name="T_group" style:family="text"><style:text-properties fo:font-weight="bold" fo:color="#1e3a8a"/></style:style>
{tone_styles}
<text:list-style style:name="L_bullets"><text:list-level-style-bullet text:level="1" text:bullet-char="•"><style:list-level-properties text:list-level-position-and-space-mode="label-alignment"><style:list-level-label-alignment text:label-followed-by="listtab" text:list-tab-stop-position="0.6cm" fo:text-indent="-0.4cm" fo:margin-left="0.6cm"/></style:list-level-properties></text:list-level-style-bullet></text:list-style>
</office:styles>
<office:automatic-styles>{portrait}{landscape}</office:automatic-styles>
<office:master-styles>
<style:master-page style:name="Standard" style:page-layout-name="PM_portrait">{master_portrait}</style:master-page>
<style:master-page style:name="Landscape" style:page-layout-name="PM_landscape">{master_landscape}</style:master-page>
</office:master-styles>
</office:document-styles>"##,
        portrait = layout("PM_portrait", w, h, "portrait"),
        landscape = layout("PM_landscape", h, w, "landscape"),
        master_portrait = master_content(PORTRAIT_WIDTH_CM, 1),
        master_landscape = master_content(LANDSCAPE_WIDTH_CM, 2),
    )
}

fn content_xml(doc: &Document) -> String {
    let toc: Vec<(u8, String)> = super::model::headings(&doc.blocks)
        .into_iter()
        .filter(|(level, _, _, outlined)| *outlined && *level <= 2)
        .map(|(level, number, text, _)| {
            (
                level,
                if number.is_empty() {
                    text
                } else {
                    format!("{number}  {text}")
                },
            )
        })
        .collect();
    let mut w = Writer {
        body: String::new(),
        auto_styles: String::new(),
        tables: 0,
        landscape: false,
    };
    w.blocks(&doc.blocks, &toc);
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:xlink="http://www.w3.org/1999/xlink" office:version="1.3">
<office:automatic-styles>
<style:style style:name="P_break" style:family="paragraph" style:parent-style-name="Standard"><style:paragraph-properties fo:break-before="page"/></style:style>
<style:style style:name="P_landscape" style:family="paragraph" style:parent-style-name="Standard" style:master-page-name="Landscape"/>
<style:style style:name="P_portrait" style:family="paragraph" style:parent-style-name="Standard" style:master-page-name="Standard"/>
<style:style style:name="Cell_head" style:family="table-cell"><style:table-cell-properties fo:background-color="#0b1d4d" fo:padding="0.08cm" fo:border="none"/></style:style>
<style:style style:name="Cell_body" style:family="table-cell"><style:table-cell-properties fo:padding="0.08cm" fo:border-left="none" fo:border-right="none" fo:border-top="none" fo:border-bottom="0.5pt solid #e2e8f0"/></style:style>
<style:style style:name="Cell_total" style:family="table-cell"><style:table-cell-properties fo:padding="0.08cm" fo:border-left="none" fo:border-right="none" fo:border-bottom="none" fo:border-top="1pt solid #94a3b8"/></style:style>
<style:style style:name="Cell_group" style:family="table-cell"><style:table-cell-properties fo:background-color="#eef2ff" fo:padding="0.08cm" fo:border="none"/></style:style>
{styles}
</office:automatic-styles>
<office:body><office:text>{body}</office:text></office:body>
</office:document-content>"##,
        styles = w.auto_styles,
        body = w.body,
    )
}

fn meta_xml(meta: &DocMeta) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" xmlns:dc="http://purl.org/dc/elements/1.1/" office:version="1.3"><office:meta><meta:generator>Marreq</meta:generator><dc:title>{}</dc:title><dc:subject>{}</dc:subject><meta:creation-date>{}T00:00:00</meta:creation-date></office:meta></office:document-meta>"##,
        esc(&meta.title),
        esc(&meta.doc_id),
        esc(&meta.date)
    )
}

const MANIFEST: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3"><manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="application/vnd.oasis.opendocument.text"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="Pictures/logo.png" manifest:media-type="image/png"/></manifest:manifest>"##;

/// Render `doc` as an ODT file.
pub fn render(doc: &Document) -> Result<Vec<u8>, String> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let io = |e: std::io::Error| format!("ODT: {e}");
    let zerr = |e: zip::result::ZipError| format!("ODT: {e}");
    // The mimetype entry must come first and be stored uncompressed.
    zip.start_file("mimetype", stored).map_err(zerr)?;
    zip.write_all(MIME.as_bytes()).map_err(io)?;
    for (name, body) in [
        ("META-INF/manifest.xml", MANIFEST.to_string()),
        ("meta.xml", meta_xml(&doc.meta)),
        ("styles.xml", styles_xml(&doc.meta)),
        ("content.xml", content_xml(doc)),
    ] {
        zip.start_file(name, deflated).map_err(zerr)?;
        zip.write_all(body.as_bytes()).map_err(io)?;
    }
    // PNG data is already compressed.
    zip.start_file("Pictures/logo.png", stored).map_err(zerr)?;
    zip.write_all(super::pdf::LOGO_PNG).map_err(io)?;
    Ok(zip.finish().map_err(zerr)?.into_inner())
}
