# Report documents

Report documents (issue #354) are PDF or ODT files built from a **template**. The template lists the sections, sets their order and options, and holds the document fields (ID, issue, signatories, change record, reference documents). There are two report types:

| Type | Key | Sections |
| --- | --- | --- |
| Verification Control Document (ECSS-E-ST-10-02C Annex B) | `vcd` | cover, change record, contents, introduction, documents, definitions, approach, summary, matrix, open items, traceability checks |
| Traceability & coverage report | `coverage` | cover, change record, contents, introduction, documents, coverage summary, coverage by group, uncovered requirements, orphan verifications, suspect links, data quality, approval and authors |

`GET …/exports/report.pdf` (the Reports page's *Report PDF*) renders the built-in coverage template.

## Code map (`marreq-core/src/reports/`)

| File | Role |
| --- | --- |
| `definition.rs` | `ReportDefinition` (serde, `deny_unknown_fields`), its defaults (`default_for`) and validation (`validated`). |
| `sections.rs` | The section catalog (`catalog()`: key, title, report types, option schema) and the renderers. |
| `data.rs` | `load()`: everything a report needs about one project, read once (requirements, verifications, links, close-out from `verification_control_service`). |
| `model.rs` | The format-neutral document: `Block`, `Inline`, `Table`, `Cell`… Sections number their headings here, so both formats use the same numbers. |
| `pdf.rs` + `typst/marreq.typ` | PDF: the model is passed to Typst as **JSON** and laid out by a fixed Typst library, compiled in-process (fonts embedded from `fonts/`; no file or network access). |
| `odt.rs` | ODT: ODF XML written directly into a zip (`mimetype` first and stored). |

### Definition rules

- `sections` order is the document order; `enabled: false` leaves a section out; `options` are checked against the section's option schema (`select`, `multi_select`, `text`).
- An unknown key, a section listed twice, or a section of another report type is **400**.
- Sections a stored template does not list (added in a later release) are appended **disabled**, so a saved template keeps producing the same document.

### Adding a section

1. Add a `SectionSpec` to `catalog()` (key, title, description, report types, default state, options).
2. Add its arm to `render_section()`; use `ctx.heading(level, text)` for numbered headings.
3. Add a test in `sections.rs` (and check both outputs render, see `mod.rs` tests).

### Text is data, never markup

Every user string reaches Typst as a JSON string, so it is printed literally (see `user_text_with_typst_syntax_is_printed_literally`). Don't build Typst markup from user text. The ODT writer escapes XML and drops control characters.

### Fonts

`fonts/` holds Inter (SIL OFL 1.1) and DejaVu Sans Mono (Bitstream Vera licence); the licence texts are next to them. The PDF embeds the subsets it uses. The ODT names the fonts; LibreOffice substitutes them when they are not installed.

## API

See [`http-api-contract.md`](http-api-contract.md): `GET …/reports/types`, `…/report_templates` (CRUD) and `POST …/reports/{vcd|coverage}.{pdf|odt}`.

## Checking output

- PDF: `pdfinfo` (pages, landscape sizes) and `pdftoppm -png` to look at pages.
- ODT: `soffice --headless --convert-to pdf file.odt` must succeed (LibreOffice reads the package), then inspect the pages the same way.
