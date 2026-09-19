# REQIF IMPORT COMPATIBILITY REPORT

Marreq  
Date: 2026-09-19

## Summary

This audit exercised the real Rust entry point
`ReqIFService::import_into_project`, including `RequirementService` validation
and requirement-version-link creation. Each fixture gets a fresh
`DieselRepoMock` project with controlled defaults. This validates the production
service path but not PostgreSQL transaction behavior or an HTTP endpoint; no
ReqIF endpoint currently exists.

| Result | Count |
| --- | ---: |
| Vendor files inspected | 20 |
| Lossless PASS | 0 |
| PARTIAL | 15 |
| FAIL / unsupported | 5 |
| Synthetic robustness files | 10 |

`success=true` in `ImportResult` means that database/service operations
completed without a hard error. It does **not** mean lossless interoperability.
The compatibility result below treats any meaningful discarded information as
PARTIAL.

## Current mapping

| ReqIF | Marreq | Fidelity |
| --- | --- | --- |
| `SPEC-OBJECT` | one requirement and current requirement version | Supported |
| `SPEC-OBJECT-TYPE` | all types collapse to the Marreq requirement model | Partial; type identity is warned and lost |
| `SPEC-HIERARCHY` | `requirement_version_links`, child → parent, `DERIVES_FROM` | Supported for valid nested references |
| `SPECIFICATION` | target Marreq project | Partial; multiple specifications merge |
| `SPEC-RELATION` | typed `requirement_version_links` | Partial; model is acyclic and cannot represent all ReqIF relation graphs |
| `SPEC-RELATION-TYPE` | heuristic mapping to `DERIVES_FROM`, `REFINES`, `DEPENDS_ON`, `SATISFIES`, or `RELATES_TO` | Partial; relation attributes and arbitrary vendor types are not preserved |
| `ATTRIBUTE-DEFINITION-*` | lookup metadata used to map values | Partial; definitions are not created as Marreq custom fields |
| `DATATYPE-DEFINITION-*` | used indirectly while parsing values | Partial; datatype constraints are not persisted |
| string values | title/reference/description/status/justification where names map | Supported for mapped core fields |
| integer/real/boolean/date values | parsed as strings | Partial; warned, usually discarded unless their attribute name maps to a core field |
| enumeration values | enum labels joined as text | Partial; warned, not persisted as enum custom fields |
| XHTML | text content flattened into description/core fields | Partial; formatting, tables, links and image structure are lost |
| `IDENTIFIER` | temporary import key; fallback Marreq reference generated when needed | Partial; original ReqIF ID is not persisted |
| `LAST-CHANGE` | parsed on `SPEC-OBJECT` | Ignored with warning; Marreq creation time is used |
| users/authors | import actor/default author and reviewer | ReqIF author metadata is not mapped |
| attachments / XHTML objects | detected | Unsupported; warning emitted |
| ReqIFZ | none | Unsupported; rejected as non-XML |

Attribute-name mappings include:

- title: `Title`, `ReqIF.Name`, `ReqIF.ChapterName`, vendor `*-TITLE`;
- reference: `Identifier`, `ReqId`, `ReqIF.ForeignID`;
- description: `Statement`, `Description`, `ReqIF.Text`, vendor `*-TXT`;
- status: `Status`, vendor `*-STATUS`;
- justification: `Rationale`.

References that do not satisfy Marreq's reference syntax receive stable
per-import `REQ-0001`, `REQ-0002`, … fallbacks. This prevents professional-tool
UUIDs from failing requirement validation, but the original ID remains only an
in-memory lookup key.

## By tool

| Tool/source | Files | Compatibility | Notes |
| --- | ---: | --- | --- |
| IBM Rational DOORS | 4 | 0 PASS / 4 PARTIAL | Objects import; XHTML is flattened; enum/date/vendor attributes are not persisted |
| Polarion | 5 | 0 PASS / 4 PARTIAL / 1 FAIL | Hierarchy imports; custom fields/XHTML are lossy; ReqIFZ unsupported; MAG8000 has 26 external relations skipped with warnings |
| Eclipse RMF | 3 | 0 PASS / 3 PARTIAL | Objects and the relation sample import; two files use the older 2010 namespace |
| Eclipse Capella fixtures | 3 | 0 PASS / 3 PARTIAL | Objects and relation import; XHTML/enumerations are lossy |
| Enterprise Architect example | 2 copies | 0 PASS / 2 FAIL | Both contain references to `FUNC-REQ-1/2`, while objects are `R001/2/3`; preflight imports nothing |
| StrictDoc | 3 | 0 PASS / 2 PARTIAL / 1 FAIL | Native 18-object hierarchy imports; minimal fixture is intentionally schema-incomplete |
| ReqIF Studio | 1 | 0 PASS / 1 FAIL | Missing hierarchy object causes preflight rejection; no partial writes |

No downloaded fixture could be positively identified as IBM DOORS Next/DNG.
This audit therefore makes no DOORS Next compatibility claim.

## Semantic fixture matrix

`Hierarchy` is the number of child→parent edges, not the number of
`SPEC-HIERARCHY` nodes. `Links` is what Marreq created.

| Fixture | ReqIF objects | Marreq requirements | Hierarchy | Relations | Marreq links | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Capella `Sample.xml` | 1 | 1 | 0 | 0 | 0 | PARTIAL |
| Capella `Sample1.xml` | 2 | 2 | 0 | 1 | 1 | PARTIAL |
| Capella `Sample3.xml` | 1 | 1 | 0 | 0 | 0 | PARTIAL |
| DOORS official Capella `Sample.reqif` | 1 | 1 | 0 | 0 | 0 | PARTIAL |
| StrictDoc DOORS date sample | 1 | 1 | 0 | 0 | 0 | PARTIAL |
| StrictDoc DOORS user sample | 1 | 1 | 0 | 0 | 0 | PARTIAL |
| RMF sample | 2 | 2 | 0 | 0 | 0 | PARTIAL |
| RMF relation sample | 2 | 2 | 0 | 1 | 1 | PARTIAL |
| RMF multi-spec sample | 6 | 6 | 0 | 0 | 0 | PARTIAL |
| EA example (LutaML) | 3 | 0 | 1 | 1 | 0 | FAIL |
| EA 8 example (StrictDoc) | 3 | 0 | 1 | 1 | 0 | FAIL |
| Polarion MAG8000 | 102 | 102 | 100 | 26 | 100 | PARTIAL |
| Polarion small XHTML sample | 1 | 1 | 0 | 0 | 0 | PARTIAL |
| Polarion section sample | 2 | 2 | 1 | 0 | 1 | PARTIAL |
| Polarion anonymized sample | 101 | 101 | 90 | 0 | 90 | PARTIAL |
| ReqIF Studio | 137 | 0 | 128 | 14 | 0 | FAIL |
| StrictDoc minimal | 0 | 0 | 0 | 0 | 0 | FAIL (schema-incomplete) |
| StrictDoc/DOORS large input | 3 | 3 | 0 | 0 | 0 | PARTIAL |
| StrictDoc native sample | 18 | 18 | 14 | 0 | 14 | PARTIAL |
| Polarion ReqIFZ | 2 inside archive | 0 | 1 | 0 | 0 | FAIL (unsupported container) |

For MAG8000, all 26 relation targets refer to objects outside the exchanged
object set. The importer now reports every skipped relation. It does not
misrepresent them as hierarchy.

## ReqIF 1.2 schema validation

Validation used the complete schema/XHTML bundle from
`strictdoc-project/reqif/reqif/reqif_schema`. The standalone XSD already present
under `docs/ReqIF` imports remote `xml.xsd` and is not self-contained offline.
The schema is the same normative `dtc/11-04-05` machine-readable document
listed by the [OMG ReqIF 1.2 specification](https://www.omg.org/spec/ReqIF/1.2/);
the OMG page also identifies `dtc/11-04-06` as the normative XHTML driver.

Several LutaML fixtures prepend a provenance comment before the XML declaration.
That is not well-formed XML. The original bytes are preserved and accepted by
Marreq/quick-xml. A second schema pass beginning at the XML declaration was used
to distinguish this packaging issue from ReqIF content validity.

Content valid against the bundled schema:

- official Capella/DOORS `Sample.reqif`;
- LutaML Capella `Sample.xml` and `Sample1.xml` after removing only the
  pre-declaration provenance comment in memory;
- Polarion MAG8000 after the same in-memory preamble removal;
- RMF multi-spec fixture after the same preamble removal;
- large StrictDoc/DOORS input after the same preamble removal.

Notable schema/semantic issues:

- Capella `Sample3`: missing required `LAST-CHANGE`;
- legacy RMF samples: 2010 namespace, not the 20110401 ReqIF 1.2 schema;
- EA examples: dangling `FUNC-REQ-1/2` IDREFs;
- Polarion small sample: incomplete enum value properties;
- Polarion section sample: duplicate `xs:ID`;
- anonymized Polarion: deliberately invalid anonymized date values (`aaa`);
- ReqIF Studio: dangling IDREFs;
- StrictDoc native sample: missing required `LAST-CHANGE`/`MAX-LENGTH`;
- StrictDoc minimal: intentionally incomplete;
- DOORS user/date samples: missing required structure or IDREF targets.

Marreq export was generated from a two-level requirement tree and validated
successfully against the same schema bundle after this audit's exporter fixes.

## Feature assessment

| Feature | Result |
| --- | --- |
| basic ReqIF structure | Supported; root and malformed XML are validated |
| namespaces | Default and prefixed namespaces supported by local-name parsing |
| string attributes | Supported for mapped fields |
| integer/real/boolean/date | Parsed, but typed persistence unsupported |
| enumerations | Labels parsed; enum custom-field semantics lost |
| nested hierarchy | Preserved when references are valid |
| multiple specifications | Objects import; document boundaries lost |
| multiple object types | Objects import; type identity lost |
| relations | Imported when endpoints exist and Marreq's acyclic graph accepts them |
| relation attributes | Unsupported; warning |
| XHTML text | Preserved as plain text |
| XHTML formatting/tables/links | Unsupported |
| images/embedded files | Unsupported; warning |
| unknown elements/attributes | Tolerated |
| malformed XML / wrong root | Rejected without panic |
| missing hierarchy IDs | Preflight rejection with zero writes |
| missing relation endpoints | Valid objects import; relation skipped with warning |
| duplicate object IDs/references | Preflight rejection with zero writes |
| ReqIFZ | Unsupported |

## Findings

### REQIF-IMP-001 — FIXED

Severity: CRITICAL  
Feature: standard nested references

Expected: parse `DEFINITION`, `TYPE`, `SOURCE`, `TARGET`, `OBJECT` and their
`*-REF` children.  
Previous actual: Marreq expected non-standard XML attributes and silently
created mostly empty requirements from real files.  
Fix: namespace-local streaming parser now handles OMG nested references while
retaining compatibility with historical Marreq attribute-style XML.

### REQIF-IMP-002 — FIXED

Severity: HIGH  
Feature: `SPEC-HIERARCHY`

Previous actual: hierarchy was ignored.  
Fix: nested hierarchy edges are parsed and created as `DERIVES_FROM` version
links. Structural link failures are returned as errors.

### REQIF-IMP-003 — PARTIALLY FIXED

Severity: HIGH  
Feature: `SPEC-RELATION`

Previous actual: endpoint attributes were expected, relation type was ignored,
every relation became `DERIVES_FROM`, and link errors were discarded.  
Fix: nested endpoints/type refs parse; known type names map to Marreq link
types; every skipped relation is warned.  
Remaining: Marreq stores hierarchy and traceability in one acyclic graph.
Valid ReqIF relation graphs that conflict with hierarchy cannot be represented.

### REQIF-IMP-004 — OPEN

Severity: HIGH  
Feature: custom attributes and datatypes

Expected: preserve definitions, typed values and enumerations.  
Actual: values can be parsed, but only five core Marreq fields are persisted.
Other values are listed in warnings.  
Impact: vendor custom metadata, dates, booleans, real/integer constraints and
enumeration semantics are lost.

### REQIF-IMP-005 — OPEN

Severity: HIGH  
Feature: XHTML and attachments

Expected: preserve rich XHTML, tables, links, images and embedded files.  
Actual: text is flattened; markup and attachments are not stored.  
Impact: presentation and embedded evidence are lost. The importer now warns.

### REQIF-IMP-006 — FIXED

Severity: HIGH  
Feature: invalid/dangling references and duplicate IDs

Previous actual: missing parents/targets were silently skipped after some rows
had already been committed.  
Fix: duplicate object IDs/references and invalid hierarchy references fail
preflight before writes. Missing external relation endpoints remain tolerable
but are explicit warnings.

### REQIF-IMP-007 — OPEN

Severity: HIGH  
Feature: transactionality

Actual: preflight prevents known structural failures from causing writes, and
each requirement creation is atomic. The complete import is still not wrapped
in one PostgreSQL transaction. An unexpected repository failure on row N can
leave rows 1…N-1 committed.

### REQIF-IMP-008 — OPEN

Severity: MEDIUM  
Feature: identifiers and timestamps

Actual: ReqIF IDs drive in-memory reference resolution but are not stored;
invalid Marreq reference formats receive generated references. `LAST-CHANGE`
is not persisted.  
Impact: external identity and exact round-trip IDs/timestamps are lost.

### REQIF-IMP-009 — OPEN

Severity: MEDIUM  
Feature: specifications and object types

Actual: multiple specifications and all object types collapse into one Marreq
project/model. Warnings are emitted.

### REQIF-IMP-010 — OPEN

Severity: HIGH  
Feature: ReqIFZ

Actual: ZIP bytes are rejected as malformed XML. No attachment extraction or
path-safety handling exists.

### REQIF-EXP-001 — FIXED

Severity: CRITICAL  
Feature: ReqIF export conformance

Previous actual: exporter omitted `REQ-IF-HEADER`, used attributes instead of
nested refs, referenced a `SPEC-OBJECT-TYPE` from `SPECIFICATION`, emitted an
XSD-invalid version value, and flattened hierarchy.  
Fix: exporter emits schema-valid ReqIF, nested refs, required type definitions,
timestamps, nested hierarchy and typed parent relations. A generated export was
validated against the StrictDoc schema bundle.

### REQIF-EXP-002 — OPEN

Severity: HIGH  
Feature: semantic round-trip

Core title/reference/description/justification and hierarchy round-trip.
Status, custom fields, original ReqIF identifiers, datatypes, enumerations,
XHTML formatting, attachments, users and arbitrary relation metadata do not.

### REQIF-OPS-001 — OPEN

Severity: MEDIUM  
Feature: accessibility

`ReqIFService` is not exposed through the current Rocket API or SPA. Automated
tests call the internal production service directly.

## StrictDoc implementation observations

StrictDoc's parser explicitly supports all seven attribute value kinds,
preserves XHTML strings, parses editable hierarchy attributes, and its
validator separately reports XML, XSD and semantic reference errors. The audit
adopted the same important distinction: schema-validity and semantic
interoperability are separate results.

## Round-trip result

The automated round-trip test covers:

```text
synthetic ReqIF -> Marreq service -> Marreq ReqIF exporter -> parser
```

It verifies two requirements, core text fields and one parent link. The
exported XML is schema-valid. This is a **partial round-trip**, not a lossless
vendor round-trip, because Marreq has nowhere to persist the open features
listed above.

## Verification results

Final repository verification:

```text
Tests run:     1858
Tests passed:  1857
Tests failed:  0
Tests skipped: 1

Rust tests:    1709 passed, 0 failed, 1 skipped
Frontend:       148 passed, 0 failed, 0 skipped
ReqIF subset:    51 passed, 0 failed, 0 skipped
```

Additional checks:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --features test-helpers --all-targets -- -D warnings`: pass
- `npm run build`: pass (existing Vite chunk-size warning only)
- generated Marreq ReqIF export against StrictDoc's ReqIF/XHTML schema bundle: pass

```text
Files tested:          30 (20 vendor/source files, 10 synthetic)
Features tested:       basic structure, seven attribute value families,
                       hierarchy, relations, multiple specifications/types,
                       namespaces, unknown elements, attachments, malformed XML,
                       duplicate/missing IDs, ReqIFZ, core round-trip
Problems found:        13
Problems fixed:        4
Partially fixed:       1
Remaining limitations: 9 (including the partial relation-model limitation)
```
