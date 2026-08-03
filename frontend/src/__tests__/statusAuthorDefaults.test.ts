import { describe, expect, it } from 'vitest';
import {
  authorDefaultRequirementStatusId,
  initialVerificationStatusIdForAuthor,
} from '../statusAuthorDefaults';

describe('authorDefaultRequirementStatusId', () => {
  it('returns null for an empty list', () => {
    expect(authorDefaultRequirementStatusId([])).toBeNull();
  });

  it('prefers a status tagged draft (case-insensitive)', () => {
    expect(
      authorDefaultRequirementStatusId([
        { id: 3, tag: 'Accepted' },
        { id: 1, tag: 'Draft' },
        { id: 2, tag: 'Rejected' },
      ]),
    ).toBe(1);
    expect(
      authorDefaultRequirementStatusId([
        { id: 9, tag: 'DRAFT' },
        { id: 1, tag: 'other' },
      ]),
    ).toBe(9);
  });

  it('falls back to the minimum id when no draft tag exists', () => {
    expect(
      authorDefaultRequirementStatusId([
        { id: 5, tag: 'Accepted' },
        { id: 2, tag: 'Rejected' },
        { id: 8, tag: 'Review' },
      ]),
    ).toBe(2);
  });
});

describe('initialVerificationStatusIdForAuthor', () => {
  it('returns null for an empty list', () => {
    expect(initialVerificationStatusIdForAuthor([])).toBeNull();
  });

  it('prefers a status tagged nr (case-insensitive)', () => {
    expect(
      initialVerificationStatusIdForAuthor([
        { id: 4, tag: 'Pass' },
        { id: 7, tag: 'NR' },
        { id: 1, tag: 'Fail' },
      ]),
    ).toBe(7);
  });

  it('falls back to the minimum id when no nr tag exists', () => {
    expect(
      initialVerificationStatusIdForAuthor([
        { id: 10, tag: 'Pass' },
        { id: 3, tag: 'Fail' },
      ]),
    ).toBe(3);
  });
});
