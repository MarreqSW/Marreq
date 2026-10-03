import { describe, expect, it } from 'vitest';
import type { ReportDefinition } from '@/api/reports';
import {
  cloneDefinition,
  moveSection,
  sameDefinition,
  setSectionOption,
  toggleSection,
} from '../reportDefinition';

const def: ReportDefinition = {
  report_type: 'vcd',
  document: {
    doc_id: 'X',
    title: '',
    issue: '1',
    revision: '0',
    classification: '',
    watermark: null,
    page_size: 'a4',
    pdf_a: false,
    signatories: [],
    change_record: [],
    documents: [],
  },
  sections: [
    { key: 'cover', enabled: true },
    { key: 'toc', enabled: true },
    { key: 'matrix', enabled: true, options: { group_by: 'category' } },
  ],
};
const keys = (d: ReportDefinition) => d.sections.map((s) => s.key);

describe('report definition helpers', () => {
  it('moves sections and clamps out-of-range targets', () => {
    expect(keys(moveSection(def, 2, 0))).toEqual(['matrix', 'cover', 'toc']);
    expect(keys(moveSection(def, 0, 1))).toEqual(['toc', 'cover', 'matrix']);
    expect(keys(moveSection(def, 0, 99))).toEqual(['toc', 'matrix', 'cover']);
    expect(moveSection(def, 1, 1)).toBe(def);
    expect(moveSection(def, 7, 0)).toBe(def);
    expect(keys(def)).toEqual(['cover', 'toc', 'matrix']);
  });

  it('toggles a section and sets or clears options without mutating the input', () => {
    const off = toggleSection(def, 'toc', false);
    expect(off.sections[1].enabled).toBe(false);
    expect(def.sections[1].enabled).toBe(true);
    const grouped = setSectionOption(def, 'matrix', 'columns', ['id']);
    expect(grouped.sections[2].options).toEqual({ group_by: 'category', columns: ['id'] });
    const cleared = setSectionOption(grouped, 'matrix', 'group_by', undefined);
    expect(cleared.sections[2].options).toEqual({ columns: ['id'] });
    expect(def.sections[2].options).toEqual({ group_by: 'category' });
  });

  it('clones deeply and compares by value', () => {
    const copy = cloneDefinition(def);
    expect(sameDefinition(copy, def)).toBe(true);
    copy.document.signatories.push({ role: 'Prepared by', name: 'Alice' });
    expect(def.document.signatories).toHaveLength(0);
    expect(sameDefinition(copy, def)).toBe(false);
    expect(sameDefinition(null, null)).toBe(true);
  });
});
