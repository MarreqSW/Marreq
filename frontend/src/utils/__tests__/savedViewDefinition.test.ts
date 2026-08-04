import { describe, expect, it } from 'vitest';
import {
  buildSavedViewDefinition,
  defaultQueryState,
  parseSavedViewDefinition,
} from '../savedViewDefinition';

describe('savedViewDefinition', () => {
  it('round-trips query state', () => {
    const state = {
      ...defaultQueryState(),
      statusFilter: 7 as const,
      categoryFilter: 2 as const,
      approvalFilter: 'approved',
      q: 'sensor',
      sortColumn: 'title' as const,
      sortDir: 'desc' as const,
      viewMode: 'list' as const,
      pageSize: 50,
    };
    const def = buildSavedViewDefinition(state);
    expect(def.version).toBe(1);
    expect(def.filters.status_id).toBe(7);
    const parsed = parseSavedViewDefinition(def);
    expect(parsed).toEqual(state);
  });

  it('fills defaults for empty definition', () => {
    const parsed = parseSavedViewDefinition({});
    expect(parsed.statusFilter).toBe('all');
    expect(parsed.columns.length).toBeGreaterThan(0);
  });
});
