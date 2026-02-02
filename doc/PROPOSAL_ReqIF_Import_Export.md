# Proposal: ReqIF 1.2 Import and Export Support

## 1. Overview

This document proposes adding **Requirements Interchange Format (ReqIF)** [1.2](https://www.omg.org/spec/ReqIF/1.2/) support to ReqMan for importing and exporting requirements data. ReqIF is an OMG standard for exchanging requirements between tools and organizations that do not share the same repository.

**Goals:**
- **Export:** Generate ReqIF 1.2 XML from a ReqMan project (requirements, optional hierarchy and attributes).
- **Import:** Parse ReqIF 1.2 XML and create/update requirements (and optionally tests) in a ReqMan project, with configurable mapping and conflict handling.

**References:**
- [OMG ReqIF 1.2 – About the specification](https://www.omg.org/spec/ReqIF/1.2/)
- Normative XML Schema: [reqif.xsd](https://www.omg.org/spec/ReqIF/20110401/reqif.xsd), [driver.xsd](https://www.omg.org/spec/ReqIF/20110402/driver.xsd)

---

## 2. ReqIF 1.2 Concepts (Relevant Subset)

| ReqIF concept                                | Description                                                                                              |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| **ReqIF**                                    | Root element; contains header (title, source tool, etc.) and core content.                               |
| **Specification**                            | Container for a set of requirements; can be nested (chapter-like hierarchy).                             |
| **SpecObject**                               | One “requirement” or similar artifact; has a type and attribute values.                                  |
| **SpecObjectType**                           | Defines which attributes a SpecObject has (e.g. Title, Description, Identifier).                         |
| **AttributeDefinition** / **AttributeValue** | Typed attributes (String, XHTML, Enum, etc.) attached to SpecObjects.                                    |
| **SpecRelation**                             | Relation between two SpecObjects (e.g. “derived”, “refines”); can represent parent-child or trace links. |
| **DatatypeDefinitions**                      | Types for attributes (String, Enumeration, XHTML, etc.).                                                 |

ReqIF is XML-based; the schema is normative. Export must produce valid ReqIF 1.2 XML; import should accept it and optionally validate against the XSD.

---

## 3. Mapping: ReqIF ↔ ReqMan

### 3.1 Export (ReqMan → ReqIF)

| ReqMan                             | ReqIF                                                                                                                            |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **Project**                        | ReqIF header (e.g. title = project name); one or more Specifications.                                                            |
| **Requirement**                    | One **SpecObject** per requirement.                                                                                              |
| **Requirement.title**              | Attribute “Title” (STRING).                                                                                                      |
| **Requirement.description**        | Attribute “Statement” or “Description” (XHTML or STRING).                                                                        |
| **Requirement.reference_code**     | Attribute “Identifier” or “ReqId” (STRING).                                                                                      |
| **Requirement.status_id**          | Attribute “Status” (ENUMERATION) – map to ReqIF enum or leave as string.                                                         |
| **Requirement.justification**      | Optional attribute “Rationale” (STRING).                                                                                         |
| **Requirement.parent_id**          | **SpecRelation** “parent–child” or “containment” from child to parent SpecObject.                                                |
| **Category / Verification / etc.** | Optional attributes or enum values if we want full fidelity; otherwise omit or summarize in a single “Classification” attribute. |

**Decisions for MVP:**
- One **Specification** per project (flat or one level of grouping).
- **SpecObjectType**: single type “Requirement” with attributes: Identifier, Title, Statement (XHTML), Status (optional), Rationale (optional).
- **SpecRelation**: only parent–child between requirements (same project).
- **Tests**: Out of scope for first version, or export as a second Specification with type “TestCase” and SpecRelations for traceability.

### 3.2 Import (ReqIF → ReqMan)

| ReqIF                                          | ReqMan                                                                                      |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------- |
| **SpecObject** (of chosen type)                | **Requirement** (or skip/ignore non-requirement types).                                     |
| Attribute “Title” / “Identifier” / “Statement” | **title**, **reference_code**, **description**.                                             |
| Attribute “Status” (enum)                      | Map to **status_id** (project’s requirement statuses); default to “Draft” if unknown.       |
| **SpecRelation** (e.g. parent–child)           | **parent_id** (if both ends are imported as requirements).                                  |
| **Specification** hierarchy                    | Optionally create parent requirements for chapters/sections, or flatten and drop structure. |

**Decisions for MVP:**
- **Target project** selected by user at import time (like Excel import).
- **Attribute mapping:** Configurable mapping from ReqIF attribute long-name (or ID) to ReqMan field (title, description, reference_code, status, rationale). Default mapping for common names (Title → title, etc.).
- **IDs:** ReqIF uses UUIDs; ReqMan uses integer IDs. Do not preserve ReqIF IDs as ReqMan IDs; generate new IDs. Optionally store ReqIF object ID in a custom field or mapping table for “re-import” or round-trip hints later.
- **Conflict handling:** “Always create new” vs “Match by Identifier and update if exists” (optional, configurable).

---

## 4. Proposed Architecture

### 4.1 New Modules / Crates

- **`src/reqif/`** (or optional crate `reqif_io`):
  - **`mod.rs`** – re-exports and config.
  - **`schema.rs`** – ReqIF 1.2 structures (parsed from XSD or hand-written minimal set: ReqIF, Specification, SpecObject, SpecObjectType, SpecRelation, AttributeDefinition, AttributeValue, DatatypeDefinition). Prefer **serde**-compatible structs for XML (de)serialization.
  - **`export.rs`** – Build ReqIF XML from `Vec<Requirement>` (and optional hierarchy, project name). Use an XML writer (e.g. `quick-xml` with serde, or `xml-rs`) and produce valid ReqIF 1.2.
  - **`import.rs`** – Parse ReqIF XML into an intermediate model (e.g. `ReqIFDocument`), then map to ReqMan `NewRequirement` + optional `parent_id` resolution. Return list of created/updated requirements and errors per row (similar to Excel import).
  - **`mapping.rs`** (optional) – Attribute name ↔ ReqMan field mapping; defaults for “Title”, “Description”, “Identifier”, “Status”, “Rationale”.

### 4.2 Services

- **`ReqIFService`** (in `src/services/reqif_service.rs` or under `src/reqif/`):
  - **`export_project(project_id, options?)`** – Load project + requirements (and parent links); call `reqif::export::to_reqif(...)`; return XML string or bytes.
  - **`import_into_project(project_id, xml_bytes, config)`** – Call `reqif::import::parse(...)` then, for each SpecObject, resolve attributes via mapping, resolve parent from SpecRelations, then create (or update) requirements via existing `RequirementService` / repository. Return summary (count, errors) and optionally list of new IDs for indexing (like Excel import).

### 4.3 API and UI

- **REST API (optional but recommended):**
  - **`GET /api/projects/<id>/export/reqif`** – Return ReqIF XML (Content-Type: `application/xml` or ReqIF-specific).
  - **`POST /api/projects/<id>/import/reqif`** – Body: multipart file upload (`.reqif` or `.xml`); optional JSON config (mapping, conflict mode). Response: JSON `{ success, message, imported_count, errors, imported_requirement_ids }` (aligned with Excel import).
- **HTML UI (aligned with Excel import):**
  - **Export:** Button/link on project or requirements page: “Export as ReqIF” → triggers download of ReqIF file (e.g. `GET /p/<project_id>/export_reqif` returning attachment).
  - **Import:** New page `GET /p/<project_id>/import_reqif` with upload form; after upload, optional “mapping” step (ReqIF attribute → ReqMan field) then “Process”; result page shows count and errors (like `/p/<id>/import_excel`).

### 4.4 Dependencies

- **XML parsing / writing:**  
  - **`quick-xml`** with **`serde`** for (de)serializing ReqIF structs, or  
  - **`roxmltree`** for read-only parsing + manual mapping (no full XSD compliance).  
  Prefer one library for both import and export for consistency.
- **XSD validation (optional):** Use **`libxml`** or an external validator for “strict” mode; not required for MVP.

### 4.5 Integration with Existing Code

- Reuse **project/requirement loading** (e.g. `RequirementService`, `DecoratedRequirementService`) and **permission checks** (project access, same as Excel import/export).
- Reuse **post-import** behavior: invalidate caches, optionally queue **semantic search indexing** for imported requirement IDs (same as Excel in `src/routes/html/excel.rs`).
- Reuse **logging** (e.g. export/import actions in `LogService`) if applicable.
- **No change** to core requirement or test models; ReqIF is an additional I/O format.

---

## 5. Implementation Phases

### Phase 1 – Export (MVP)
1. Add `src/reqif/` with minimal ReqIF 1.2 structures (ReqIF root, Specification, SpecObject, SpecObjectType, attribute values, SpecRelation for parent).
2. Implement `reqif::export::to_reqif(project_name, requirements, parent_map)` → XML string.
3. Add `ReqIFService::export_project(project_id)` using existing services to load data.
4. Add route `GET /p/<project_id>/export_reqif` (and optionally `GET /api/projects/<id>/export/reqif`) returning ReqIF file.
5. Add “Export ReqIF” on project/requirements UI.

### Phase 2 – Import (MVP)
1. Implement `reqif::import::parse(xml_bytes)` → intermediate model (list of SpecObjects + SpecRelations + types).
2. Implement mapping from ReqIF attributes to ReqMan fields (with configurable defaults).
3. Implement `ReqIFService::import_into_project(project_id, xml, config)` creating requirements and resolving parent_id from SpecRelations.
4. Add route `POST /p/<project_id>/import_reqif` (upload + process) and optional mapping step; result page like Excel import.
5. Invalidate caches and queue semantic indexing for imported IDs.

### Phase 3 – Enhancements (Optional)
- **Tests in ReqIF:** Export test cases as second Specification + SpecRelations to requirements; import tests and matrix links.
- **XSD validation** for import (strict mode).
- **Round-trip:** Preserve ReqIF object IDs in a mapping table for “update if exists” on re-import.
- **Rich text:** ReqIF XHTML for description; convert to/from ReqMan’s description format (e.g. HTML or Markdown).

---

## 6. File Layout (Suggested)

```
src/
  reqif/
    mod.rs
    schema.rs    # ReqIF 1.2 structs (minimal)
    export.rs
    import.rs
    mapping.rs   # optional
  services/
    reqif_service.rs
  routes/
    html/
      reqif.rs   # GET export_reqif, GET/POST import_reqif
```

---

## 7. Risks and Mitigations

| Risk                                          | Mitigation                                                                                                        |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| ReqIF schema is large and complex             | Implement only the subset needed for SpecObject + Specification + SpecRelation; ignore unused elements on import. |
| Different tools use different attribute names | Configurable attribute→field mapping and sensible defaults (Title, Description, Identifier).                      |
| Large files (memory)                          | Stream parsing for import (e.g. `quick-xml` with Reader); limit file size in route.                               |
| Invalid or non–ReqIF XML                      | Catch parse errors; return user-friendly message and optional validation mode.                                    |

---

## 8. Summary

- **Export:** Generate ReqIF 1.2 XML from project requirements (and parent hierarchy), using a minimal but valid subset of the schema.
- **Import:** Parse ReqIF XML, map SpecObjects to requirements with configurable attribute mapping, resolve parent from SpecRelations, create (or optionally update) requirements in a chosen project; reuse existing import UX and post-import logic.
- **Scope:** One new module `src/reqif/`, one service `ReqIFService`, and routes + UI similar to Excel import/export; no change to core domain models.

This keeps ReqIF support consistent with existing ReqMan architecture and the [OMG ReqIF 1.2](https://www.omg.org/spec/ReqIF/1.2/) specification while allowing incremental delivery (export first, then import, then enhancements).
