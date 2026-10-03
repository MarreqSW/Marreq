import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { useOutletContext } from 'react-router-dom';
import {
  createVerificationStatus,
  deleteVerificationStatus,
  getMyPermissions,
  listVerificationStatuses,
  updateVerificationStatus,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { VerificationOutcome, VerificationStatus, VerificationStatusWriteBody } from '@/api/types';
import { StatusBadge } from '@/components/StatusBadge';
import TagColorPicker from '@/components/TagColorPicker';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { btnDanger, btnPrimary, inp } from './catalogUi';

/** What each status means for requirement close-out in the VCD (issue #353). */
const OUTCOMES: { value: VerificationOutcome; label: string }[] = [
  { value: 'passed', label: 'Passed' },
  { value: 'failed', label: 'Failed' },
  { value: 'in_progress', label: 'In progress' },
  { value: 'not_run', label: 'Not run' },
];

export default function CatalogVerificationStatusesPage() {
  const { projectId: pid } = useOutletContext<ProjectOutletContext>();
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
    /** `''` lets the server infer it from the title. */
    outcome: VerificationOutcome | '';
  }>({ title: '', description: '', tag: '', tag_color: null, outcome: '' });
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
      setCanEdit(Boolean(perms?.manage_project_configuration && (csrfToken ?? '').length));
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
        ...(draft.outcome ? { outcome: draft.outcome } : {}),
      };
      await createVerificationStatus(body, token);
      setDraft({ title: '', description: '', tag: '', tag_color: null, outcome: '' });
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
        ...(c.outcome ? { outcome: c.outcome } : {}),
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
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 text-red-800 dark:text-red-100 text-sm px-4 py-2">
          {err}
        </div>
      ) : null}
      {!canEdit ? (
        <p className="text-xs text-stitch-muted">
          You need <strong className="text-stitch-accent">manage project configuration</strong> permission
          (project admin).
        </p>
      ) : null}

      <form
        onSubmit={addRow}
        className="rounded-xl border border-stitch-border bg-stitch-surface p-4 space-y-3"
      >
        <h3 className="text-sm font-bold text-stitch-fg">New verification status</h3>
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
            className={`md:col-span-2 ${inp}`}
            placeholder="Description"
            value={draft.description}
            onChange={(e) => setDraft((d) => ({ ...d, description: e.target.value }))}
          />
          <select
            className={inp}
            aria-label="Close-out outcome"
            value={draft.outcome}
            onChange={(e) =>
              setDraft((d) => ({ ...d, outcome: e.target.value as VerificationOutcome | '' }))
            }
          >
            <option value="">Outcome: from the title</option>
            {OUTCOMES.map((o) => (
              <option key={o.value} value={o.value}>
                Outcome: {o.label}
              </option>
            ))}
          </select>
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
              <th className="px-3 py-2" title="What the status means for requirement close-out">
                Outcome
              </th>
              <th className="px-3 py-2">Flags</th>
              <th className="px-3 py-2 w-28">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {sorted.map((c) => {
              const ro = c.is_system || !canEdit || busy;
              return (
                <tr key={c.id} className="hover:bg-stitch-higher/40">
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
                  <td className="px-3 py-2 align-top w-36">
                    <select
                      className={inp}
                      aria-label={`Outcome of ${c.title}`}
                      value={c.outcome ?? 'not_run'}
                      disabled={ro}
                      onChange={(e) =>
                        setRows((prev) =>
                          prev.map((x) =>
                            x.id === c.id
                              ? { ...x, outcome: e.target.value as VerificationOutcome }
                              : x,
                          ),
                        )
                      }
                    >
                      {OUTCOMES.map((o) => (
                        <option key={o.value} value={o.value}>
                          {o.label}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="px-3 py-2 align-top text-xs text-stitch-muted">
                    {c.is_system ? (
                      <span className="border border-stitch-border rounded-sm px-1.5 py-0.5">System</span>
                    ) : (
                      '—'
                    )}
                  </td>
                  <td className="px-3 py-2 align-top">
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
                      className={`mt-1 ${btnDanger}`}
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
