import { useEffect, useId, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { searchRequirements } from '@/api/client';
import type { DashboardProject, SearchHit } from '@/api/types';
import ProjectInitial from './ProjectInitial';

type Result = {
  key: string;
  group: 'Projects' | 'Requirements' | 'Actions';
  to: string;
  label: string;
  hint: string;
  icon?: string;
  project?: DashboardProject;
};

const ACTIONS: { label: string; to: string; icon: string; adminOnly?: boolean }[] = [
  { label: 'New project', to: '/projects/new', icon: 'add_box' },
  { label: 'Import project bundle', to: '/projects/import-bundle', icon: 'upload_file' },
  { label: 'Groups', to: '/groups', icon: 'workspaces' },
  { label: 'New group', to: '/groups/new', icon: 'group_add' },
  { label: 'Account settings', to: '/account', icon: 'manage_accounts' },
  { label: 'Change password', to: '/change-password', icon: 'password' },
  { label: 'Administration', to: '/admin', icon: 'admin_panel_settings', adminOnly: true },
];

const MAX_PROJECTS = 5;
const DEBOUNCE_MS = 200;

/**
 * The start screen's search (issue #387): projects and actions are matched
 * in the browser, requirements by reference code or title on the server.
 * Ctrl+K / ⌘K focuses it; arrows move through the results and Enter opens one.
 */
export default function StartSearch({
  projects,
  isAdmin,
}: {
  projects: DashboardProject[];
  isAdmin: boolean;
}) {
  const navigate = useNavigate();
  const listId = useId();
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [q, setQ] = useState('');
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [searching, setSearching] = useState(false);
  const [active, setActive] = useState(0);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.select();
      }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);

  const needle = q.trim().toLowerCase();

  useEffect(() => {
    if (needle.length < 2) {
      setHits([]);
      setSearching(false);
      return;
    }
    let cancelled = false;
    setSearching(true);
    const timer = window.setTimeout(() => {
      searchRequirements(needle)
        .then((found) => {
          if (!cancelled) setHits(found);
        })
        .catch(() => {
          if (!cancelled) setHits([]);
        })
        .finally(() => {
          if (!cancelled) setSearching(false);
        });
    }, DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [needle]);

  const results = useMemo<Result[]>(() => {
    if (!needle) return [];
    const byId = new Map(projects.map((p) => [p.id, p]));
    const projectResults: Result[] = projects
      .filter((p) => p.name.toLowerCase().includes(needle) || p.slug.toLowerCase().includes(needle))
      .slice(0, MAX_PROJECTS)
      .map((p) => ({
        key: `p-${p.id}`,
        group: 'Projects',
        to: `${p.project_base_path}/dashboard`,
        label: p.name,
        hint: p.archived ? 'Archived' : (p.group_name ?? 'Personal'),
        project: p,
      }));
    const requirementResults: Result[] = hits.flatMap((h) => {
      const project = byId.get(h.project_id);
      if (!project) return [];
      return [
        {
          key: `r-${h.requirement_id}`,
          group: 'Requirements' as const,
          to: `${project.project_base_path}/requirements/${h.requirement_id}`,
          // Imported requirements often use the code as their title too.
          label: h.title && h.title !== h.reference_code ? `${h.reference_code} · ${h.title}` : h.reference_code,
          hint: project.name,
          project,
        },
      ];
    });
    const actionResults: Result[] = ACTIONS.filter(
      (a) => (!a.adminOnly || isAdmin) && a.label.toLowerCase().includes(needle),
    ).map((a) => ({ key: `a-${a.to}`, group: 'Actions', to: a.to, label: a.label, hint: '', icon: a.icon }));
    return [...projectResults, ...requirementResults, ...actionResults];
  }, [needle, projects, hits, isAdmin]);

  useEffect(() => {
    setActive(0);
  }, [needle, results.length]);

  const open = (result: Result | undefined) => {
    if (result) navigate(result.to);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActive((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      open(results[active]);
    } else if (e.key === 'Escape') {
      setQ('');
    }
  };

  const expanded = needle.length > 0;
  const optionId = (i: number) => `${listId}-option-${i}`;

  return (
    <div className="relative">
      <div className="rounded-xl border-2 border-stitch-accent/60 bg-stitch-surface shadow-lg flex items-center gap-3 px-4 sm:px-5 py-3 focus-within:border-stitch-accent">
        <span className="material-symbols-outlined text-2xl text-stitch-accent" aria-hidden>
          search
        </span>
        <input
          ref={inputRef}
          type="text"
          role="combobox"
          aria-label="Search projects, requirements and actions"
          aria-expanded={expanded}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={expanded && results.length > 0 ? optionId(active) : undefined}
          autoFocus
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Jump to a project, a requirement (REQ-…), or an action…"
          className="flex-1 min-w-0 bg-transparent text-base text-stitch-fg placeholder:text-stitch-muted outline-hidden"
        />
        <kbd className="hidden sm:inline rounded-sm border border-stitch-border bg-stitch-canvas px-2 py-0.5 text-xs font-mono text-stitch-muted">
          Ctrl K
        </kbd>
      </div>
      {expanded ? (
        <ul
          id={listId}
          role="listbox"
          aria-label="Search results"
          className="absolute left-0 right-0 top-[calc(100%+6px)] z-50 max-h-[60vh] overflow-y-auto rounded-xl border border-stitch-border bg-stitch-surface shadow-stitch py-1"
        >
          {results.length === 0 ? (
            <li className="px-4 py-3 text-sm text-stitch-muted" role="presentation">
              {searching ? 'Searching…' : needle.length < 2 ? 'Keep typing to search requirements…' : 'No matches.'}
            </li>
          ) : (
            results.map((r, i) => (
              <li key={r.key} role="presentation">
                {i === 0 || results[i - 1]!.group !== r.group ? (
                  <p className="px-4 pt-2 pb-1 text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
                    {r.group}
                  </p>
                ) : null}
                <div
                  id={optionId(i)}
                  role="option"
                  aria-selected={i === active}
                  onMouseEnter={() => setActive(i)}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    open(r);
                  }}
                  className={`flex items-center gap-3 px-4 py-2 cursor-pointer ${
                    i === active ? 'bg-stitch-elevated' : ''
                  }`}
                >
                  {r.project ? (
                    <ProjectInitial id={r.project.id} name={r.project.name} />
                  ) : (
                    <span className="material-symbols-outlined text-lg text-stitch-muted w-6 text-center" aria-hidden>
                      {r.icon}
                    </span>
                  )}
                  <span className="flex-1 min-w-0 truncate text-sm text-stitch-fg">{r.label}</span>
                  {r.hint ? <span className="text-xs text-stitch-muted truncate max-w-[40%]">{r.hint}</span> : null}
                </div>
              </li>
            ))
          )}
          {searching && results.length > 0 ? (
            <li role="presentation" className="px-4 py-2 text-xs text-stitch-muted">
              Searching requirements…
            </li>
          ) : null}
        </ul>
      ) : null}
    </div>
  );
}
