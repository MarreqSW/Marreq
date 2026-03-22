import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  getMyPermissions,
  getVerification,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import { StatusBadge } from '@/components/StatusBadge';
import type {
  EffectivePermissions,
  Verification,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';

export default function ViewVerificationPage() {
  const { projectId: projectIdParam, verificationId: verificationIdParam } = useParams();
  const pid = Number(projectIdParam);
  const vid = Number(verificationIdParam);
  const { dashboard } = useDashboard();

  const projectSlug = useMemo(
    () => dashboard?.projects?.find((p) => p.id === pid)?.slug,
    [dashboard?.projects, pid],
  );
  const projectName = useMemo(
    () => dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project',
    [dashboard?.projects, pid],
  );

  const [row, setRow] = useState<Verification | null>(null);
  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [siblings, setSiblings] = useState<Verification[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid) || !Number.isFinite(vid)) return;
    setLoadError(null);
    try {
      const [v, st, m, all, p] = await Promise.all([
        getVerification(vid),
        listVerificationStatuses(),
        listVerificationMethodsByProject(pid),
        listVerifications(),
        getMyPermissions(pid).catch(() => null),
      ]);
      if (v.project_id !== pid) {
        setLoadError('This verification belongs to another project.');
        return;
      }
      setRow(v);
      setStatuses(st);
      setMethods(m);
      setSiblings(all.filter((x) => x.project_id === pid && x.id !== vid));
      setPerms(p);
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

  const canEdit = Boolean(perms?.edit_requirements);

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-800 dark:text-red-100">
        {loadError}
        <div className="mt-3">
          <Link to={`/p/${pid}/verifications`} className="font-semibold text-stitch-accent underline">
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
          <Link to={`/p/${pid}/verifications`} className="hover:text-stitch-accent transition-colors">
            Verifications
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <span className="text-stitch-accent font-bold">{row.reference_code || `#${row.id}`}</span>
          <span className="text-stitch-muted font-normal normal-case tracking-normal">· View</span>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {projectSlug ? (
            <a
              href={`/p/${projectSlug}/verifications/show/${vid}`}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-stitch-border text-stitch-muted hover:text-stitch-accent text-[10px] font-bold uppercase tracking-wider transition-colors"
            >
              <span className="material-symbols-outlined text-sm">open_in_new</span>
              Classic
            </a>
          ) : null}
          {canEdit ? (
            <Link
              to={`/p/${pid}/verifications/${vid}/edit`}
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
                  to={`/p/${pid}/verifications/${row.parent_id}`}
                  className="text-stitch-accent font-medium hover:underline"
                >
                  {parentRow.reference_code || `#${parentRow.id}`} — {parentRow.name}
                </Link>
              ) : (
                <span className="text-stitch-muted">Parent #{row.parent_id}</span>
              )}
            </dd>
          </div>
        </dl>

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
            <Link to={`/p/${pid}/requirements`} className="text-stitch-accent font-semibold hover:underline">
              requirements
            </Link>{' '}
            (comments on the requirement) or continue using your project&apos;s classic workflows if configured.
          </p>
          {canEdit ? (
            <p>
              <Link
                to={`/p/${pid}/verifications/${vid}/edit`}
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
                Version history, baselines, and detailed diffs for verifications are not exposed via this API yet.
              </p>
            </div>
          </div>
          {projectSlug ? (
            <a
              href={`/p/${projectSlug}/verifications/show/${vid}`}
              className="text-[10px] font-bold uppercase tracking-wide text-stitch-accent hover:underline shrink-0"
            >
              Activity in classic →
            </a>
          ) : null}
        </div>
        <div className="p-4 md:p-6 text-sm text-stitch-muted leading-relaxed">
          <p>
            Use the{' '}
            {projectSlug ? (
              <a
                href={`/p/${projectSlug}/verifications/show/${vid}`}
                className="text-stitch-accent font-semibold hover:underline"
              >
                classic verification page
              </a>
            ) : (
              <span className="text-stitch-fg">classic verification page</span>
            )}{' '}
            to review historical snapshots, baseline comparisons, attachments, and any audit-style activity your
            deployment records there.
          </p>
        </div>
      </section>
    </div>
  );
}
