import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { useOutletContext } from 'react-router-dom';
import {
  createVerificationMethod,
  deleteVerificationMethod,
  getMyPermissions,
  listVerificationMethodsByProject,
  updateVerificationMethod,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { VerificationMethod, VerificationMethodWriteBody } from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { btnDanger, btnPrimary, inp } from './catalogUi';

export default function CatalogVerificationMethodsPage() {
  const { projectId, basePath } = useOutletContext<ProjectOutletContext>();
  const pid = projectId;
  const { csrfToken, dashboard } = useDashboard();
  const projectSlug = dashboard?.projects?.find((p) => p.id === pid)?.slug;
  const [rows, setRows] = useState<VerificationMethod[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [canEdit, setCanEdit] = useState(false);
  const [draft, setDraft] = useState({ title: '', description: '', tag: '' });
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [list, perms] = await Promise.all([
        listVerificationMethodsByProject(pid),
        getMyPermissions(pid).catch(() => null),
      ]);
      setRows(list);
      setCanEdit(Boolean(perms?.edit_requirements && (csrfToken ?? '').length));
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Load failed');
    } finally {
      setLoading(false);
    }
  }, [pid, csrfToken]);

  useEffect(() => {
    void load();
  }, [load]);

  const token = csrfToken ?? '';

  async function addRow(e: FormEvent) {
    e.preventDefault();
    if (!canEdit || !token) return;
    setBusy(true);
    setErr(null);
    try {
      const body: VerificationMethodWriteBody = {
        id: null,
        title: draft.title.trim(),
        description: draft.description.trim(),
        tag: draft.tag.trim() || 'tag',
        project_id: pid,
      };
      await createVerificationMethod(pid, body, token);
      setDraft({ title: '', description: '', tag: '' });
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Create failed');
    } finally {
      setBusy(false);
    }
  }

  async function saveRow(id: number) {
    const c = rows.find((r) => r.id === id);
    if (!c || !canEdit || !token) return;
    setBusy(true);
    setErr(null);
    try {
      const body: VerificationMethodWriteBody = {
        id: c.id,
        title: c.title.trim(),
        description: c.description.trim(),
        tag: c.tag.trim(),
        project_id: c.project_id,
      };
      await updateVerificationMethod(pid, c.id, body, token);
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  }

  async function removeRow(id: number) {
    if (!canEdit || !token) return;
    if (!window.confirm('Delete this verification method? Requirements may reference it.')) return;
    setBusy(true);
    setErr(null);
    try {
      await deleteVerificationMethod(pid, id, token);
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Delete failed');
    } finally {
      setBusy(false);
    }
  }

  const sorted = useMemo(
    () => [...rows].sort((a, b) => a.title.localeCompare(b.title)),
    [rows],
  );

  if (loading) {
    return <p className="text-stitch-muted text-sm">Loading…</p>;
  }

  return (
    <div className="space-y-6">
      {err ? (
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 text-red-100 text-sm px-4 py-2">
          {err}
        </div>
      ) : null}
      {!canEdit ? (
        <p className="text-xs text-stitch-muted">
          You need <strong className="text-stitch-accent">edit requirements</strong> permission.
        </p>
      ) : null}

      {projectSlug ? (
        <p className="text-xs text-stitch-muted">
          Classic UI:{' '}
          <a
            href={`${basePath}/verification`}
            className="text-stitch-accent font-semibold hover:underline"
          >
            open legacy verification methods page
          </a>{' '}
          (same data; this SPA uses the JSON API).
        </p>
      ) : null}

      <form
        onSubmit={addRow}
        className="rounded-xl border border-stitch-border bg-stitch-surface p-4 space-y-3"
      >
        <h3 className="text-sm font-bold text-stitch-fg">New verification method</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <input
            className={inp}
            placeholder="Title"
            value={draft.title}
            onChange={(e) => setDraft((d) => ({ ...d, title: e.target.value }))}
            required
          />
          <input
            className={inp}
            placeholder="Tag"
            value={draft.tag}
            onChange={(e) => setDraft((d) => ({ ...d, tag: e.target.value }))}
          />
          <input
            className={`md:col-span-3 ${inp}`}
            placeholder="Description"
            value={draft.description}
            onChange={(e) => setDraft((d) => ({ ...d, description: e.target.value }))}
          />
        </div>
        <button type="submit" disabled={!canEdit || busy} className={btnPrimary}>
          Add method
        </button>
      </form>

      <div className="rounded-xl border border-stitch-border overflow-hidden bg-stitch-surface">
        <table className="w-full text-left text-sm">
          <thead className="bg-stitch-elevated text-[10px] uppercase text-stitch-muted font-bold">
            <tr>
              <th className="px-3 py-2">Title</th>
              <th className="px-3 py-2">Tag</th>
              <th className="px-3 py-2">Description</th>
              <th className="px-3 py-2 w-28">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {sorted.map((c) => (
              <tr key={c.id} className="hover:bg-white/[0.02]">
                <td className="px-3 py-2 align-top">
                  <input
                    className={inp}
                    value={c.title}
                    disabled={!canEdit || busy}
                    onChange={(e) =>
                      setRows((prev) =>
                        prev.map((x) => (x.id === c.id ? { ...x, title: e.target.value } : x)),
                      )
                    }
                  />
                </td>
                <td className="px-3 py-2 align-top w-36">
                  <input
                    className={inp}
                    value={c.tag}
                    disabled={!canEdit || busy}
                    onChange={(e) =>
                      setRows((prev) =>
                        prev.map((x) => (x.id === c.id ? { ...x, tag: e.target.value } : x)),
                      )
                    }
                  />
                </td>
                <td className="px-3 py-2 align-top">
                  <input
                    className={inp}
                    value={c.description}
                    disabled={!canEdit || busy}
                    onChange={(e) =>
                      setRows((prev) =>
                        prev.map((x) =>
                          x.id === c.id ? { ...x, description: e.target.value } : x,
                        ),
                      )
                    }
                  />
                </td>
                <td className="px-3 py-2 align-top space-y-1">
                  <button
                    type="button"
                    disabled={!canEdit || busy}
                    className={`block w-full ${btnPrimary} py-1.5`}
                    onClick={() => void saveRow(c.id)}
                  >
                    Save
                  </button>
                  <button
                    type="button"
                    disabled={!canEdit || busy}
                    className={btnDanger}
                    onClick={() => void removeRow(c.id)}
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {sorted.length === 0 && (
          <p className="p-6 text-center text-stitch-muted text-sm">No verification methods.</p>
        )}
      </div>
    </div>
  );
}
