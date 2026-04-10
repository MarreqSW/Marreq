import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { useOutletContext } from 'react-router-dom';
import {
  createCustomField,
  deleteCustomField,
  getMyPermissions,
  listCustomFieldsByProject,
  updateCustomField,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { CustomFieldDefinition, CustomFieldWriteBody } from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { btnDanger, btnPrimary, inp } from './catalogUi';

const FIELD_TYPES = ['text', 'enum', 'boolean', 'number'] as const;

function enumLinesFromApi(ev: unknown): string {
  if (Array.isArray(ev)) return ev.map(String).join('\n');
  if (ev && typeof ev === 'string') return ev;
  return '';
}

export default function CatalogCustomFieldsPage() {
  const { projectId: pid } = useOutletContext<ProjectOutletContext>();
  const { csrfToken } = useDashboard();
  const [rows, setRows] = useState<CustomFieldDefinition[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [canEdit, setCanEdit] = useState(false);
  const [draft, setDraft] = useState({
    label: '',
    field_type: 'text' as (typeof FIELD_TYPES)[number],
    enum_lines: '',
    sort_order: '0',
  });
  const [enumLinesById, setEnumLinesById] = useState<Record<number, string>>({});
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [list, perms] = await Promise.all([
        listCustomFieldsByProject(pid),
        getMyPermissions(pid).catch(() => null),
      ]);
      setRows(list);
      const el: Record<number, string> = {};
      for (const f of list) {
        el[f.id] = enumLinesFromApi(f.enum_values);
      }
      setEnumLinesById(el);
      setCanEdit(Boolean(perms?.manage_custom_fields && (csrfToken ?? '').length));
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

  function buildPayload(
    f: CustomFieldDefinition,
    enumLines: string,
  ): CustomFieldWriteBody {
    const lines = enumLines
      .split('\n')
      .map((x) => x.trim())
      .filter(Boolean);
    return {
      label: f.label.trim(),
      field_type: f.field_type,
      enum_values: f.field_type === 'enum' ? (lines.length ? lines : null) : null,
      sort_order: f.sort_order,
    };
  }

  async function addRow(e: FormEvent) {
    e.preventDefault();
    if (!canEdit || !token) return;
    setBusy(true);
    setErr(null);
    try {
      const lines = draft.enum_lines
        .split('\n')
        .map((x) => x.trim())
        .filter(Boolean);
      const body: CustomFieldWriteBody = {
        label: draft.label.trim(),
        field_type: draft.field_type,
        enum_values: draft.field_type === 'enum' ? (lines.length ? lines : null) : null,
        sort_order: Number(draft.sort_order) || 0,
      };
      await createCustomField(pid, body, token);
      setDraft({ label: '', field_type: 'text', enum_lines: '', sort_order: '0' });
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
      const lines = enumLinesById[id] ?? '';
      await updateCustomField(pid, id, buildPayload(c, lines), token);
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  }

  async function removeRow(id: number) {
    if (!canEdit || !token) return;
    if (!window.confirm('Delete this custom field? Values on requirements will be removed.')) return;
    setBusy(true);
    setErr(null);
    try {
      await deleteCustomField(pid, id, token);
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Delete failed');
    } finally {
      setBusy(false);
    }
  }

  const sorted = useMemo(
    () => [...rows].sort((a, b) => a.sort_order - b.sort_order || a.label.localeCompare(b.label)),
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
          You need <strong className="text-stitch-accent">manage custom fields</strong> permission.
        </p>
      ) : null}

      <form
        onSubmit={addRow}
        className="rounded-xl border border-stitch-border bg-stitch-surface p-4 space-y-3"
      >
        <h3 className="text-sm font-bold text-stitch-fg">New custom field</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <input
            className={inp}
            placeholder="Label"
            value={draft.label}
            onChange={(e) => setDraft((d) => ({ ...d, label: e.target.value }))}
            required
          />
          <select
            className={inp}
            value={draft.field_type}
            disabled={!canEdit || busy}
            onChange={(e) =>
              setDraft((d) => ({
                ...d,
                field_type: e.target.value as (typeof FIELD_TYPES)[number],
              }))
            }
          >
            {FIELD_TYPES.map((t) => (
              <option key={t} value={t} className="bg-stitch-surface">
                {t}
              </option>
            ))}
          </select>
          <input
            className={inp}
            type="number"
            placeholder="Sort order"
            value={draft.sort_order}
            onChange={(e) => setDraft((d) => ({ ...d, sort_order: e.target.value }))}
          />
          {draft.field_type === 'enum' ? (
            <textarea
              className={`${inp} md:col-span-2 min-h-[80px]`}
              placeholder="Enum options (one per line)"
              value={draft.enum_lines}
              onChange={(e) => setDraft((d) => ({ ...d, enum_lines: e.target.value }))}
            />
          ) : null}
        </div>
        <button type="submit" disabled={!canEdit || busy} className={btnPrimary}>
          Add field
        </button>
      </form>

      <div className="rounded-xl border border-stitch-border overflow-hidden bg-stitch-surface">
        <table className="w-full text-left text-sm">
          <thead className="bg-stitch-elevated text-[10px] uppercase text-stitch-muted font-bold">
            <tr>
              <th className="px-3 py-2">Label</th>
              <th className="px-3 py-2">Type</th>
              <th className="px-3 py-2">Sort</th>
              <th className="px-3 py-2">Enum values</th>
              <th className="px-3 py-2 w-28">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {sorted.map((c) => (
              <tr key={c.id} className="hover:bg-stitch-higher/40">
                <td className="px-3 py-2 align-top">
                  <input
                    className={inp}
                    value={c.label}
                    disabled={!canEdit || busy}
                    onChange={(e) =>
                      setRows((prev) =>
                        prev.map((x) => (x.id === c.id ? { ...x, label: e.target.value } : x)),
                      )
                    }
                  />
                </td>
                <td className="px-3 py-2 align-top w-36">
                  <select
                    className={inp}
                    value={c.field_type}
                    disabled={!canEdit || busy}
                    onChange={(e) =>
                      setRows((prev) =>
                        prev.map((x) =>
                          x.id === c.id ? { ...x, field_type: e.target.value } : x,
                        ),
                      )
                    }
                  >
                    {FIELD_TYPES.map((t) => (
                      <option key={t} value={t} className="bg-stitch-surface">
                        {t}
                      </option>
                    ))}
                  </select>
                </td>
                <td className="px-3 py-2 align-top w-24">
                  <input
                    type="number"
                    className={inp}
                    value={c.sort_order}
                    disabled={!canEdit || busy}
                    onChange={(e) =>
                      setRows((prev) =>
                        prev.map((x) =>
                          x.id === c.id
                            ? { ...x, sort_order: Number(e.target.value) || 0 }
                            : x,
                        ),
                      )
                    }
                  />
                </td>
                <td className="px-3 py-2 align-top">
                  {c.field_type === 'enum' ? (
                    <textarea
                      className={`${inp} min-h-[72px]`}
                      disabled={!canEdit || busy}
                      value={enumLinesById[c.id] ?? ''}
                      onChange={(e) =>
                        setEnumLinesById((prev) => ({ ...prev, [c.id]: e.target.value }))
                      }
                    />
                  ) : (
                    <span className="text-stitch-muted text-xs">—</span>
                  )}
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
          <p className="p-6 text-center text-stitch-muted text-sm">No custom fields yet.</p>
        )}
      </div>
    </div>
  );
}
