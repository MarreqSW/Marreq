# ReqIF 1.2 support

Marreq supports ReqIF 1.2 as an interchange format for requirements. This note documents the supported behavior for issue #71.

## Supported flows

- Export current project requirements as ReqIF XML.
- Export an immutable baseline snapshot as ReqIF XML.
- Import ReqIF XML into a project and create requirements.
- Preserve parent-child requirement relations where the ReqIF file contains relation data.
- Include requirement comments in the exported `Remarks` attribute when present.

## Field mapping

| Marreq field | ReqIF representation |
| --- | --- |
| `reference_code` | requirement reference attribute |
| `title` | title or summary attribute |
| `description` | description text attribute |
| requirement status | status attribute |
| `justification` | justification attribute |
| parent links | ReqIF relation entries |
| comments | `Remarks` attribute on export |

## Import defaults

ReqIF does not always contain every Marreq-specific catalog value. Import requires project-local defaults for author, reviewer, category, applicability, verification method, and fallback status.

## Validation checklist

1. Export a project and confirm the XML contains `REQ-IF` and requirement objects.
2. Export a baseline and confirm it uses the baseline snapshot.
3. Import a ReqIF file with a parent-child relation and confirm a requirement version link is created.
4. Import a status with different casing and confirm it resolves to the existing project status.
5. Export a requirement with comments and confirm the comments appear in `Remarks`.

## Code entry points

- `marreq-core/src/services/reqif_service.rs`
- `marreq-core/src/reqif/import.rs`
- `marreq-core/src/reqif/mapping.rs`
- `marreq-core/src/reqif/export.rs`
- `marreq-core/src/reqif/mod.rs`

Related issue: #71
