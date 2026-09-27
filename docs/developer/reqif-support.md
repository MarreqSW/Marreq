# ReqIF 1.2 support

Marreq provides partial ReqIF 1.2 interoperability. “Imports without an error”
does not imply lossless import. See the fixture-backed
[ReqIF import compatibility report](reqif-import-compatibility-report.md) for
the tested tools, semantic matrix, severity-ranked findings, schema results and
known limitations.

## Supported flows

- Export current project requirements as ReqIF XML.
- Export an immutable baseline snapshot as ReqIF XML.
- Import ReqIF XML into a project and create requirements.
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
images, nesting) and attachments are not currently persisted. The import result reports these losses as warnings. ReqIFZ is not
supported.

The Rocket API exposes the same service:

- `GET /api/projects/{project_id}/exports/requirements.reqif`
- `GET /api/projects/{project_id}/exports/baselines/{baseline_id}.reqif`
- `POST /api/projects/{project_id}/imports/reqif`

The SPA downloads live requirements ReqIF from **Reports**, baseline ReqIF from
the baseline detail page, and imports `.reqif`/`.xml` from **Import**.

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
