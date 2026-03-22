import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import {
  createVerification,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { Verification, VerificationMethod, VerificationStatus } from '@/api/types';
import { statusTagColorSwatchStyle } from '@/components/StatusBadge';

const selectClass =
  'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

export default function CreateVerificationPage() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const navigate = useNavigate();
  const { csrfToken, dashboard, refresh: refreshDashboard } = useDashboard();

  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [existing, setExisting] = useState<Verification[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [name, setName] = useState('');
  const [referenceCode, setReferenceCode] = useState('');
  const [description, setDescription] = useState('');
  const [source, setSource] = useState('manual');
  const [statusId, setStatusId] = useState(0);
  const [parentId, setParentId] = useState<string>(''); // "" = none
  const [methodId, setMethodId] = useState<string>(''); // "" = none

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoadError(null);
    try {
      const [st, meth, ver] = await Promise.all([
        listVerificationStatuses(),
        listVerificationMethodsByProject(pid),
        listVerifications(),
      ]);
      setStatuses(st);
      setMethods(meth);
      setExisting(ver.filter((v) => v.project_id === pid));
      const forProject = st.filter((s) => s.project_id === pid);
      const useSt = forProject.length > 0 ? forProject : st;
      if (useSt[0]) setStatusId((id) => (id === 0 ? useSt[0]!.id : id));
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load form data');
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === pid);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, pid]);

  const statusMeta = useMemo(() => statuses.find((s) => s.id === statusId), [statuses, statusId]);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    if (!token) {
      setSaveError('Missing CSRF token; refresh the page.');
      return;
    }
    setSaveError(null);
    setSaving(true);
    try {
      await createVerification(
        {
          name: name.trim(),
          reference_code: referenceCode.trim(),
          description: description.trim(),
          source: source.trim() || 'manual',
          status_id: statusId,
          parent_id: parentId === '' ? null : Number(parentId),
          project_id: pid,
          verification_method_id: methodId === '' ? null : Number(methodId),
        },
        token,
      );
      await refreshDashboard();
      navigate(`/p/${pid}/verifications`);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : 'Create failed');
    } finally {
      setSaving(false);
    }
  }

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-200">
        {loadError}
        <div className="mt-3">
          <Link
            to={`/p/${pid}/verifications`}
            className="font-semibold text-stitch-accent underline"
          >
            Back to verifications
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="pb-28 font-body text-stitch-fg text-stitch max-w-4xl">
      <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <Link to={`/p/${pid}/verifications`} className="hover:text-stitch-accent transition-colors">
          Verifications
        </Link>
        <span className="material-symbols-outlined text-sm text-stitch-muted">chevron_right</span>
        <span className="text-stitch-accent font-bold">New</span>
      </nav>

      <div className="mb-8">
        <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
          <span>{projectName}</span>
          <span className="mx-2">/</span>
          <span className="text-stitch-accent font-bold">Create verification</span>
        </nav>
        <h1 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
          Create verification
        </h1>
        <p className="text-stitch-muted text-sm mt-2">
          Add a test / verification record for this project. Link an optional parent for hierarchy.
        </p>
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
              placeholder="VER-0001"
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
              placeholder="Verification title"
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
              placeholder="What is being verified…"
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
                placeholder="manual, test_rig, …"
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
            <div>
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
                {existing.map((v) => (
                  <option key={v.id} value={v.id} className="bg-stitch-surface text-stitch-fg">
                    {v.reference_code} — {v.name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Verification method (optional)
              </label>
              <select
                className={selectClass}
                value={methodId}
                onChange={(e) => setMethodId(e.target.value)}
              >
                <option value="" className="bg-stitch-surface text-stitch-fg">
                  None
                </option>
                {methods.map((m) => (
                  <option key={m.id} value={m.id} className="bg-stitch-surface text-stitch-fg">
                    {m.title}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </section>

        {saveError && (
          <div className="rounded-lg bg-red-500/15 border border-red-500/30 text-red-100 text-sm px-4 py-3">
            {saveError}
          </div>
        )}

        <footer className="fixed bottom-0 left-0 right-0 z-40 bg-stitch-surface/85 backdrop-blur-md border-t border-stitch-border px-4 md:px-8 py-3 flex flex-wrap items-center justify-between gap-3">
          <Link
            to={`/p/${pid}/verifications`}
            className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-danger transition-colors px-2 py-2"
          >
            Cancel
          </Link>
          <button
            type="submit"
            disabled={saving}
            className="bg-stitch-accent text-stitch-canvas px-6 py-2.5 rounded-md text-xs font-bold uppercase tracking-widest shadow-stitch disabled:opacity-50 hover:bg-stitch-accent-dim transition-colors"
          >
            {saving ? 'Creating…' : 'Create verification'}
          </button>
        </footer>
      </form>
    </div>
  );
}
