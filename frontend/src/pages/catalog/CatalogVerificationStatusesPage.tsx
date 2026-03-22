import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import {
  createVerificationStatus,
  deleteVerificationStatus,
  getMyPermissions,
  listVerificationStatuses,
  updateVerificationStatus,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { VerificationStatus, VerificationStatusWriteBody } from '@/api/types';
import { StatusBadge } from '@/components/StatusBadge';
import TagColorPicker from '@/components/TagColorPicker';
import { btnDanger, btnPrimary, inp } from './catalogUi';

export default function CatalogVerificationStatusesPage() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const { csrfToken } = useDashboard();
  const [rows, setRows] = useState<VerificationStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [canEdit, setCanEdit] = useState(false);
  const [draft, setDraft] = useState<{
    title: string;
    description: string;
    tag: string;
    tag_color: string | null;
  }>({ title: '', description: '', tag: '', tag_color: null });
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [all, perms] = await Promise.all([
        listVerificationStatuses(),
        getMyPermissions(pid).catch(() => null),
      ]);
      setRows(all.filter((s) => s.project_id === pid));
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
      const body: VerificationStatusWriteBody = {
        title: draft.title.trim(),
        description: draft.description.trim(),
        tag: draft.tag.trim() || 'TAG',
        project_id: pid,
        tag_color: draft.tag_color?.trim() || null,
      };
      await createVerificationStatus(body, token);
      setDraft({ title: '', description: '', tag: '', tag_color: null });
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Create failed');
    } finally {
      setBusy(false);
    }
  }

  async function saveRow(id: number) {
    const c = rows.find((r) => r.id === id);
    if (!c || !canEdit || !token || c.is_system) return;
    setBusy(true);
    setErr(null);
    try {
      const body: VerificationStatusWriteBody = {
        id: c.id,
        title: c.title.trim(),
        description: c.description.trim(),
        tag: c.tag.trim(),
        project_id: c.project_id,
        is_system: c.is_system,
        tag_color: c.tag_color?.trim() || null,
      };
      await updateVerificationStatus(c.id, body, token);
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  }

  async function removeRow(id: number) {
    const c = rows.find((r) => r.id === id);
    if (!c || c.is_system) return;
    if (!canEdit || !token) return;
    if (!window.confirm('Delete this verification status?')) return;
    setBusy(true);
    setErr(null);
    try {
      await deleteVerificationStatus(id, token);
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

      <form
        onSubmit={addRow}
        className="rounded-xl border border-stitch-border bg-stitch-surface p-4 space-y-3"
      >
        <h3 className="text-sm font-bold text-white">New verification status</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <input
            className={inp}
            placeholder="Title (e.g. Passed)"
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
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
              Color
            </label>
            <TagColorPicker
              value={draft.tag_color}
              onChange={(v) => setDraft((d) => ({ ...d, tag_color: v }))}
              disabled={!canEdit || busy}
              inputClassName={inp}
            />
          </div>
          <input
            className={`md:col-span-3 ${inp}`}
            placeholder="Description"
            value={draft.description}
            onChange={(e) => setDraft((d) => ({ ...d, description: e.target.value }))}
          />
        </div>
        <button type="submit" disabled={!canEdit || busy} className={btnPrimary}>
          Add status
        </button>
      </form>

      <div className="rounded-xl border border-stitch-border overflow-hidden bg-stitch-surface">
        <table className="w-full text-left text-sm">
          <thead className="bg-stitch-elevated text-[10px] uppercase text-stitch-muted font-bold">
            <tr>
              <th className="px-3 py-2">Title</th>
              <th className="px-3 py-2">Tag</th>
              <th className="px-3 py-2">Color</th>
              <th className="px-3 py-2">Description</th>
              <th className="px-3 py-2">Flags</th>
              <th className="px-3 py-2 w-28">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {sorted.map((c) => {
              const ro = c.is_system || !canEdit || busy;
              return (
                <tr key={c.id} className="hover:bg-white/[0.02]">
                  <td className="px-3 py-2 align-top space-y-2">
                    <input
                      className={inp}
                      value={c.title}
                      disabled={ro}
                      onChange={(e) =>
                        setRows((prev) =>
                          prev.map((x) => (x.id === c.id ? { ...x, title: e.target.value } : x)),
                        )
                      }
                    />
                    <StatusBadge title={c.title.trim() || '—'} tagColor={c.tag_color} />
                  </td>
                  <td className="px-3 py-2 align-top w-28">
                    <input
                      className={inp}
                      value={c.tag}
                      disabled={ro}
                      onChange={(e) =>
                        setRows((prev) =>
                          prev.map((x) => (x.id === c.id ? { ...x, tag: e.target.value } : x)),
                        )
                      }
                    />
                  </td>
                  <td className="px-3 py-2 align-top min-w-[220px]">
                    <TagColorPicker
                      value={c.tag_color}
                      onChange={(v) =>
                        setRows((prev) =>
                          prev.map((x) => (x.id === c.id ? { ...x, tag_color: v } : x)),
                        )
                      }
                      disabled={ro}
                      inputClassName={inp}
                    />
                  </td>
                  <td className="px-3 py-2 align-top">
                    <input
                      className={inp}
                      value={c.description}
                      disabled={ro}
                      onChange={(e) =>
                        setRows((prev) =>
                          prev.map((x) =>
                            x.id === c.id ? { ...x, description: e.target.value } : x,
                          ),
                        )
                      }
                    />
                  </td>
                  <td className="px-3 py-2 align-top text-xs text-stitch-muted">
                    {c.is_system ? (
                      <span className="border border-stitch-border rounded px-1.5 py-0.5">System</span>
                    ) : (
                      '—'
                    )}
                  </td>
                  <td className="px-3 py-2 align-top space-y-1">
                    <button
                      type="button"
                      disabled={ro}
                      className={`block w-full ${btnPrimary} py-1.5`}
                      onClick={() => void saveRow(c.id)}
                    >
                      Save
                    </button>
                    <button
                      type="button"
                      disabled={ro || c.is_system}
                      className={btnDanger}
                      onClick={() => void removeRow(c.id)}
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {sorted.length === 0 && (
          <p className="p-6 text-center text-stitch-muted text-sm">No statuses for this project.</p>
        )}
      </div>
    </div>
  );
}
