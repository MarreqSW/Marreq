import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useOutletContext, useParams } from 'react-router-dom';
import {
  getMyPermissions,
  getVerification,
  getVerificationMatrix,
  listRequirements,
  listVerificationActivityByProject,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listUsersOptional,
  listVerifications,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import { StatusBadge } from '@/components/StatusBadge';
import type {
  EffectivePermissions,
  EntityActivityItem,
  Requirement,
  Verification,
  VerificationMethod,
  User,
  VerificationStatus,
} from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';

function formatTs(iso: string): string {
  try {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' });
  } catch {
    return iso;
  }
}

function formatRelativeTime(iso: string): string {
  try {
    const d = new Date(iso);
    const ms = Date.now() - d.getTime();
    if (Number.isNaN(d.getTime()) || ms < 0) return '';
    const minutes = Math.floor(ms / 60000);
    if (minutes < 1) return 'just now';
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days}d ago`;
    return d.toLocaleDateString(undefined, { dateStyle: 'short' });
  } catch {
    return '';
  }
}

function formatVerificationActivityValue(
  field: string,
  raw: string,
  statusById: Map<number, VerificationStatus>,
  methodById: Map<number, VerificationMethod>,
): string {
  const t = raw.trim();
  if (t === '—' || t === '') return t || '—';
  if (field === 'Status') {
    const id = Number(t);
    if (Number.isFinite(id)) return statusById.get(id)?.title ?? raw;
  }
  if (field === 'Verification type') {
    const id = Number(t);
    if (Number.isFinite(id)) {
      if (id === 0) return '—';
      return methodById.get(id)?.title ?? raw;
    }
  }
  return raw;
}

export default function ViewVerificationPage() {
  const { basePath } = useOutletContext<ProjectOutletContext>();
  const { projectId: projectIdParam, verificationId: verificationIdParam } = useParams();
  const pid = Number(projectIdParam);
  const vid = Number(verificationIdParam);
  const { dashboard } = useDashboard();

  const projectName = useMemo(
    () => dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project',
    [dashboard?.projects, pid],
  );

  const [row, setRow] = useState<Verification | null>(null);
  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [siblings, setSiblings] = useState<Verification[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [linkedReqIds, setLinkedReqIds] = useState<number[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [users, setUsers] = useState<User[] | null>(null);
  const [activityLog, setActivityLog] = useState<EntityActivityItem[]>([]);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid) || !Number.isFinite(vid)) return;
    setLoadError(null);
    try {
      const [v, st, m, all, p, reqs, mx, u, act] = await Promise.all([
        getVerification(vid),
        listVerificationStatuses(),
        listVerificationMethodsByProject(pid),
        listVerifications(),
        getMyPermissions(pid).catch(() => null),
        listRequirements(pid),
        getVerificationMatrix(pid, vid),
        listUsersOptional(),
        listVerificationActivityByProject(pid, vid).catch(() => [] as EntityActivityItem[]),
      ]);
      setUsers(u);
      if (v.project_id !== pid) {
        setLoadError('This verification belongs to another project.');
        return;
      }
      setRow(v);
      setStatuses(st);
      setMethods(m);
      setSiblings(all.filter((x) => x.project_id === pid && x.id !== vid));
      setPerms(p);
      setRequirements(reqs);
      setLinkedReqIds([...mx.requirement_ids].sort((a, b) => a - b));
      setActivityLog(act);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load verification');
    }
  }, [pid, vid]);

  useEffect(() => {
    void load();
  }, [load]);

  const statusById = useMemo(() => {
    const m = new Map<number, VerificationStatus>();
    for (const s of statuses) m.set(s.id, s);
    return m;
  }, [statuses]);

  const methodById = useMemo(() => {
    const m = new Map<number, VerificationMethod>();
    for (const x of methods) m.set(x.id, x);
    return m;
  }, [methods]);

  const parentRow = useMemo(() => {
    if (!row?.parent_id) return null;
    return siblings.find((x) => x.id === row.parent_id) ?? null;
  }, [row, siblings]);

  const reqById = useMemo(() => new Map(requirements.map((r) => [r.id, r])), [requirements]);

  const linkedRequirements = useMemo(() => {
    return linkedReqIds.map((id) => reqById.get(id)).filter(Boolean) as Requirement[];
  }, [linkedReqIds, reqById]);

  const canEdit = Boolean(perms?.edit_requirements);

  const userLabel = useCallback(
    (id: number) => {
      const u = users?.find((x) => x.id === id);
      if (u) return `${u.name} (${u.username})`;
      return `User #${id}`;
    },
    [users],
  );

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-800 dark:text-red-100">
        {loadError}
        <div className="mt-3">
          <Link to={`${basePath}/verifications`} className="font-semibold text-stitch-accent underline">
            Back to verifications
          </Link>
        </div>
      </div>
    );
  }

  if (!row) {
    return (
      <div className="text-stitch-muted text-sm py-12 text-center">Loading verification…</div>
    );
  }

  const vst = statusById.get(row.status_id);
  const methodTitle =
    row.verification_method_id == null
      ? '—'
      : methodById.get(row.verification_method_id)?.title ?? `Method #${row.verification_method_id}`;

  return (
    <div className="max-w-4xl pb-12">
      <nav className="flex flex-wrap items-center justify-between gap-3 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <div className="flex items-center gap-2 min-w-0">
          <Link to={`${basePath}/verifications`} className="hover:text-stitch-accent transition-colors">
            Verifications
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <span className="text-stitch-accent font-bold">{row.reference_code || `#${row.id}`}</span>
          <span className="text-stitch-muted font-normal normal-case tracking-normal">· View</span>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <a
            href={`${basePath}/verifications/show/${vid}`}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-stitch-border text-stitch-muted hover:text-stitch-accent text-[10px] font-bold uppercase tracking-wider transition-colors"
          >
            <span className="material-symbols-outlined text-sm">open_in_new</span>
            Classic
          </a>
          {canEdit ? (
            <Link
              to={`${basePath}/verifications/${vid}/edit`}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-gradient-to-br from-[#000666] to-[#1a237e] text-white text-[10px] font-bold uppercase tracking-wider shadow-lg hover:opacity-95 transition-opacity"
            >
              <span className="material-symbols-outlined text-sm">edit</span>
              Edit
            </Link>
          ) : null}
        </div>
      </nav>

      <div className="mb-6">
        <p className="text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">{projectName}</p>
        <h1 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
          {row.name.trim() || '—'}
        </h1>
      </div>

      <section className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-6 md:p-8 space-y-6">
        <dl className="grid grid-cols-1 sm:grid-cols-2 gap-6 text-sm">
          <div>
            <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Reference</dt>
            <dd className="font-mono text-stitch-accent font-semibold">{row.reference_code || `—`}</dd>
          </div>
          <div>
            <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Status</dt>
            <dd>
              {vst ? (
                <StatusBadge title={vst.title} tagColor={vst.tag_color} />
              ) : (
                <span className="text-stitch-fg">Status #{row.status_id}</span>
              )}
            </dd>
          </div>
          <div>
            <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Author</dt>
            <dd className="text-stitch-fg">{userLabel(row.author_id)}</dd>
          </div>
          <div>
            <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Reviewer</dt>
            <dd className="text-stitch-fg">{userLabel(row.reviewer_id)}</dd>
          </div>
          {row.status_set_by != null ? (
            <div className="sm:col-span-2">
              <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Last status change
              </dt>
              <dd className="text-stitch-fg text-xs">
                {userLabel(row.status_set_by)}
                {row.status_set_at ? ` · ${row.status_set_at}` : ''}
              </dd>
            </div>
          ) : null}
          <div className="sm:col-span-2">
            <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Description</dt>
            <dd className="text-stitch-fg whitespace-pre-wrap leading-relaxed">
              {row.description.trim() ? row.description : '—'}
            </dd>
          </div>
          <div>
            <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Source</dt>
            <dd className="text-stitch-fg">{row.source.trim() ? row.source : '—'}</dd>
          </div>
          <div>
            <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Verification type</dt>
            <dd className="text-stitch-fg">{methodTitle}</dd>
          </div>
          <div className="sm:col-span-2">
            <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Parent</dt>
            <dd>
              {row.parent_id == null ? (
                <span className="text-stitch-muted">—</span>
              ) : parentRow ? (
                <Link
                  to={`${basePath}/verifications/${row.parent_id}`}
                  className="font-mono font-semibold text-stitch-accent hover:underline"
                  title={parentRow.name?.trim() || undefined}
                >
                  {(parentRow.reference_code ?? '').trim() || `#${parentRow.id}`}
                </Link>
              ) : (
                <span className="text-stitch-muted">Parent #{row.parent_id}</span>
              )}
            </dd>
          </div>
        </dl>

      </section>

      <section className="mt-8 bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-6 md:p-8">
        <div className="flex flex-wrap items-start justify-between gap-3 mb-4">
          <div>
            <h2 className="text-sm font-bold font-headline text-stitch-fg">Linked requirements</h2>
            <p className="text-[10px] text-stitch-muted mt-1">
              Traceability matrix: this verification is linked from these requirements.
            </p>
          </div>
          {canEdit ? (
            <Link
              to={`${basePath}/verifications/${vid}/edit`}
              className="text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline"
            >
              Edit links
            </Link>
          ) : null}
        </div>
        {linkedReqIds.length === 0 ? (
          <p className="text-sm text-stitch-muted">
            No requirement links yet.
            {canEdit ? (
              <>
                {' '}
                <Link to={`${basePath}/verifications/${vid}/edit`} className="text-stitch-accent font-semibold hover:underline">
                  Add links in the editor
                </Link>
                .
              </>
            ) : null}
          </p>
        ) : (
          <ul className="space-y-2">
            {linkedRequirements.map((r) => (
              <li key={r.id}>
                <Link
                  to={`${basePath}/requirements/${r.id}`}
                  className="group flex flex-wrap items-baseline gap-x-2 gap-y-0.5 text-sm"
                >
                  <span className="font-mono text-stitch-accent font-semibold group-hover:underline">
                    {r.reference_code || `#${r.id}`}
                  </span>
                  <span className="text-stitch-fg group-hover:underline">{r.title}</span>
                </Link>
              </li>
            ))}
            {linkedReqIds.some((id) => !reqById.has(id)) ? (
              <li className="text-xs text-amber-200/90">
                Some linked requirement ids are missing from the project list (ids:{' '}
                {linkedReqIds.filter((id) => !reqById.has(id)).join(', ')}).
              </li>
            ) : null}
          </ul>
        )}
      </section>

      <section className="mt-8 bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
        <div className="px-6 py-3 border-b border-stitch-border bg-stitch-elevated">
          <div className="flex items-center gap-2">
            <span className="material-symbols-outlined text-stitch-accent text-xl">forum</span>
            <h2 className="text-sm font-bold font-headline text-stitch-accent">Discussion</h2>
          </div>
          <p className="text-[10px] text-stitch-muted mt-1">
            Requirement comments are available on each requirement; verifications do not have a separate REST thread in
            this app yet.
          </p>
        </div>
        <div className="p-4 md:p-6 text-sm text-stitch-muted leading-relaxed space-y-3">
          <p>
            To discuss this verification with your team, link it from related{' '}
            <Link to={`${basePath}/requirements`} className="text-stitch-accent font-semibold hover:underline">
              requirements
            </Link>{' '}
            (comments on the requirement) or your team&apos;s usual process outside this app.
          </p>
          {canEdit ? (
            <p>
              <Link
                to={`${basePath}/verifications/${vid}/edit`}
                className="text-stitch-accent font-bold hover:underline inline-flex items-center gap-1"
              >
                <span className="material-symbols-outlined text-base">edit</span>
                Open editor to update fields
              </Link>
            </p>
          ) : null}
        </div>
      </section>

      <section className="mt-8 bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
        <div className="px-6 py-3 border-b border-stitch-border bg-stitch-elevated flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2 min-w-0">
            <span className="material-symbols-outlined text-stitch-accent text-xl shrink-0">history</span>
            <div>
              <h2 className="text-sm font-bold font-headline text-stitch-accent">Changelog</h2>
              <p className="text-[10px] text-stitch-muted mt-0.5">
                Historic activity from the audit log: creates and field updates (newest first).
              </p>
            </div>
          </div>
          <a
            href={`${basePath}/verifications/show/${vid}`}
            className="text-[10px] font-bold uppercase tracking-wide text-stitch-accent hover:underline shrink-0"
          >
            Activity in classic →
          </a>
        </div>
        <div className="p-4 md:p-6 max-h-[min(520px,55vh)] overflow-y-auto">
          {activityLog.length === 0 ? (
            <p className="text-xs text-stitch-muted">No audit entries yet.</p>
          ) : (
            <ul className="space-y-0 divide-y divide-stitch-border">
              {activityLog.map((entry) => (
                <li key={entry.log_id} className="py-4 first:pt-0">
                  <div className="flex flex-wrap items-baseline justify-between gap-2 mb-1">
                    <span className="text-sm font-semibold text-stitch-fg">{entry.summary}</span>
                    <span className="text-[10px] font-mono text-stitch-muted">
                      {formatTs(entry.created_at)}
                      {formatRelativeTime(entry.created_at) ? ` · ${formatRelativeTime(entry.created_at)}` : ''}
                    </span>
                  </div>
                  <p className="text-[10px] text-stitch-muted mb-2">
                    <span className="font-semibold text-stitch-fg/90">{entry.username}</span>
                    <span className="mx-1">·</span>
                    <span className="uppercase tracking-wide">{entry.action_type}</span>
                  </p>
                  {entry.description ? (
                    <p className="text-xs text-stitch-fg/80 mb-2 whitespace-pre-wrap">{entry.description}</p>
                  ) : null}
                  {entry.changes.length > 0 ? (
                    <ul className="mt-2 space-y-1.5 text-[11px]">
                      {entry.changes.map((ch, idx) => (
                        <li
                          key={`${entry.log_id}-${idx}-${ch.field}`}
                          className="grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-2 text-stitch-fg/90"
                        >
                          <span className="font-bold text-stitch-muted">{ch.field}</span>
                          <span className="text-stitch-muted line-clamp-3 sm:col-span-2">
                            <span className="text-red-300/90 line-through decoration-stitch-border">
                              {formatVerificationActivityValue(ch.field, ch.old_value, statusById, methodById)}
                            </span>
                            <span className="mx-1 text-stitch-border">→</span>
                            <span className="text-emerald-200/90">
                              {formatVerificationActivityValue(ch.field, ch.new_value, statusById, methodById)}
                            </span>
                          </span>
                        </li>
                      ))}
                    </ul>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
          <p className="mt-4 text-[10px] text-stitch-muted leading-relaxed">
            Need historical snapshots, baseline comparisons, or attachments?{' '}
            <a href={`${basePath}/verifications/show/${vid}`} className="text-stitch-accent font-semibold hover:underline">
              Open the classic verification page
            </a>
            .
          </p>
        </div>
      </section>
    </div>
  );
}
