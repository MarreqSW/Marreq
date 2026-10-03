import type { ReportDefinition, ReportTemplate, ReportTypeInfo } from '@/api/reports';

/** Report types as `GET …/reports/types` returns them (trimmed). */
export function reportTypes(): ReportTypeInfo[] {
  const doc = {
    doc_id: 'SAT-VCD-001',
    title: '',
    issue: '1',
    revision: '0',
    classification: '',
    watermark: null,
    page_size: 'a4' as const,
    pdf_a: false,
    signatories: [{ role: 'Prepared by', name: 'Alice' }],
    change_record: [],
    documents: [],
  };
  return [
    {
      key: 'vcd',
      title: 'Verification Control Document',
      description: 'Per requirement: method, evidence, compliance and close-out.',
      formats: ['pdf', 'odt'],
      sections: [
        { key: 'cover', title: 'Cover page', description: 'Title page.', report_types: ['vcd'], default_enabled: true, options: [] },
        { key: 'toc', title: 'Contents', description: 'Table of contents.', report_types: ['vcd'], default_enabled: true, options: [] },
        {
          key: 'matrix',
          title: 'Verification control matrix',
          description: 'One row per requirement.',
          report_types: ['vcd'],
          default_enabled: true,
          options: [
            {
              key: 'group_by',
              label: 'Group by',
              kind: 'select',
              choices: [
                { value: 'code_prefix', label: 'Reference code prefix' },
                { value: 'category', label: 'Category' },
              ],
              default: 'code_prefix',
            },
            {
              key: 'columns',
              label: 'Columns',
              kind: 'multi_select',
              choices: [
                { value: 'id', label: 'Req. ID' },
                { value: 'closeout', label: 'Close-out' },
              ],
              default: ['id', 'closeout'],
            },
          ],
        },
      ],
      default_definition: {
        report_type: 'vcd',
        document: doc,
        sections: [
          { key: 'cover', enabled: true },
          { key: 'toc', enabled: true },
          { key: 'matrix', enabled: true },
        ],
      },
    },
    {
      key: 'coverage',
      title: 'Traceability & Coverage Report',
      description: 'Coverage figures and gaps.',
      formats: ['pdf', 'odt'],
      sections: [
        { key: 'cover', title: 'Cover page', description: '', report_types: ['coverage'], default_enabled: true, options: [] },
      ],
      default_definition: {
        report_type: 'coverage',
        document: { ...doc, doc_id: 'SAT-TCR-001' },
        sections: [{ key: 'cover', enabled: true }],
      },
    },
  ];
}

export function reportTemplate(patch: Partial<ReportTemplate> = {}): ReportTemplate {
  const definition: ReportDefinition = JSON.parse(JSON.stringify(reportTypes()[0].default_definition));
  definition.sections = [definition.sections[2], definition.sections[0], { ...definition.sections[1], enabled: false }];
  return {
    id: 7,
    project_id: 5,
    name: 'CDR VCD',
    report_type: 'vcd',
    visibility: 'private',
    owner_id: 1,
    owner_name: 'Alice',
    definition,
    can_edit: true,
    created_at: '2026-10-03T00:00:00',
    updated_at: '2026-10-03T00:00:00',
    ...patch,
  };
}
