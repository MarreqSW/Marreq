# ReqIF 1.2 support

Marreq provides partial ReqIF 1.2 interoperability. “Imports without an error”
does not imply lossless import. See the fixture-backed
[ReqIF import compatibility report](reqif-import-compatibility-report.md) for
the tested tools, semantic matrix, severity-ranked findings, schema results and
known limitations.

## Supported flows

- Export current project requirements as ReqIF XML, or as a ReqIFZ archive
  with their attachment files.
- Export an immutable baseline snapshot as ReqIF XML or ReqIFZ.
- Import ReqIF XML or a ReqIFZ archive into a project and create
  requirements; files referenced from XHTML become attachments.
- Preserve nested `SPEC-HIERARCHY` as parent-child requirement links.
- Import representable `SPEC-RELATION` entries as typed requirement links.
- Include requirement comments in the exported `Remarks` attribute when present.
- Emit schema-valid ReqIF XML with nested hierarchy.

## Field mapping

| Marreq field | ReqIF representation |
| --- | --- |
| `reference_code` | requirement reference attribute |
| `title` | title or summary attribute |
| `description` | `Statement` XHTML attribute on export (`ATTRIBUTE-VALUE-XHTML`); on import, the description/statement attribute (STRING or XHTML) |
| requirement status | status attribute |
| `justification` | justification attribute |
| parent links | ReqIF relation entries |
| comments | `Remarks` attribute on export |

### Statement formatting (XHTML)

Requirement statements are stored as "Marreq statement Markdown"
(`marreq-core/src/rich_text.rs`, mirrored in `frontend/src/utils/statementMarkdown.ts`):
paragraphs, `- ` bulleted and `1. ` numbered lists, `**bold**`, `*italic*`,
`` `code` `` and `[label](url)` links (http/https/mailto only).

- **Export** declares `xmlns:xhtml="http://www.w3.org/1999/xhtml"`, a
  `DATATYPE-DEFINITION-XHTML` (`dt-xhtml`) and an `ATTRIBUTE-DEFINITION-XHTML`
  `Statement` (`ad-statement`). Each value is
  `<xhtml:div>` with `xhtml:p`, `ul`, `ol` (`start` when not 1), `li`,
  `strong`, `em`, `code`, `a href` and `br`; all text is escaped. Identifier,
  title, rationale and remarks stay STRING attributes.
- **Import** converts XHTML back to the same Markdown instead of flattening it:
  `p`/`div`/headings become paragraphs, `ul`/`ol`/`li` become list items,
  `strong`/`b`, `em`/`i`, `code`/`tt`, `a` (safe `href` only) and `br` map to
  their Markdown forms, table rows become separate lines, and other elements
  keep their text. Marker characters in the source text are escaped. Entity
  references (`&lt;`, `&amp;`, `&#8364;`, …) are resolved in element text and
  attribute values.
- The 2000-byte statement limit applies after conversion; longer imported
  statements are truncated (at a character boundary) with a warning.

Unmapped attributes, datatypes, object types, specification boundaries,
original identifiers, XHTML formatting beyond the subset above (tables,
images, nesting) are not currently persisted. The import result reports these
losses as warnings.

On export, each requirement's attachment file names are written to an
`Attachments` string attribute (`ad-attachments`, `"; "`-separated).

## ReqIFZ (issue #343)

A ReqIFZ archive is a ZIP file with one or more `.reqif` documents and the
files they reference.

- **Export** writes `<name>.reqif` at the root and each requirement attachment
  at `files/<attachment id>/<file name>`. The statement XHTML gains one
  `<xhtml:object class="marreq-attachment" data="files/…" type="…">name</xhtml:object>`
  per file (images first, paths percent-encoded). A file missing from the blob
  store is left out. Baseline archives use the files the baseline recorded.
- **Import** detects the ZIP signature on the normal ReqIF import route and:
  - opens the archive with strict checks (`reqif/archive.rs`): no absolute
    or `..` paths, symlinks, encrypted or duplicate entries; at most 10,000
    entries, 1 GiB uncompressed in total and a 100:1 compression ratio for
    entries above 1 MiB; documents are read up to 64 MiB;
  - imports every `.reqif` in archive order, reserving reference codes across
    documents and prefixing messages with the document name when there are
    several;
  - resolves each `<xhtml:object data>` relative to its document and attaches
    the file through `AttachmentService::create` (type allowlist, per-file
    limit, project quota). Failures become warnings. Marreq's own object
    fallback text is not copied into statements, so round trips stay clean.
  - Archives are limited by `MARREQ_REQIFZ_MAX_MB` (default 100); plain XML
    imports keep the 20 MiB limit. `FILE-NAME` / `ATTRIBUTE-VALUE-ATTACHMENT`
    embeddings are not imported.

The Rocket API exposes the same service:

- `GET /api/projects/{project_id}/exports/requirements.reqif` (and `.reqifz`)
- `GET /api/projects/{project_id}/exports/baselines/{baseline_id}.reqif` (and `.reqifz`)
- `POST /api/projects/{project_id}/imports/reqif` (`.reqif`, `.xml` or ReqIFZ)

The SPA downloads live requirements ReqIF and ReqIFZ from **Reports**, baseline
ReqIF and ReqIFZ from the baseline detail page, and imports `.reqif`/`.xml`/
`.reqifz` from **Import**.

## Import defaults

ReqIF does not always contain every Marreq-specific catalog value. Import requires project-local defaults for author, reviewer, category, applicability, verification method, and fallback status.

## Validation checklist

1. Export a project and confirm the XML contains `REQ-IF` and requirement objects.
2. Export a baseline and confirm it uses the baseline snapshot.
3. Import a ReqIF file with a parent-child relation and confirm a requirement version link is created.
4. Import a status with different casing and confirm it resolves to the existing project status.
5. Export a requirement with comments and confirm the comments appear in `Remarks`.
6. Run `cargo test -p marreq-core --lib reqif -- --test-threads=1` to exercise
   the real vendor fixture suite under `tests/reqif/fixtures`.

## Code entry points

- `marreq-core/src/services/reqif_service.rs`
- `marreq-core/src/reqif/import.rs`
- `marreq-core/src/reqif/mapping.rs`
- `marreq-core/src/reqif/export.rs`
- `marreq-core/src/reqif/mod.rs`
- `marreq-core/src/api/exports.rs`
- `marreq-core/src/api/imports.rs`

Related issue: #71
