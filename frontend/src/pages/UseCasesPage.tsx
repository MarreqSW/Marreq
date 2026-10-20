import { FormEvent, useCallback, useEffect, useState } from 'react';
import { Link, useNavigate, useOutletContext, useSearchParams } from 'react-router-dom';
import { createUseCase, getNextUseCaseCode, listUseCases } from '@/api/client';
import type { UseCaseRow } from '@/api/types';
import Dialog from '@/components/Dialog';
import StitchPageHeader from '@/components/StitchPageHeader';
import { useDashboard } from '@/context/DashboardContext';
import { useProjectArchived } from '@/hooks/useProjectArchived';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { approvalChip, formatRelative } from '@/utils/useCaseDocument';

const STATES = [
  ['', 'All states'],
  ['unpublished', 'Unpublished'],
  ['draft', 'Draft'],
  ['reviewed', 'Reviewed'],
  ['approved', 'Approved'],
] as const;

const COLUMNS: { key: string; label: string; className?: string }[] = [
  { key: 'code', label: 'Code' },
  { key: 'title', label: 'Title' },
  { key: 'actor', label: 'Primary actor' },
  { key: '', label: 'Steps', className: 'text-center' },
  { key: '', label: 'Alt · Exc', className: 'text-center' },
  { key: 'approval', label: 'State' },
  { key: 'modified', label: 'Updated' },
];

/** Use cases of the project (issue #399, screen 1 of #388). */
export default function UseCasesPage() {
  const { projectId: pid, basePath } = useOutletContext<ProjectOutletContext>();
  const { dashboard, csrfToken } = useDashboard();
  const archived = useProjectArchived(pid);
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const projectName = dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const [rows, setRows] = useState<UseCaseRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [q, setQ] = useState('');
  const [state, setState] = useState('');
  const [sort, setSort] = useState<{ column: string; dir: 'asc' | 'desc' }>({
    column: 'code',
    dir: 'asc',
  });

  const [createOpen, setCreateOpen] = useState(false);
  const [code, setCode] = useState('');
  const [title, setTitle] = useState('');
  const [actor, setActor] = useState('');
  const [createErr, setCreateErr] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const load = useCallback(async () => {
    setErr(null);
    try {
      setRows(
        await listUseCases(pid, {
          q: q.trim() || undefined,
          approval_state: state || undefined,
          sort_column: sort.column,
          sort_dir: sort.dir,
        }),
      );
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load use cases');
    } finally {
      setLoading(false);
    }
  }, [pid, q, state, sort]);

  useEffect(() => {
    const t = window.setTimeout(() => void load(), 150);
    return () => window.clearTimeout(t);
  }, [load]);

  async function openCreate() {
    setCreateErr(null);
    setTitle('');
    setActor('');
    setCreateOpen(true);
    try {
      setCode(await getNextUseCaseCode(pid));
    } catch {
      setCode('');
    }
  }

  // `?new=1` (the top bar's "Create use case") opens the dialog.
  useEffect(() => {
    if (params.get('new') !== '1') return;
    setParams({}, { replace: true });
    if (!archived) void openCreate();
  }, [params]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!csrfToken) {
      setCreateErr('Missing CSRF token; refresh the page.');
      return;
    }
    setCreating(true);
    setCreateErr(null);
    try {
      const created = await createUseCase(
        pid,
        {
          reference_code: code.trim() || undefined,
          title: title.trim(),
          primary_actor: actor.trim() || undefined,
        },
        csrfToken,
      );
      navigate(`${basePath}/use-cases/${created.id}?edit=1`);
    } catch (e2) {
      setCreateErr(e2 instanceof Error ? e2.message : 'Create failed');
    } finally {
      setCreating(false);
    }
  }

  function toggleSort(column: string) {
    if (!column) return;
    setSort((s) =>
      s.column === column ? { column, dir: s.dir === 'asc' ? 'desc' : 'asc' } : { column, dir: 'asc' },
    );
  }

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Use cases"
        title="Use cases"
        subtitle="How the system behaves, step by step, with the alternatives and exceptions along the way."
      >
        {archived ? null : (
          <button
            type="button"
            onClick={() => void openCreate()}
            className="inline-flex items-center gap-1.5 rounded-md bg-linear-to-br from-primary to-primary-container px-4 py-2 text-xs font-bold uppercase tracking-widest text-white shadow-md"
          >
            <span className="material-symbols-outlined text-sm">add</span>
            New use case
          </button>
        )}
      </StitchPageHeader>

      <div className="mb-3 flex flex-wrap gap-2">
        <label className="flex max-w-sm flex-1 items-center gap-2 rounded-md border border-stitch-border bg-stitch-surface px-3 py-1.5 text-sm">
          <span className="material-symbols-outlined text-sm text-stitch-muted">search</span>
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Filter by code, title or actor…"
            aria-label="Filter use cases"
            className="w-full bg-transparent text-stitch-fg outline-hidden placeholder:text-stitch-muted"
          />
        </label>
        <select
          value={state}
          onChange={(e) => setState(e.target.value)}
          aria-label="Approval state"
          className="rounded-md border border-stitch-border bg-stitch-surface px-3 py-1.5 text-sm text-stitch-fg"
        >
          {STATES.map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
      </div>

      {err ? (
        <div role="alert" className="mb-3 rounded-xl border border-red-500/25 bg-red-500/10 p-4 text-sm text-red-700 dark:text-red-200">
          {err}
        </div>
      ) : null}

      <div className="overflow-x-auto rounded-xl border border-stitch-border bg-stitch-surface shadow-stitch">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] uppercase tracking-widest text-stitch-muted">
              {COLUMNS.map((c) => (
                <th key={c.label} className={`px-4 py-3 ${c.className ?? ''}`}>
                  {c.key ? (
                    <button
                      type="button"
                      onClick={() => toggleSort(c.key)}
                      className="inline-flex items-center gap-1 uppercase tracking-widest hover:text-stitch-fg"
                    >
                      {c.label}
                      {sort.column === c.key ? (
                        <span className="material-symbols-outlined text-xs">
                          {sort.dir === 'asc' ? 'arrow_upward' : 'arrow_downward'}
                        </span>
                      ) : null}
                    </button>
                  ) : (
                    c.label
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {loading ? (
              <tr>
                <td colSpan={COLUMNS.length} className="px-4 py-8 text-center text-stitch-muted">
                  Loading use cases…
                </td>
              </tr>
            ) : rows.length === 0 ? (
              <tr>
                <td colSpan={COLUMNS.length} className="px-4 py-8 text-center text-stitch-muted">
                  {q || state ? 'No use case matches the filter.' : 'No use cases yet.'}
                </td>
              </tr>
            ) : (
              rows.map((r) => {
                const chip = approvalChip(r.approval_state);
                return (
                  <tr key={r.id} className="hover:bg-stitch-elevated/40">
                    <td className="px-4 py-3">
                      <Link
                        to={`${basePath}/use-cases/${r.id}`}
                        className="whitespace-nowrap font-mono text-xs font-semibold text-stitch-accent hover:underline"
                      >
                        {r.reference_code}
                      </Link>
                    </td>
                    <td className="px-4 py-3 font-semibold text-stitch-fg">
                      <Link to={`${basePath}/use-cases/${r.id}`} className="hover:underline">
                        {r.title}
                      </Link>
                    </td>
                    <td className="px-4 py-3 text-stitch-muted">{r.primary_actor ?? '—'}</td>
                    <td className="px-4 py-3 text-center">{r.step_count}</td>
                    <td className="px-4 py-3 text-center text-stitch-muted">
                      {r.alternative_count} · {r.exception_count}
                    </td>
                    <td className="px-4 py-3">
                      <span className="flex flex-wrap items-center gap-1">
                        <span
                          className={`rounded-sm px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider ${chip.className}`}
                        >
                          {chip.label}
                          {r.current_version_number != null ? ` · v${r.current_version_number}` : ''}
                        </span>
                        {r.has_draft && r.current_version_number != null ? (
                          <span className="rounded-sm bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider text-amber-800 dark:text-amber-300">
                            Editing
                          </span>
                        ) : null}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-xs text-stitch-muted">{formatRelative(r.updated_at)}</td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
      {!loading && rows.length > 0 ? (
        <p className="mt-3 text-xs text-stitch-muted">
          {rows.length} use case{rows.length === 1 ? '' : 's'}
        </p>
      ) : null}

      <Dialog
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        title="New use case"
        subtitle="It starts as a draft; publish it when the main scenario is written."
      >
        <form onSubmit={(e) => void onCreate(e)} className="space-y-4">
          <div className="grid grid-cols-3 gap-3">
            <label className="block">
              <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Code</span>
              <input
                value={code}
                onChange={(e) => setCode(e.target.value.toUpperCase())}
                className="w-full rounded-md border border-stitch-border bg-stitch-elevated px-3 py-2 font-mono text-sm text-stitch-fg"
              />
            </label>
            <label className="col-span-2 block">
              <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Primary actor</span>
              <input
                value={actor}
                onChange={(e) => setActor(e.target.value)}
                placeholder="e.g. Ground operator"
                className="w-full rounded-md border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg"
              />
            </label>
          </div>
          <label className="block">
            <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Title</span>
            <input
              required
              minLength={3}
              maxLength={255}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="e.g. Downlink telemetry pass"
              className="w-full rounded-md border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg"
            />
          </label>
          {createErr ? (
            <p role="alert" className="text-sm text-red-600 dark:text-red-300">
              {createErr}
            </p>
          ) : null}
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setCreateOpen(false)}
              className="rounded-md border border-stitch-border px-3 py-1.5 text-xs font-semibold text-stitch-fg"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={creating}
              className="rounded-md bg-stitch-accent px-3 py-1.5 text-xs font-bold text-stitch-on-accent disabled:opacity-50"
            >
              {creating ? 'Creating…' : 'Create and edit'}
            </button>
          </div>
        </form>
      </Dialog>
    </div>
  );
}
