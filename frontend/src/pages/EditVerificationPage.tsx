import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate, useOutletContext, useParams } from 'react-router-dom';
import {
  deleteVerificationGlobally,
  getVerification,
  getVerificationMatrix,
  listRequirements,
  listVerificationStatuses,
  listVerifications,
  putVerificationMatrix,
  updateVerificationField,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { Requirement, Verification, VerificationStatus } from '@/api/types';
import {
  matrixSelectionEquals,
  RequirementMatrixPicker,
} from '@/components/RequirementMatrixPicker';
import { statusTagColorSwatchStyle } from '@/components/StatusBadge';
import type { ProjectOutletContext } from '@/types/projectOutlet';

const selectClass =
  'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

export default function EditVerificationPage() {
  const { basePath } = useOutletContext<ProjectOutletContext>();
  const { projectId: projectIdParam, verificationId: verificationIdParam } = useParams();
  const pid = Number(projectIdParam);
  const vid = Number(verificationIdParam);
  const navigate = useNavigate();
  const { csrfToken, dashboard, refresh: refreshDashboard } = useDashboard();

  const [base, setBase] = useState<Verification | null>(null);
  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [siblings, setSiblings] = useState<Verification[]>([]);
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [linkedReqIds, setLinkedReqIds] = useState<number[]>([]);
  const [baselineLinkedIds, setBaselineLinkedIds] = useState<number[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);

  const [name, setName] = useState('');
  const [referenceCode, setReferenceCode] = useState('');
  const [description, setDescription] = useState('');
  const [source, setSource] = useState('');
  const [statusId, setStatusId] = useState(0);
  const [parentId, setParentId] = useState<string>('');

  const load = useCallback(async () => {
    if (!Number.isFinite(pid) || !Number.isFinite(vid)) return;
    setLoadError(null);
    try {
      const [v, st, all, reqs, mx] = await Promise.all([
        getVerification(vid),
        listVerificationStatuses(),
        listVerifications(),
        listRequirements(pid),
        getVerificationMatrix(pid, vid),
      ]);
      if (v.project_id !== pid) {
        setLoadError('This verification belongs to another project.');
        return;
      }
      setBase(v);
      setName(v.name);
      setReferenceCode(v.reference_code);
      setDescription(v.description);
      setSource(v.source);
      setStatusId(v.status_id);
      setParentId(v.parent_id != null ? String(v.parent_id) : '');
      setStatuses(st);
      setSiblings(all.filter((x) => x.project_id === pid && x.id !== vid));
      setRequirements(reqs);
      const ids = [...mx.requirement_ids].sort((a, b) => a - b);
      setLinkedReqIds(ids);
      setBaselineLinkedIds(ids);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load verification');
    }
  }, [pid, vid]);

  useEffect(() => {
    void load();
  }, [load]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';
  const projectSlug = dashboard?.projects?.find((p) => p.id === pid)?.slug;

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === pid);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, pid]);

  const statusMeta = useMemo(() => statuses.find((s) => s.id === statusId), [statuses, statusId]);

  const selectedParent = useMemo(() => {
    if (parentId === '') return null;
    const id = Number(parentId);
    if (!Number.isFinite(id)) return null;
    return siblings.find((x) => x.id === id) ?? null;
  }, [parentId, siblings]);

  const matrixDirty = useMemo(
    () => !matrixSelectionEquals(linkedReqIds, baselineLinkedIds),
    [linkedReqIds, baselineLinkedIds],
  );

  const dirty = useMemo(() => {
    if (!base) return false;
    return (
      name !== base.name ||
      referenceCode !== base.reference_code ||
      description !== base.description ||
      source !== base.source ||
      statusId !== base.status_id ||
      (parentId === '' ? null : Number(parentId)) !== base.parent_id ||
      matrixDirty
    );
  }, [base, name, referenceCode, description, source, statusId, parentId, matrixDirty]);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    if (!token || !base) {
      setSaveError('Missing CSRF token or data; refresh the page.');
      return;
    }
    setSaveError(null);
    setSaving(true);
    try {
      const updates: Array<{ field: string; value: string }> = [];
      if (name !== base.name) updates.push({ field: 'name', value: name.trim() });
      if (referenceCode !== base.reference_code) {
        updates.push({ field: 'reference_code', value: referenceCode.trim() });
      }
      if (description !== base.description) {
        updates.push({ field: 'description', value: description.trim() });
      }
      if (source !== base.source) updates.push({ field: 'source', value: source.trim() });
      if (statusId !== base.status_id) {
        updates.push({ field: 'status_id', value: String(statusId) });
      }
      const newParent = parentId === '' ? null : Number(parentId);
      if (newParent !== base.parent_id) {
        updates.push({
          field: 'parent_id',
          value: newParent == null ? '' : String(newParent),
        });
      }
      for (const u of updates) {
        await updateVerificationField(vid, u.field, u.value, token);
      }
      if (matrixDirty) {
        await putVerificationMatrix(pid, vid, { requirement_ids: linkedReqIds }, token);
        setBaselineLinkedIds([...linkedReqIds].sort((a, b) => a - b));
      }
      await refreshDashboard();
      await load();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  }

  async function deleteVerification() {
    if (
      !window.confirm(
        'Delete this verification permanently? Traceability links may be removed. This cannot be undone.',
      )
    ) {
      return;
    }
    const token = csrfToken ?? '';
    if (!token) {
      setSaveError('Missing CSRF token; refresh the page.');
      return;
    }
    setDeleteBusy(true);
    setSaveError(null);
    try {
      await deleteVerificationGlobally(vid, token);
      await refreshDashboard();
      navigate(`${basePath}/verifications`);
    } catch (e) {
      setSaveError(e instanceof Error ? e.message : 'Delete failed');
    } finally {
      setDeleteBusy(false);
    }
  }

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-200">
        {loadError}
        <div className="mt-3">
          <Link
            to={`${basePath}/verifications`}
            className="font-semibold text-stitch-accent underline"
          >
            Back to verifications
          </Link>
        </div>
      </div>
    );
  }

  if (!base) {
    return (
      <div className="text-stitch-muted text-sm py-12 text-center">Loading verification…</div>
    );
  }

  return (
    <div className="pb-28 font-body text-stitch-fg text-stitch max-w-4xl">
      <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <Link to={`${basePath}/verifications`} className="hover:text-stitch-accent transition-colors">
          Verifications
        </Link>
        <span className="material-symbols-outlined text-sm text-stitch-muted">chevron_right</span>
        <span className="text-stitch-accent font-bold">{base.reference_code || `#${base.id}`}</span>
      </nav>

      <div className="mb-8">
        <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
          <span>{projectName}</span>
          <span className="mx-2">/</span>
          <span className="text-stitch-accent font-bold">Edit verification</span>
        </nav>
        <h1 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
          {base.name}
        </h1>
        <p className="text-stitch-muted text-sm mt-2">
          Fields update via the API per changed column; traceability links save together with your changes.
        </p>
        {projectSlug ? (
          <p className="text-stitch-muted text-xs mt-2">
            <a
              href={`${basePath}/verifications/show/${vid}`}
              className="text-stitch-accent font-semibold hover:underline"
            >
              Classic verification page
            </a>{' '}
            — attachments and extra fields when available.
          </p>
        ) : null}
      </div>

      <form onSubmit={onSubmit} className="space-y-8">
        <section className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-6 md:p-8 space-y-6">
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
              Reference code
            </label>
            <input
              required
              value={referenceCode}
              onChange={(e) => setReferenceCode(e.target.value)}
              className={selectClass}
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
              Name
            </label>
            <input
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              className={selectClass}
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
              Description
            </label>
            <textarea
              required
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={5}
              className={`${selectClass} min-h-[120px] resize-y`}
            />
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Source
              </label>
              <input
                value={source}
                onChange={(e) => setSource(e.target.value)}
                className={selectClass}
              />
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Status
              </label>
              <div className="flex items-center gap-2">
                <div
                  className="w-2 h-2 rounded-full shrink-0 bg-stitch-accent/40"
                  style={statusTagColorSwatchStyle(statusMeta?.tag_color)}
                  title={statusMeta?.tag_color ? 'Catalog color' : undefined}
                />
                <select
                  className={`${selectClass} flex-1 min-w-0`}
                  value={statusId}
                  onChange={(e) => setStatusId(Number(e.target.value))}
                >
                  {statusOptions.map((s) => (
                    <option key={s.id} value={s.id} className="bg-stitch-surface text-stitch-fg">
                      {s.title}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <div className="sm:col-span-2">
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Parent verification (optional)
              </label>
              <select
                className={selectClass}
                value={parentId}
                onChange={(e) => setParentId(e.target.value)}
              >
                <option value="" className="bg-stitch-surface text-stitch-fg">
                  None
                </option>
                {siblings.map((v) => (
                  <option key={v.id} value={v.id} className="bg-stitch-surface text-stitch-fg">
                    {(v.reference_code ?? '').trim() || `#${v.id}`} —{' '}
                    {v.name.length > 48 ? `${v.name.slice(0, 48)}…` : v.name}
                  </option>
                ))}
              </select>
              {parentId !== '' ? (
                <div className="mt-1.5">
                  <Link
                    to={`${basePath}/verifications/${parentId}`}
                    className="text-xs font-mono font-semibold text-stitch-accent hover:underline"
                    title={selectedParent?.name?.trim() || undefined}
                  >
                    {(selectedParent?.reference_code ?? '').trim() || `#${parentId}`}
                  </Link>
                </div>
              ) : null}
            </div>
          </div>
          {base.verification_method_id != null && (
            <p className="text-xs text-stitch-muted">
              Verification method ID{' '}
              <span className="font-mono text-stitch-accent">{base.verification_method_id}</span>{' '}
              is set on this record. Changing it is not exposed in this UI yet.
            </p>
          )}
        </section>

        <section className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-6 md:p-8 space-y-4">
          <div>
            <h2 className="text-sm font-bold font-headline text-stitch-fg">Traceability (requirements)</h2>
            <p className="text-xs text-stitch-muted mt-1">
              Which requirements this verification covers in the matrix. Replaces all links for this test when you
              save.
            </p>
          </div>
          <RequirementMatrixPicker
            projectId={pid}
            basePath={basePath}
            requirements={requirements}
            selectedIds={linkedReqIds}
            onChange={setLinkedReqIds}
            disabled={saving}
          />
        </section>

        {saveError && (
          <div className="rounded-lg bg-red-500/15 border border-red-500/30 text-red-100 text-sm px-4 py-3">
            {saveError}
          </div>
        )}

        <footer className="fixed bottom-0 left-0 right-0 z-40 bg-stitch-surface/85 backdrop-blur-md border-t border-stitch-border px-4 md:px-8 py-3 flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              className="text-xs font-bold uppercase tracking-wider text-red-400/90 hover:text-red-300 px-2 py-2 disabled:opacity-40"
              disabled={deleteBusy || !(csrfToken ?? '').length}
              onClick={() => void deleteVerification()}
            >
              {deleteBusy ? 'Deleting…' : 'Delete verification'}
            </button>
            <Link
              to={`${basePath}/verifications`}
              className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-danger transition-colors px-2 py-2"
            >
              Back
            </Link>
          </div>
          <button
            type="submit"
            disabled={saving || !dirty}
            className="bg-stitch-accent text-stitch-canvas px-6 py-2.5 rounded-md text-xs font-bold uppercase tracking-widest shadow-stitch disabled:opacity-50 hover:bg-stitch-accent-dim transition-colors"
          >
            {saving ? 'Saving…' : 'Save changes'}
          </button>
        </footer>
      </form>
    </div>
  );
}
