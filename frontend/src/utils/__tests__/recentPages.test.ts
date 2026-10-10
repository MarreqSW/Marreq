import { beforeEach, describe, expect, it } from 'vitest';
import { readRecentPages, recordVisit, sectionForPath, setVisitDetail } from '../recentPages';

beforeEach(() => localStorage.clear());

describe('recentPages', () => {
  it('names the section of a project path', () => {
    expect(sectionForPath('/requirements/12')).toBe('Requirements');
    expect(sectionForPath('/traceability')).toBe('Traceability');
    expect(sectionForPath('/settings/members')).toBe('Project settings');
    expect(sectionForPath('')).toBe('Dashboard');
    expect(sectionForPath('/unknown')).toBe('Dashboard');
  });

  it('keeps one entry per project, newest first, with its detail', () => {
    recordVisit(1, '/a/dashboard', '/dashboard', 1000);
    recordVisit(2, '/b/requirements', '/requirements', 2000);
    recordVisit(1, '/a/requirements/7', '/requirements/7', 3000);
    setVisitDetail(1, '/a/requirements/7', 'REQ-7');
    setVisitDetail(1, '/a/other', 'ignored');

    expect(readRecentPages()).toEqual([
      { projectId: 1, path: '/a/requirements/7', section: 'Requirements', detail: 'REQ-7', at: 3000 },
      { projectId: 2, path: '/b/requirements', section: 'Requirements', at: 2000 },
    ]);

    // Revisiting the same page keeps its detail; another page drops it.
    recordVisit(1, '/a/requirements/7', '/requirements/7', 4000);
    expect(readRecentPages()[0]).toMatchObject({ detail: 'REQ-7', at: 4000 });
    recordVisit(1, '/a/baselines', '/baselines', 5000);
    expect(readRecentPages()[0]).not.toHaveProperty('detail');
  });

  it('ignores corrupt storage', () => {
    localStorage.setItem('marreq-recent-pages', '{not json');
    expect(readRecentPages()).toEqual([]);
    localStorage.setItem('marreq-recent-pages', JSON.stringify([{ projectId: 'x' }, 3]));
    expect(readRecentPages()).toEqual([]);
  });
});
