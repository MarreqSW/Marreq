import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { getAttention, markNotificationRead } from '@/api/client';
import type { AttentionItem, AttentionResponse, DashboardProject } from '@/api/types';
import AppTopBar from '@/components/start/AppTopBar';
import ProjectInitial from '@/components/start/ProjectInitial';
import StartSearch from '@/components/start/StartSearch';
import { useDashboard } from '@/context/DashboardContext';
import NoProjectsHome from '@/pages/NoProjectsHome';
import { parseUser } from '@/utils/parseUser';
import { readRecentPages } from '@/utils/recentPages';

/** Attention rows shown before "Show all". */
const ATTENTION_PREVIEW = 4;
const RECENT_SHOWN = 3;

function plural(n: number, one: string, many: string) {
  return `${n} ${n === 1 ? one : many}`;
}

/** "2 approvals and 3 suspect links are waiting for you." */
export function attentionSummary(a: AttentionResponse): string {
  const parts = [
    a.approvals ? plural(a.approvals, 'approval', 'approvals') : null,
    a.reviews ? plural(a.reviews, 'draft to review', 'drafts to review') : null,
    a.suspect_links ? plural(a.suspect_links, 'suspect link', 'suspect links') : null,
    a.notifications ? plural(a.notifications, 'notification', 'notifications') : null,
  ].filter((p): p is string => p != null);
  if (parts.length === 0) return 'Nothing needs your attention right now.';
  const list = parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)}`;
  const total = a.approvals + a.reviews + a.suspect_links + a.notifications;
  return `${list} ${total === 1 ? 'is' : 'are'} waiting for you.`;
}

/** Compact age of a server timestamp (naive UTC), e.g. "5 h". */
export function shortAge(at: string | number | null, now = Date.now()): string {
  if (at == null) return '';
  const then = typeof at === 'number' ? at : new Date(at.endsWith('Z') ? at : `${at}Z`).getTime();
  const minutes = Math.max(0, Math.floor((now - then) / 60_000));
  if (minutes < 1) return 'now';
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} d`;
  if (days < 365) return `${Math.floor(days / 30)} mo`;
  return `${Math.floor(days / 365)} y`;
}

function itemPath(item: AttentionItem, project: DashboardProject): string {
  const base = project.project_base_path;
  switch (item.kind) {
    case 'review':
      return `${base}/requirements?approval=draft`;
    case 'suspect':
      return `${base}/traceability?view=matrix&mx_suspect=1`;
    default:
      return item.requirement_id != null ? `${base}/requirements/${item.requirement_id}` : `${base}/dashboard`;
  }
}

function itemIcon(item: AttentionItem): [string, string] {
  switch (item.kind) {
    case 'approval':
      return ['approval', 'text-emerald-700 dark:text-emerald-300'];
    case 'review':
      return ['rate_review', 'text-indigo-700 dark:text-indigo-300'];
    case 'suspect':
      return ['link_off', 'text-amber-700 dark:text-amber-300'];
    default:
      return item.notification_type === 'comment_added'
        ? ['comment', 'text-sky-700 dark:text-sky-300']
        : ['rate_review', 'text-indigo-700 dark:text-indigo-300'];
  }
}

const sectionTitle = 'text-xs font-bold uppercase tracking-widest text-stitch-muted';
const listClass =
  'rounded-xl border border-stitch-border bg-stitch-surface shadow-sm overflow-hidden divide-y divide-stitch-border';

/**
 * The start screen after sign-in (issue #387): a search box, what needs the
 * user's attention across their projects, and the pages they visited last.
 */
export default function StartPage() {
  const { dashboard, loading, csrfToken } = useDashboard();
  const user = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);
  const projects = useMemo(() => dashboard?.projects ?? [], [dashboard?.projects]);
  const [attention, setAttention] = useState<AttentionResponse | null>(null);
  const [attentionError, setAttentionError] = useState(false);
  const [showAllAttention, setShowAllAttention] = useState(false);
  const [projectList, setProjectList] = useState<'none' | 'active' | 'archived'>('none');

  useEffect(() => {
    if (!dashboard || projects.length === 0) return;
    let cancelled = false;
    getAttention()
      .then((a) => {
        if (!cancelled) setAttention(a);
      })
      .catch(() => {
        if (!cancelled) setAttentionError(true);
      });
    return () => {
      cancelled = true;
    };
  }, [dashboard, projects.length]);

  const byId = useMemo(() => new Map(projects.map((p) => [p.id, p])), [projects]);
  const recent = useMemo(
    () => readRecentPages().filter((e) => byId.has(e.projectId)).slice(0, RECENT_SHOWN),
    [byId],
  );
  const active = projects.filter((p) => !p.archived);
  const archived = projects.filter((p) => p.archived);

  if (!dashboard) {
    return loading ? (
      <div className="min-h-screen flex items-center justify-center bg-stitch-canvas text-stitch-muted text-sm">
        Loading…
      </div>
    ) : null;
  }

  const displayName = (user?.name?.trim() || user?.username || '').trim();
  if (projects.length === 0) return <NoProjectsHome displayName={displayName} />;

  const firstName = displayName.split(/\s+/)[0] ?? '';
  const items = (attention?.items ?? []).filter((i) => byId.has(i.project_id));
  const shownItems = showAllAttention ? items : items.slice(0, ATTENTION_PREVIEW);

  return (
    <div className="min-h-screen flex flex-col bg-stitch-canvas text-stitch-fg text-stitch font-sans antialiased">
      <AppTopBar />
      <main className="flex-1 px-4 sm:px-6 py-10">
        <div className="max-w-3xl mx-auto">
          <h1 className="text-center text-2xl font-bold font-headline">
            Where to{firstName ? `, ${firstName}` : ''}?
          </h1>
          <p className="text-center text-sm text-stitch-muted mt-1 mb-5 min-h-5" aria-live="polite">
            {attention ? attentionSummary(attention) : ''}
          </p>

          <StartSearch projects={projects} isAdmin={user?.is_admin === true} />

          <section className="mt-8" aria-labelledby="attention-heading">
            <div className="flex items-baseline justify-between px-1 mb-2">
              <h2 id="attention-heading" className={sectionTitle}>
                Needs your attention
                {items.length > 0 ? (
                  <span className="ml-2 rounded-full bg-red-600 text-white px-1.5 py-0.5 text-[10px] tracking-normal">
                    {items.length}
                  </span>
                ) : null}
              </h2>
              {items.length > ATTENTION_PREVIEW ? (
                <button
                  type="button"
                  onClick={() => setShowAllAttention((v) => !v)}
                  className="text-xs font-semibold text-stitch-accent hover:underline"
                >
                  {showAllAttention ? 'Show fewer' : `Show all (${items.length})`}
                </button>
              ) : null}
            </div>
            {attentionError ? (
              <p className="px-1 text-sm text-stitch-muted">Could not load what needs your attention.</p>
            ) : !attention ? (
              <p className="px-1 text-sm text-stitch-muted">Loading…</p>
            ) : items.length === 0 ? (
              <p className="px-1 text-sm text-stitch-muted">You are all caught up.</p>
            ) : (
              <ul className={listClass}>
                {shownItems.map((item) => {
                  const project = byId.get(item.project_id)!;
                  const [icon, colour] = itemIcon(item);
                  return (
                    <li key={`${item.kind}-${item.project_id}-${item.requirement_id ?? ''}-${item.notification_id ?? ''}`}>
                      <Link
                        to={itemPath(item, project)}
                        onClick={() => {
                          if (item.notification_id != null) {
                            void markNotificationRead(item.notification_id, csrfToken ?? '').catch(() => {});
                          }
                        }}
                        className="flex items-center gap-3 px-4 py-2.5 hover:bg-stitch-elevated/60"
                      >
                        <span className={`material-symbols-outlined text-lg ${colour}`} aria-hidden>
                          {icon}
                        </span>
                        <span className="flex-1 min-w-0 truncate text-sm text-stitch-fg">{item.title}</span>
                        <span className="hidden sm:flex items-center gap-1.5 text-xs text-stitch-muted w-40 min-w-0">
                          <ProjectInitial id={project.id} name={project.name} size="xs" />
                          <span className="truncate">{project.name}</span>
                        </span>
                        <span className="text-xs text-stitch-muted w-10 text-right">{shortAge(item.at)}</span>
                        <span className="material-symbols-outlined text-base text-stitch-muted" aria-hidden>
                          chevron_right
                        </span>
                      </Link>
                    </li>
                  );
                })}
              </ul>
            )}
          </section>

          <section className="mt-6" aria-labelledby="recent-heading">
            <h2 id="recent-heading" className={`${sectionTitle} mb-2 px-1`}>
              {recent.length > 0 ? 'Recent' : 'Your projects'}
            </h2>
            <ul className={listClass}>
              {recent.length > 0
                ? recent.map((entry) => {
                    const project = byId.get(entry.projectId)!;
                    return (
                      <li key={entry.projectId}>
                        <Link to={entry.path} className="flex items-center gap-3 px-4 py-2.5 hover:bg-stitch-elevated/60">
                          <ProjectInitial id={project.id} name={project.name} />
                          <span className="text-sm font-semibold text-stitch-fg w-44 truncate">{project.name}</span>
                          <span className="flex-1 min-w-0 truncate text-sm text-stitch-muted">
                            {entry.detail ? `${entry.section} › ${entry.detail}` : entry.section}
                          </span>
                          <span className="text-xs text-stitch-muted">{shortAge(entry.at)}</span>
                        </Link>
                      </li>
                    );
                  })
                : active.slice(0, RECENT_SHOWN).map((p) => <ProjectRow key={p.id} project={p} />)}
            </ul>
          </section>

          <nav
            aria-label="More"
            className="mt-5 flex flex-wrap items-center justify-between gap-3 px-1 text-sm"
          >
            <div className="flex flex-wrap gap-5 font-semibold text-stitch-fg">
              {(
                [
                  ['/projects/new', 'add_box', 'New project'],
                  ['/projects/import-bundle', 'upload_file', 'Import'],
                  ['/groups', 'workspaces', 'Groups'],
                ] as const
              ).map(([to, icon, label]) => (
                <Link key={to} to={to} className="flex items-center gap-1.5 hover:text-stitch-accent">
                  <span className="material-symbols-outlined text-base text-stitch-muted" aria-hidden>
                    {icon}
                  </span>
                  {label}
                </Link>
              ))}
            </div>
            <div className="flex gap-4 font-semibold">
              <button
                type="button"
                aria-expanded={projectList === 'active'}
                onClick={() => setProjectList((v) => (v === 'active' ? 'none' : 'active'))}
                className="text-stitch-accent hover:underline"
              >
                All projects ({active.length})
              </button>
              {archived.length > 0 ? (
                <button
                  type="button"
                  aria-expanded={projectList === 'archived'}
                  onClick={() => setProjectList((v) => (v === 'archived' ? 'none' : 'archived'))}
                  className="text-stitch-muted hover:underline"
                >
                  Archived ({archived.length})
                </button>
              ) : null}
            </div>
          </nav>

          {projectList !== 'none' ? (
            <section className="mt-4" aria-label={projectList === 'active' ? 'All projects' : 'Archived projects'}>
              <ul className={listClass}>
                {(projectList === 'active' ? active : archived).map((p) => (
                  <ProjectRow key={p.id} project={p} />
                ))}
              </ul>
            </section>
          ) : null}
        </div>
      </main>
    </div>
  );
}

function ProjectRow({ project }: { project: DashboardProject }) {
  const role = typeof project.role_label === 'string' ? project.role_label : '';
  return (
    <li>
      <Link
        to={`${project.project_base_path}/dashboard`}
        className="flex items-center gap-3 px-4 py-2.5 hover:bg-stitch-elevated/60"
      >
        <ProjectInitial id={project.id} name={project.name} />
        <span className="text-sm font-semibold text-stitch-fg flex-1 min-w-0 truncate">{project.name}</span>
        <span className="text-xs text-stitch-muted">{project.group_name ?? 'Personal'}</span>
        {role ? <span className="text-xs text-stitch-muted w-20 text-right">{role}</span> : null}
      </Link>
    </li>
  );
}
