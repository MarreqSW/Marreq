/**
 * Pages recently visited in each project, for the start screen's Recent list
 * (issue #387). Kept per browser in localStorage: one entry per project (its
 * latest page), newest first.
 */

const KEY = 'marreq-recent-pages';
const MAX_ENTRIES = 10;

export type RecentPage = {
  projectId: number;
  /** Path and query inside the app, e.g. `/space-project/requirements/12`. */
  path: string;
  /** The project section, e.g. "Requirements". */
  section: string;
  /** What was open in it, e.g. a reference code. */
  detail?: string;
  /** Epoch milliseconds. */
  at: number;
};

const SECTIONS: Record<string, string> = {
  dashboard: 'Dashboard',
  requirements: 'Requirements',
  verifications: 'Verifications',
  traceability: 'Traceability',
  matrix: 'Traceability',
  baselines: 'Baselines',
  reports: 'Reports',
  settings: 'Project settings',
  help: 'Help',
};

/** The section for a path below `/<project-slug>`, e.g. `/requirements/12` → Requirements. */
export function sectionForPath(subPath: string): string {
  const first = subPath.split('/').filter(Boolean)[0] ?? 'dashboard';
  return SECTIONS[first] ?? 'Dashboard';
}

export function readRecentPages(): RecentPage[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(parsed)
      ? parsed.filter(
          (e): e is RecentPage =>
            typeof e?.projectId === 'number' &&
            typeof e?.path === 'string' &&
            typeof e?.section === 'string' &&
            typeof e?.at === 'number',
        )
      : [];
  } catch {
    return [];
  }
}

function write(entries: RecentPage[]) {
  try {
    localStorage.setItem(KEY, JSON.stringify(entries.slice(0, MAX_ENTRIES)));
  } catch {
    /* storage unavailable: Recent stays empty */
  }
}

/** Record a visit; it replaces the project's previous entry. */
export function recordVisit(projectId: number, path: string, subPath: string, now = Date.now()) {
  const previous = readRecentPages().find((e) => e.projectId === projectId);
  const entry: RecentPage = { projectId, path, section: sectionForPath(subPath), at: now };
  // Keep the detail while the same page is shown again (e.g. after a refresh).
  if (previous?.path === path && previous.detail) entry.detail = previous.detail;
  write([entry, ...readRecentPages().filter((e) => e.projectId !== projectId)]);
}

/** Name what is open on a recorded page, once the page has loaded it. */
export function setVisitDetail(projectId: number, path: string, detail: string) {
  const entries = readRecentPages();
  const entry = entries.find((e) => e.projectId === projectId && e.path === path);
  if (!entry || entry.detail === detail) return;
  entry.detail = detail;
  write(entries);
}
