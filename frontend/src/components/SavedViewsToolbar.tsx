import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import {
  createSavedView,
  deleteSavedView,
  listSavedViews,
  updateSavedView,
} from '@/api/client';
import type { SavedView, SavedViewVisibility } from '@/api/types';
import { useDashboard } from '@/context/DashboardContext';
import {
  buildSavedViewDefinition,
  parseSavedViewDefinition,
  type RequirementsQueryState,
} from '@/utils/savedViewDefinition';

type Props = {
  projectId: number;
  currentUserId: number | null;
  query: RequirementsQueryState;
  selectedViewId: number | null;
  onSelectView: (view: SavedView | null) => void;
  onApplyState: (state: RequirementsQueryState) => void;
};

export default function SavedViewsToolbar({
  projectId,
  currentUserId,
  query,
  selectedViewId,
  onSelectView,
  onApplyState,
}: Props) {
  const { csrfToken } = useDashboard();
  const [views, setViews] = useState<SavedView[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [showSave, setShowSave] = useState(false);
  const [saveName, setSaveName] = useState('');
  const [saveVisibility, setSaveVisibility] = useState<SavedViewVisibility>('private');

  const load = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      setViews(await listSavedViews(projectId));
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load saved views');
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const privateViews = useMemo(
    () => views.filter((v) => v.visibility === 'private'),
    [views],
  );
  const sharedViews = useMemo(
    () => views.filter((v) => v.visibility === 'shared'),
    [views],
  );

  const selected = views.find((v) => v.id === selectedViewId) ?? null;
  const canMutateSelected =
    selected &&
    !selected.locked &&
    (currentUserId != null && selected.owner_id === currentUserId);

  const applyView = (view: SavedView) => {
    onSelectView(view);
    onApplyState(parseSavedViewDefinition(view.definition));
  };

  const onPick = (raw: string) => {
    if (!raw) {
      onSelectView(null);
      return;
    }
    const id = Number(raw);
    const view = views.find((v) => v.id === id);
    if (view) applyView(view);
  };

  async function onSaveNew(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    if (!token || !saveName.trim()) return;
    setBusy(true);
    setErr(null);
    try {
      const created = await createSavedView(
        projectId,
        {
          name: saveName.trim(),
          description: null,
          visibility: saveVisibility,
          definition: buildSavedViewDefinition(query),
        },
        token,
      );
      setSaveName('');
      setShowSave(false);
      await load();
      onSelectView(created);
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  }

  async function onUpdateSelected() {
    if (!selected || !canMutateSelected) return;
    const token = csrfToken ?? '';
    if (!token) return;
    setBusy(true);
    setErr(null);
    try {
      const updated = await updateSavedView(
        projectId,
        selected.id,
        {
          name: selected.name,
          description: selected.description,
          visibility: selected.visibility,
          definition: buildSavedViewDefinition(query),
        },
        token,
      );
      await load();
      onSelectView(updated);
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : 'Update failed');
    } finally {
      setBusy(false);
    }
  }

  async function onDeleteSelected() {
    if (!selected || !canMutateSelected) return;
    if (!window.confirm(`Delete saved view “${selected.name}”?`)) return;
    const token = csrfToken ?? '';
    if (!token) return;
    setBusy(true);
    setErr(null);
    try {
      await deleteSavedView(projectId, selected.id, token);
      onSelectView(null);
      await load();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : 'Delete failed');
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="bg-stitch-elevated p-4 rounded-xl border border-stitch-border flex flex-wrap items-center gap-3">
      <div className="flex items-center gap-2 text-xs text-stitch-muted">
        <span className="material-symbols-outlined text-sm">bookmark</span>
        <span className="font-bold uppercase tracking-wider">Saved views</span>
      </div>
      <select
        aria-label="Saved views"
        disabled={loading || busy}
        value={selectedViewId ?? ''}
        onChange={(e) => onPick(e.target.value)}
        className="min-w-[180px] bg-stitch-surface border border-stitch-border rounded text-xs py-1.5 px-2 text-stitch-fg outline-none focus:border-stitch-accent"
      >
        <option value="">None</option>
        {privateViews.length > 0 && (
          <optgroup label="Private">
            {privateViews.map((v) => (
              <option key={v.id} value={v.id}>
                {v.name}
                {v.locked ? ' (locked)' : ''}
              </option>
            ))}
          </optgroup>
        )}
        {sharedViews.length > 0 && (
          <optgroup label="Shared">
            {sharedViews.map((v) => (
              <option key={v.id} value={v.id}>
                {v.name}
                {v.locked ? ' (locked)' : ''}
              </option>
            ))}
          </optgroup>
        )}
      </select>
      {selected?.locked ? (
        <span className="text-[10px] font-bold uppercase tracking-wider text-amber-200/90 border border-amber-500/30 rounded px-2 py-1">
          Used in baseline
        </span>
      ) : null}
      <button
        type="button"
        disabled={busy || !(csrfToken ?? '').length}
        onClick={() => setShowSave((s) => !s)}
        className="text-xs font-bold text-stitch-accent hover:underline disabled:opacity-40"
      >
        Save current as…
      </button>
      {canMutateSelected ? (
        <>
          <button
            type="button"
            disabled={busy}
            onClick={() => void onUpdateSelected()}
            className="text-xs font-bold text-stitch-fg/80 hover:text-stitch-accent hover:underline disabled:opacity-40"
          >
            Update
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => void onDeleteSelected()}
            className="text-xs font-bold text-red-300/90 hover:underline disabled:opacity-40"
          >
            Delete
          </button>
        </>
      ) : null}
      {showSave ? (
        <form
          onSubmit={(e) => void onSaveNew(e)}
          className="w-full flex flex-wrap items-end gap-2 border-t border-stitch-border/60 pt-3 mt-1"
        >
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
              Name
            </label>
            <input
              required
              value={saveName}
              onChange={(e) => setSaveName(e.target.value)}
              className="text-xs bg-stitch-surface border border-stitch-border rounded px-2 py-1.5 text-stitch-fg min-w-[160px]"
              placeholder="e.g. Open items"
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
              Visibility
            </label>
            <select
              value={saveVisibility}
              onChange={(e) => setSaveVisibility(e.target.value as SavedViewVisibility)}
              className="text-xs bg-stitch-surface border border-stitch-border rounded px-2 py-1.5 text-stitch-fg"
            >
              <option value="private">Private</option>
              <option value="shared">Shared</option>
            </select>
          </div>
          <button
            type="submit"
            disabled={busy || !saveName.trim()}
            className="text-xs font-bold bg-stitch-accent text-white rounded px-3 py-1.5 disabled:opacity-40"
          >
            Save
          </button>
          <button
            type="button"
            onClick={() => setShowSave(false)}
            className="text-xs text-stitch-muted hover:underline"
          >
            Cancel
          </button>
        </form>
      ) : null}
      {err ? <p className="w-full text-xs text-red-300">{err}</p> : null}
    </div>
  );
}
