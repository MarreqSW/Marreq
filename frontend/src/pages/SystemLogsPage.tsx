import { Fragment, FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useOutletContext } from 'react-router-dom';
import {
  cleanupAdminLogs,
  downloadAdminLogsJson,
  listAdminLogs,
  listApplicability,
  listCategories,
  listRequirementStatuses,
  listUsersOptional,
  listVerificationMethods,
  listVerificationStatuses,
} from '@/api/client';
import type { AdminLogItem, AdminLogListParams } from '@/api/types';
import { Pagination } from '@/components/table/Pagination';
import StitchPageHeader from '@/components/StitchPageHeader';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import {
  formatLogChangeValue,
  type CatalogLabelMaps,
} from '@/utils/formatLogChangeValue';
import { parseUser } from '@/utils/parseUser';

const PAGE_SIZE = 50;
const inputClass =
  'w-full bg-stitch-elevated border border-stitch-border rounded-md px-2 py-1.5 text-xs text-stitch-fg';

function formatTs(iso: string): string {
  const d = new Date(iso.includes('T') ? iso : iso.replace(' ', 'T'));
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}

function optionalNumber(raw: string): number | undefined {
  const t = raw.trim();
  if (!t) return undefined;
  const n = Number(t);
  return Number.isFinite(n) ? n : undefined;
}

function optionalText(raw: string): string | undefined {
  const t = raw.trim();
  return t || undefined;
}

export default function SystemLogsPage() {
  const { projectId: pid, basePath } = useOutletContext<ProjectOutletContext>();
  const { dashboard, csrfToken } = useDashboard();
  const me = parseUser(dashboard?.user);

  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [items, setItems] = useState<AdminLogItem[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<number | null>(null);
  const [cleanupDays, setCleanupDays] = useState('90');
  const [busy, setBusy] = useState(false);

  const [entityType, setEntityType] = useState('');
  const [entityId, setEntityId] = useState('');
  const [userId, setUserId] = useState('');
  const [actionType, setActionType] = useState('');
  const [projectIdFilter, setProjectIdFilter] = useState('');
  const [since, setSince] = useState('');
  const [until, setUntil] = useState('');
  const [applied, setApplied] = useState<AdminLogListParams>({});
  const [catalogs, setCatalogs] = useState<CatalogLabelMaps>({
    requirementStatusById: new Map(),
    verificationStatusById: new Map(),
    categoryById: new Map(),
    applicabilityById: new Map(),
    methodById: new Map(),
  });

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const filterParams = useCallback(
    (pageNum: number): AdminLogListParams => ({
      ...applied,
      limit: PAGE_SIZE,
      offset: (pageNum - 1) * PAGE_SIZE,
    }),
    [applied],
  );

  const load = useCallback(
    async (pageNum: number) => {
      setLoading(true);
      setError(null);
      setNotice(null);
      try {
        const users = await listUsersOptional();
        if (users === null) {
          setAllowed(false);
          setItems([]);
          setTotal(0);
          return;
        }
        setAllowed(true);
        const [res, reqSt, verSt, cats, apps, methods] = await Promise.all([
          listAdminLogs(filterParams(pageNum)),
          listRequirementStatuses().catch(() => []),
          listVerificationStatuses().catch(() => []),
          listCategories().catch(() => []),
          listApplicability().catch(() => []),
          listVerificationMethods().catch(() => []),
        ]);
        setItems(res.items);
        setTotal(res.total);
        setCatalogs({
          requirementStatusById: new Map(reqSt.map((s) => [s.id, s.title])),
          verificationStatusById: new Map(verSt.map((s) => [s.id, s.title])),
          categoryById: new Map(cats.map((c) => [c.id, c.title])),
          applicabilityById: new Map(apps.map((a) => [a.id, a.title])),
          methodById: new Map(methods.map((m) => [m.id, m.title])),
        });
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load logs');
      } finally {
        setLoading(false);
      }
    },
    [filterParams],
  );

  useEffect(() => {
    void load(page);
  }, [load, page]);

  function onApplyFilters(e: FormEvent) {
    e.preventDefault();
    setPage(1);
    setApplied({
      entity_type: optionalText(entityType),
      entity_id: optionalNumber(entityId),
      user_id: optionalNumber(userId),
      action_type: optionalText(actionType),
      project_id: optionalNumber(projectIdFilter),
      since: optionalText(since),
      until: optionalText(until),
    });
  }

  async function onExport() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await downloadAdminLogsJson(applied);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setBusy(false);
    }
  }

  async function onCleanup() {
    const days = Number(cleanupDays);
    if (!Number.isInteger(days) || days < 1) {
      setError('Cleanup days must be an integer of at least 1');
      return;
    }
    if (
      !window.confirm(
        `Delete audit logs older than ${days} days? This cannot be undone.`,
      )
    ) {
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const res = await cleanupAdminLogs(days, csrfToken ?? '');
      setPage(1);
      await load(1);
      setNotice(`Removed ${res.deleted} log ${res.deleted === 1 ? 'entry' : 'entries'}.`);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Cleanup failed');
    } finally {
      setBusy(false);
    }
  }

  const pageCount = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const projectNameById = useMemo(() => {
    const m = new Map<number, string>();
    for (const p of dashboard?.projects ?? []) {
      m.set(p.id, p.name);
    }
    return m;
  }, [dashboard?.projects]);

  if (loading && allowed === null) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading…
      </div>
    );
  }

  if (allowed === false) {
    return (
      <div>
        <StitchPageHeader
          projectName={projectName}
          section="Admin"
          title="System logs"
          subtitle="Restricted area."
        />
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-center">
          <span className="material-symbols-outlined text-4xl text-stitch-muted mb-3 block">
            lock
          </span>
          <p className="text-stitch-fg font-semibold">Access denied</p>
          <p className="text-sm text-stitch-muted mt-2 max-w-md mx-auto">
            Viewing system logs requires a global administrator account. You are signed in as{' '}
            <span className="text-stitch-accent">{me?.username ?? '?'}</span>.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Admin"
        title="System logs"
        subtitle="Instance-wide audit trail. The project in the URL is only for navigation."
      >
        <Link
          to={`${basePath}/admin`}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher"
        >
          User directory
        </Link>
      </StitchPageHeader>

      <form
        onSubmit={onApplyFilters}
        className="mb-4 grid grid-cols-2 md:grid-cols-4 xl:grid-cols-8 gap-2 items-end"
      >
        <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
          Entity type
          <input
            className={`${inputClass} mt-1`}
            value={entityType}
            onChange={(e) => setEntityType(e.target.value)}
          />
        </label>
        <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
          Entity ID
          <input
            className={`${inputClass} mt-1`}
            value={entityId}
            onChange={(e) => setEntityId(e.target.value)}
            inputMode="numeric"
          />
        </label>
        <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
          User ID
          <input
            className={`${inputClass} mt-1`}
            value={userId}
            onChange={(e) => setUserId(e.target.value)}
            inputMode="numeric"
          />
        </label>
        <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
          Action
          <input
            className={`${inputClass} mt-1`}
            value={actionType}
            onChange={(e) => setActionType(e.target.value)}
          />
        </label>
        <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
          Project ID
          <input
            className={`${inputClass} mt-1`}
            value={projectIdFilter}
            onChange={(e) => setProjectIdFilter(e.target.value)}
            inputMode="numeric"
          />
        </label>
        <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
          Since
          <input
            type="datetime-local"
            className={`${inputClass} mt-1`}
            value={since}
            onChange={(e) => setSince(e.target.value)}
          />
        </label>
        <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
          Until
          <input
            type="datetime-local"
            className={`${inputClass} mt-1`}
            value={until}
            onChange={(e) => setUntil(e.target.value)}
          />
        </label>
        <button
          type="submit"
          className="text-xs font-bold uppercase tracking-wider text-stitch-canvas bg-stitch-accent rounded-md px-3 py-2 hover:opacity-90"
        >
          Filter
        </button>
      </form>

      <div className="mb-4 flex flex-wrap items-center gap-3">
        <button
          type="button"
          disabled={busy || allowed !== true}
          onClick={() => void onExport()}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher disabled:opacity-40"
        >
          Export JSON
        </button>
        <label className="flex items-center gap-2 text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
          Cleanup older than
          <input
            className={`${inputClass} w-20`}
            value={cleanupDays}
            onChange={(e) => setCleanupDays(e.target.value)}
            inputMode="numeric"
            aria-label="Cleanup days"
          />
          days
        </label>
        <button
          type="button"
          disabled={busy || allowed !== true}
          onClick={() => void onCleanup()}
          className="text-xs font-bold uppercase tracking-wider text-red-300 border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher disabled:opacity-40"
        >
          Cleanup
        </button>
      </div>

      {error ? <p className="text-sm text-red-300 mb-3">{error}</p> : null}
      {notice ? <p className="text-sm text-emerald-300 mb-3">{notice}</p> : null}

      {loading ? (
        <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
          Loading…
        </div>
      ) : (
        <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
                <th className="px-4 py-3">Time</th>
                <th className="px-4 py-3">User</th>
                <th className="px-4 py-3">Action</th>
                <th className="px-4 py-3">Entity</th>
                <th className="px-4 py-3">Project</th>
                <th className="px-4 py-3">Summary</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {items.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-4 py-8 text-center text-stitch-muted text-sm">
                    No log entries match the current filters.
                  </td>
                </tr>
              ) : (
                items.map((row) => (
                  <Fragment key={row.log_id}>
                    <tr
                      className="hover:bg-white/[0.03] cursor-pointer"
                      onClick={() =>
                        setExpanded((id) => (id === row.log_id ? null : row.log_id))
                      }
                    >
                      <td className="px-4 py-3 text-[11px] font-mono text-stitch-muted whitespace-nowrap">
                        {formatTs(row.created_at)}
                      </td>
                      <td className="px-4 py-3 text-stitch-fg">{row.username}</td>
                      <td className="px-4 py-3 uppercase text-[11px] tracking-wide text-stitch-accent">
                        {row.action_type}
                      </td>
                      <td className="px-4 py-3 font-mono text-xs text-stitch-muted">
                        {row.entity_type}
                        {row.entity_id != null ? ` #${row.entity_id}` : ''}
                      </td>
                      <td className="px-4 py-3 text-xs text-stitch-muted">
                        {row.project_id == null
                          ? '—'
                          : (projectNameById.get(row.project_id) ?? `#${row.project_id}`)}
                      </td>
                      <td className="px-4 py-3 text-stitch-fg">{row.summary}</td>
                    </tr>
                    {expanded === row.log_id ? (
                      <tr>
                        <td colSpan={6} className="px-4 py-3 bg-stitch-elevated/40">
                          {row.description ? (
                            <p className="text-xs text-stitch-fg/80 mb-2 whitespace-pre-wrap">
                              {row.description}
                            </p>
                          ) : null}
                          {row.changes.length === 0 ? (
                            <p className="text-[11px] text-stitch-muted">No field-level changes.</p>
                          ) : (
                            <ul className="space-y-1.5 text-[11px]">
                              {row.changes.map((ch, idx) => (
                                <li
                                  key={`${row.log_id}-${idx}-${ch.field}`}
                                  className="grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-2"
                                >
                                  <span className="font-bold text-stitch-muted">{ch.field}</span>
                                  <span className="sm:col-span-2 text-stitch-fg/90">
                                    <span className="text-red-300/90 line-through mr-2">
                                      {formatLogChangeValue(
                                        ch.field,
                                        ch.old_value,
                                        row.entity_type,
                                        catalogs,
                                      )}
                                    </span>
                                    <span>
                                      {formatLogChangeValue(
                                        ch.field,
                                        ch.new_value,
                                        row.entity_type,
                                        catalogs,
                                      )}
                                    </span>
                                  </span>
                                </li>
                              ))}
                            </ul>
                          )}
                        </td>
                      </tr>
                    ) : null}
                  </Fragment>
                ))
              )}
            </tbody>
          </table>
          {total > PAGE_SIZE ? (
            <div className="flex justify-end px-4 py-3 border-t border-stitch-border">
              <Pagination page={page} pageCount={pageCount} onPageChange={setPage} />
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}
