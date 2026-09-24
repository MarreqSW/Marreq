import { describe, expect, it } from 'vitest';
import { formatLogChangeValue, type CatalogLabelMaps } from '../formatLogChangeValue';

const maps: CatalogLabelMaps = {
  requirementStatusById: new Map([[13, 'Draft']]),
  verificationStatusById: new Map([[4, 'Passed']]),
  categoryById: new Map([[2, 'Functional']]),
  applicabilityById: new Map([[8, 'Flight']]),
  methodById: new Map([[5, 'Test']]),
};

describe('formatLogChangeValue', () => {
  it('maps requirement status, category, and applicability ids to titles', () => {
    expect(formatLogChangeValue('Status', '13', 'REQUIREMENT', maps)).toBe('Draft');
    expect(formatLogChangeValue('Category', '2', 'requirement', maps)).toBe('Functional');
    expect(formatLogChangeValue('Applicability', '8', 'REQUIREMENT', maps)).toBe('Flight');
  });

  it('maps verification status and method ids to titles', () => {
    expect(formatLogChangeValue('Status', '4', 'VERIFICATION', maps)).toBe('Passed');
    expect(formatLogChangeValue('Verification type', '5', 'TEST', maps)).toBe('Test');
  });

  it('leaves unknown ids and non-catalog fields unchanged', () => {
    expect(formatLogChangeValue('Status', '99', 'REQUIREMENT', maps)).toBe('99');
    expect(formatLogChangeValue('Title', '13', 'REQUIREMENT', maps)).toBe('13');
    expect(formatLogChangeValue('Status', '—', 'REQUIREMENT', maps)).toBe('—');
  });
});
