import { type FormEvent, useEffect, useState } from 'react';
import { getProjectStorage, setProjectStorageQuota } from '@/api/client';
import type { ProjectStorage } from '@/api/types';
import { formatBytes } from '@/utils/formatBytes';

type Props = {
  projectId: number;
  /** Instance administrator: may change the quota. */
  isAdmin: boolean;
  csrfToken: string;
};

const MB = 1024 * 1024;
const labelCls = 'block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1';

/** Attachment storage used by the project, its quota and the per-file limit. */
export default function ProjectStorageSettings({ projectId, isAdmin, csrfToken }: Props) {
  const [storage, setStorage] = useState<ProjectStorage | null>(null);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [saveErr, setSaveErr] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getProjectStorage(projectId)
      .then((s) => {
        if (cancelled) return;
        setStorage(s);
        setDraft(String(Math.round(s.quota_bytes / MB)));
      })
      .catch((err: unknown) => {
        if (!cancelled) setLoadErr(err instanceof Error ? err.message : 'Could not load storage usage.');
      });
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  async function save(quotaMb: number | null) {
    setBusy(true);
    setSaveErr(null);
    setSaved(false);
    try {
      const next = await setProjectStorageQuota(projectId, quotaMb, csrfToken);
      setStorage(next);
      setDraft(String(Math.round(next.quota_bytes / MB)));
      setSaved(true);
    } catch (err) {
      setSaveErr(err instanceof Error ? err.message : 'Could not change the quota');
    } finally {
      setBusy(false);
    }
  }

  function onSubmit(e: FormEvent) {
    e.preventDefault();
    const mb = Number(draft);
    if (!Number.isInteger(mb) || mb < 1) {
      setSaveErr('Enter a whole number of megabytes (at least 1).');
      return;
    }
    void save(mb);
  }

  const pct = storage ? Math.min(100, Math.round((storage.used_bytes / Math.max(1, storage.quota_bytes)) * 100)) : 0;

  return (
    <section id="project-storage" className="mb-10 scroll-mt-8">
      <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-widest mb-4">Storage</h3>
      {loadErr ? <p className="text-sm text-red-300">{loadErr}</p> : null}
      {!storage && !loadErr ? <p className="text-sm text-stitch-muted">Loading…</p> : null}
      {storage ? (
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4 max-w-4xl space-y-4">
          <div>
            <div
              className="h-2 rounded-full bg-stitch-elevated overflow-hidden"
              role="progressbar"
              aria-label="Attachment storage used"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={pct}
            >
              <div className={`h-full ${pct >= 90 ? 'bg-red-400' : 'bg-stitch-accent'}`} style={{ width: `${pct}%` }} />
            </div>
            <p className="mt-2 text-sm text-stitch-fg">
              {formatBytes(storage.used_bytes)} of {formatBytes(storage.quota_bytes)} used ({pct}%)
              {storage.quota_is_default ? (
                <span className="text-stitch-muted"> · instance default</span>
              ) : (
                <span className="text-stitch-muted"> · set for this project (default {formatBytes(storage.default_quota_bytes)})</span>
              )}
            </p>
            {storage.retained_by_baselines_bytes > 0 ? (
              <p className="text-xs text-stitch-muted">
                {formatBytes(storage.retained_by_baselines_bytes)} of it are deleted files that baselines still keep.
              </p>
            ) : null}
            <p className="text-xs text-stitch-muted">
              Files can be at most {formatBytes(storage.max_file_bytes)} each. Allowed types:{' '}
              {storage.allowed_extensions.join(', ')}.
            </p>
          </div>

          {isAdmin ? (
            <form noValidate onSubmit={onSubmit} aria-label="Storage quota" className="flex flex-wrap items-end gap-3">
              <label>
                <span className={labelCls}>Quota (MB)</span>
                <input
                  type="number"
                  min={1}
                  step={1}
                  value={draft}
                  onChange={(e) => {
                    setDraft(e.target.value);
                    setSaved(false);
                    setSaveErr(null);
                  }}
                  className="w-36 rounded-md border border-stitch-border bg-stitch-surface px-3 py-2 text-sm text-stitch-fg"
                />
              </label>
              <button
                type="submit"
                disabled={busy}
                className="rounded-md bg-stitch-accent px-4 py-2 text-xs font-bold uppercase tracking-wider text-stitch-on-accent disabled:opacity-50"
              >
                Set quota
              </button>
              {!storage.quota_is_default ? (
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void save(null)}
                  className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-accent disabled:opacity-50"
                >
                  Reset to default
                </button>
              ) : null}
              {saveErr ? (
                <span role="alert" className="text-xs text-red-400">
                  {saveErr}
                </span>
              ) : null}
              {saved ? (
                <span role="status" className="text-xs text-emerald-600 dark:text-emerald-400">
                  Saved.
                </span>
              ) : null}
            </form>
          ) : (
            <p className="text-xs text-stitch-muted">Only instance administrators can change the quota.</p>
          )}
        </div>
      ) : null}
    </section>
  );
}
