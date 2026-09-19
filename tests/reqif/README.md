# ReqIF interoperability fixtures

These files exercise Marreq's real `ReqIFService::import_into_project` path. Vendor
fixtures are stored byte-for-byte as downloaded; tests never rewrite them. Synthetic
files isolate robustness behavior and are maintained by Marreq.

The automated tests are in
`marreq-core/src/reqif/vendor_import_tests.rs`.

## Sources and coverage

| Fixture | Claimed/source tool | Source URL | Main coverage |
| --- | --- | --- | --- |
| `doors/capella_Sample.reqif` | IBM Rational DOORS | [Eclipse Capella Requirements VP](https://github.com/eclipse-capella/capella-requirements-vp/blob/master/tests/org.polarsys.capella.vp.requirements.ju/model/inputs/Sample.reqif) | XHTML, enumeration, specification, hierarchy |
| `doors/strictdoc_doors_date.reqif` | DOORS sample | [StrictDoc ReqIF](https://github.com/strictdoc-project/reqif/blob/main/tests/integration/reqif_software/Doors/01_anonimized_example_date_data_type/sample.reqif) | date datatype/value |
| `doors/strictdoc_doors_user.reqif` | IBM | [StrictDoc ReqIF](https://github.com/strictdoc-project/reqif/blob/main/tests/integration/reqif_software/Doors/03_example_from_a_user/sample.reqif) | XHTML and incomplete/non-standard document structure |
| `capella/eclipse_capella_Sample.xml` | IBM Rational DOORS, consumed by Capella | [LutaML mirror](https://github.com/lutaml/reqif/blob/main/spec/fixtures/eclipse_capella_Sample.xml) | XHTML, enumeration, specification |
| `capella/eclipse_capella_Sample1.xml` | IBM Rational DOORS, consumed by Capella | [LutaML mirror](https://github.com/lutaml/reqif/blob/main/spec/fixtures/eclipse_capella_Sample1.xml) | two objects and one relation |
| `capella/eclipse_capella_Sample3.xml` | IBM Rational DOORS, consumed by Capella | [LutaML mirror](https://github.com/lutaml/reqif/blob/main/spec/fixtures/eclipse_capella_Sample3.xml) | string defaults, XHTML, multi enumeration |
| `eclipse-rmf/eclipse_rmf_sample.reqif` | Manually written RMF sample (ReqIF 2010 namespace) | [LutaML](https://github.com/lutaml/reqif/blob/main/spec/fixtures/eclipse_rmf_sample.reqif) | legacy namespace, integer and string |
| `eclipse-rmf/eclipse_rmf_specRelationTest.reqif` | Manually written RMF sample | [LutaML](https://github.com/lutaml/reqif/blob/main/spec/fixtures/eclipse_rmf_specRelationTest.reqif) | `SPEC-RELATION` |
| `eclipse-rmf/strictdoc_04_sample3_eclipse_rmf.reqif` | Eclipse RMF | [LutaML mirror of StrictDoc sample](https://github.com/lutaml/reqif/blob/main/spec/fixtures/strictdoc_04_convert_reqif_to_json_sample3_eclipse_rmf.reqif) | multiple specifications, XHTML, enumeration |
| `enterprise-architect/ea_example.reqif.xml` | EA example; header says microTool in-Step | [LutaML](https://github.com/lutaml/reqif/blob/main/spec/fixtures/ea_example.reqif.xml) | multiple object types, enumeration, relation, dangling references |
| `enterprise-architect/strictdoc_ea8.reqif` | Sparx Enterprise Architect 8 example; header says microTool in-Step | [StrictDoc ReqIF](https://github.com/strictdoc-project/reqif/blob/main/tests/integration/reqif_software/SparxSystems_Enterprise_Architect_8.0/01_example/sample.reqif) | same EA interoperability case, direct source |
| `polarion/polarion_export.xml` | Polarion ALM | [LutaML](https://github.com/lutaml/reqif/blob/main/spec/fixtures/polarion_export.xml) | XHTML and enumeration |
| `polarion/strictdoc_04_sample1_polarion.reqif` | Polarion | [LutaML mirror of StrictDoc sample](https://github.com/lutaml/reqif/blob/main/spec/fixtures/strictdoc_04_convert_reqif_to_json_sample1_polarion.reqif) | section hierarchy, custom attributes, XHTML |
| `polarion/strictdoc_polarion_anonymized.reqif` | anonymized Polarion | [StrictDoc ReqIF](https://github.com/strictdoc-project/reqif/blob/main/tests/integration/reqif_software/Polarion/01_anonimized_example/sample.reqif) | 101 objects, deep hierarchy, XHTML, dates |
| `polarion/antcc_MAG8000-LTE-FeatureSpecReqBL4.reqif` | Polarion | [LutaML](https://github.com/lutaml/reqif/blob/main/spec/fixtures/antcc_MAG8000-LTE-FeatureSpecReqBL4.reqif) | 102 objects, 100 hierarchy edges, 26 external relations, XHTML |
| `polarion/polarion.reqifz` | Polarion ReqIFZ | [LutaML mirror](https://github.com/lutaml/reqif/blob/main/spec/fixtures/strictdoc_04_convert_reqif_to_json_sample_polarion_reqifz.reqifz) | compressed ReqIF package |
| `strictdoc/strictdoc_01_minimal_reqif_sample.reqif` | StrictDoc test fixture | [LutaML mirror](https://github.com/lutaml/reqif/blob/main/spec/fixtures/strictdoc_01_minimal_reqif_sample.reqif) | minimal/incomplete document |
| `strictdoc/strictdoc_02_read_reqif_input.reqif` | IBM Rational DOORS | [LutaML mirror](https://github.com/lutaml/reqif/blob/main/spec/fixtures/strictdoc_02_read_reqif_input.reqif) | 3 objects, many vendor extensions and attribute definitions |
| `strictdoc/strictdoc_04_sample2_sdoc.reqif` | StrictDoc | [LutaML mirror](https://github.com/lutaml/reqif/blob/main/spec/fixtures/strictdoc_04_convert_reqif_to_json_sample2_sdoc.reqif) | 18 objects, nested hierarchy, custom attributes |
| `reqif-studio/strictdoc_reqif_studio.reqif` | ReqIF Studio | [StrictDoc ReqIF](https://github.com/strictdoc-project/reqif/blob/main/tests/integration/reqif_software/ReqIF_Studio/01_anonimized_example/sample.reqif) | 137 objects, 128 hierarchy edges, 14 relations, dangling hierarchy reference |

The LutaML fixtures intentionally include a provenance comment before the XML
declaration. Strict XML processors reject that lexical form even when the ReqIF
content after the comment validates. Tests preserve and tolerate this source
form; the compatibility report records both lexical and schema results.

No downloaded fixture was found that could be positively identified as IBM
DOORS Next/DNG. The report therefore does not claim DOORS Next validation.

## Synthetic robustness cases

| Fixture | Purpose |
| --- | --- |
| `synthetic/marreq-style.reqif` | Marreq core-field and parent round-trip |
| `synthetic/prefixed-namespace.reqif` | namespace-prefixed ReqIF elements |
| `synthetic/unknown-elements.reqif` | unknown vendor elements |
| `synthetic/embedded-file.reqif` | unsupported embedded file warning |
| `synthetic/formatted-xhtml.reqif` | XHTML links, table cells and image objects |
| `synthetic/typed-values.reqif` | integer, real, boolean, date and enumeration values |
| `synthetic/dangling-relation.reqif` | external/missing relation target |
| `synthetic/duplicate-identifiers.reqif` | duplicate `SPEC-OBJECT` IDs rejected before writes |
| `synthetic/invalid-root.xml` | non-ReqIF XML rejected |
| `synthetic/malformed.xml` | malformed XML rejected without panic |

## Validation

Schema checks use the ReqIF schema bundle vendored by
[`strictdoc-project/reqif`](https://github.com/strictdoc-project/reqif/tree/main/reqif/reqif_schema).
That bundle includes the XHTML dependencies, unlike the standalone XSD under
`docs/ReqIF/`, whose remote `xml.xsd` import cannot be resolved offline.

Passing the Rust test means that the expected parser/import behavior was
observed. It does **not** imply lossless interoperability; expected partial and
rejected cases are asserted explicitly. See
`docs/developer/reqif-import-compatibility-report.md`.
