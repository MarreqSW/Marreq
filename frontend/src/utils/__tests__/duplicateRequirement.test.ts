import { describe, expect, it } from 'vitest';
import {
  duplicateRequirementTitle,
  nextDuplicateReference,
} from '../duplicateRequirement';

describe('requirement duplication defaults', () => {
  it('marks the copied title', () => {
    expect(duplicateRequirementTitle('Power mode')).toBe('Power mode (Copy)');
  });

  it('increments a numeric reference while preserving its width', () => {
    expect(
      nextDuplicateReference('REQ-PWR-001', [
        { reference_code: 'REQ-PWR-001' },
        { reference_code: 'REQ-PWR-002' },
      ]),
    ).toBe('REQ-PWR-003');
  });

  it('uses a collision-free copy suffix for non-numeric references', () => {
    expect(
      nextDuplicateReference('POWER', [
        { reference_code: 'POWER' },
        { reference_code: 'power-copy' },
      ]),
    ).toBe('POWER-COPY-2');
  });
});
