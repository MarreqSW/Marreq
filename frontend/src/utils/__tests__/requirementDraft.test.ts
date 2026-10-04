import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  DRAFT_MAX_AGE_MS,
  allowUserDrafts,
  clearDraft,
  clearUserDrafts,
  formatDraftTime,
  readDraft,
  requirementDraftKey,
  writeDraft,
} from '../requirementDraft';

const values = { title: 'Long statement' };

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

describe('requirementDraft (issue #255)', () => {
  it('keys drafts by user, project and requirement', () => {
    expect(requirementDraftKey(7, 3, 12)).toBe('marreq-draft:v1:u7:p3:12');
    expect(requirementDraftKey(7, 3, 'new')).toBe('marreq-draft:v1:u7:p3:new');
  });

  it('writes, reads and clears a draft', () => {
    const key = requirementDraftKey(7, 3, 12);
    const draft = { savedAt: new Date().toISOString(), baseVersionId: 40, values };
    expect(writeDraft(key, draft)).toBe(true);
    expect(readDraft(key)).toEqual(draft);
    clearDraft(key);
    expect(readDraft(key)).toBeNull();
  });

  it('drops drafts older than 30 days', () => {
    const key = requirementDraftKey(7, 3, 12);
    const now = Date.parse('2026-10-04T12:00:00Z');
    const old = new Date(now - DRAFT_MAX_AGE_MS - 1000).toISOString();
    writeDraft(key, { savedAt: old, baseVersionId: 1, values });
    expect(readDraft(key, now)).toBeNull();
    expect(localStorage.getItem(key)).toBeNull();
  });

  it('treats corrupt or foreign data as no draft', () => {
    const key = requirementDraftKey(7, 3, 12);
    localStorage.setItem(key, '{not json');
    expect(readDraft(key)).toBeNull();
    localStorage.setItem(key, JSON.stringify({ savedAt: 'yesterday', values }));
    expect(readDraft(key)).toBeNull();
    localStorage.setItem(key, JSON.stringify({ savedAt: new Date().toISOString() }));
    expect(readDraft(key)).toBeNull();
  });

  it('reports a refused write instead of throwing', () => {
    const setItem = vi.spyOn(localStorage, 'setItem').mockImplementation(() => {
      throw new DOMException('quota', 'QuotaExceededError');
    });
    try {
      expect(writeDraft('k', { savedAt: new Date().toISOString(), baseVersionId: null, values })).toBe(
        false,
      );
    } finally {
      setItem.mockRestore();
    }
  });

  it('clears only the signed-out user’s drafts', () => {
    const draft = { savedAt: new Date().toISOString(), baseVersionId: null, values };
    writeDraft(requirementDraftKey(7, 1, 'new'), draft);
    writeDraft(requirementDraftKey(7, 2, 5), draft);
    writeDraft(requirementDraftKey(70, 1, 5), draft);
    localStorage.setItem('marreq-theme', 'dark');
    clearUserDrafts(7);
    expect(localStorage.getItem(requirementDraftKey(7, 1, 'new'))).toBeNull();
    expect(localStorage.getItem(requirementDraftKey(7, 2, 5))).toBeNull();
    expect(localStorage.getItem(requirementDraftKey(70, 1, 5))).not.toBeNull();
    expect(localStorage.getItem('marreq-theme')).toBe('dark');

    // An editor unmounting during sign-out must not write its draft back...
    writeDraft(requirementDraftKey(7, 2, 5), draft);
    expect(localStorage.getItem(requirementDraftKey(7, 2, 5))).toBeNull();
    // ...until the user is signed in again.
    allowUserDrafts(7);
    writeDraft(requirementDraftKey(7, 2, 5), draft);
    expect(localStorage.getItem(requirementDraftKey(7, 2, 5))).not.toBeNull();
  });

  it('formats today as a time and older drafts with the date', () => {
    const now = new Date(2026, 9, 4, 15, 0);
    expect(formatDraftTime(new Date(2026, 9, 4, 12, 3), now)).not.toMatch(/Oct/);
    expect(formatDraftTime(new Date(2026, 9, 1, 12, 3), now)).toMatch(/Oct/);
  });
});
