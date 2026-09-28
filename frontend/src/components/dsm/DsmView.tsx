import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { downloadDsmXlsx, getDsm, listCategories, listRequirements } from '@/api/client';
import type { Category, Dsm, Requirement } from '@/api/types';
import {
  DSM_LINK_TYPES,
  linkTypeMeta,
  readDsmParams,
  selectedLinkTypes,
  toggleLinkType,
  writeDsmParams,
  type DsmParams,
} from '@/utils/dsm';
import DsmGrid, { type DsmFocus } from './DsmGrid';
import DsmSidePanel from './DsmSidePanel';

type Props = { projectId: number; basePath: string };

const chipOn = 'border-stitch-accent bg-stitch-accent/15 text-stitch-fg';
const chipOff = 'border-stitch-border bg-stitch-elevated/50 text-stitch-muted hover:bg-stitch-higher line-through';
const label = 'text-[10px] uppercase tracking-widest text-stitch-muted font-bold mr-1';
const button =
  'rounded-md border border-stitch-border bg-stitch-surface px-3 py-1.5 text-xs font-bold uppercase tracking-wider text-stitch-accent hover:bg-stitch-higher disabled:opacity-50';

export default function DsmView({ projectId, basePath }: Props) {
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  const params = useMemo(() => readDsmParams(searchParams), [searchParams]);
  const paramsKey = searchParams.toString();

  const [dsm, setDsm] = useState<Dsm | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [categories, setCategories] = useState<Category[]>([]);
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [highlightedLoop, setHighlightedLoop] = useState<number | null>(null);
  /**
   * Problem clicked in the side panel, by requirement ids so it survives a reload
   * (e.g. the switch to Partition order). `seq` makes repeated clicks re-centre the matrix.
   */
  const [selection, setSelection] = useState<
    ({ kind: 'loop'; ids: number[] } | { kind: 'finding'; source: number; target: number }) & { seq: number } | null
  >(null);
  const [notice, setNotice] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setDsm(await getDsm(projectId, params));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not load the dependency matrix');
    } finally {
      setLoading(false);
    }
    // `params` is derived from the URL, so `paramsKey` covers every filter.
  }, [projectId, paramsKey]);

  useEffect(() => {
    void load();
  }, [load]);

  // Scope options: categories and requirements of the whole project.
  useEffect(() => {
    let cancelled = false;
    Promise.all([listCategories(), listRequirements(projectId)])
      .then(([cats, reqs]) => {
        if (cancelled) return;
        setCategories(cats.filter((c) => c.project_id === projectId));
        setRequirements(
          [...reqs].sort((a, b) =>
            a.reference_code.localeCompare(b.reference_code, undefined, { numeric: true }),
          ),
        );
      })
      .catch(() => {
        /* scope selectors stay empty; the matrix still works */
      });
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  const update = (next: DsmParams) => {
    setNotice(null);
    setSearchParams(writeDsmParams(searchParams, next), { replace: true });
  };

  const onExport = async () => {
    setExporting(true);
    try {
      await downloadDsmXlsx(projectId, params);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setExporting(false);
    }
  };

  const selected = selectedLinkTypes(params);
  const indexById = useMemo(() => new Map(dsm?.requirements.map((r) => [r.id, r.index]) ?? []), [dsm]);
  const loopKey = (ids: number[]) => [...ids].sort((a, b) => a - b).join(',');
  // Resolve the selection against the current data; it disappears if filters removed it.
  const selectedLoop = useMemo(() => {
    if (selection?.kind !== 'loop' || !dsm) return null;
    const i = dsm.loops.findIndex((l) => loopKey(l.requirement_ids) === loopKey(selection.ids));
    return i >= 0 ? i : null;
  }, [selection, dsm]);
  const selectedFinding = useMemo(() => {
    if (selection?.kind !== 'finding' || !dsm) return null;
    const row = indexById.get(selection.source);
    const col = indexById.get(selection.target);
    return row != null && col != null && dsm.cells.some((c) => c.row === row && c.col === col) ? { row, col } : null;
  }, [selection, dsm, indexById]);
  const highlightIds = useMemo(() => {
    const loop = highlightedLoop ?? selectedLoop;
    return loop != null && dsm ? new Set(dsm.loops[loop]?.requirement_ids) : undefined;
  }, [highlightedLoop, selectedLoop, dsm]);

  const loopSpan = (ids: number[]): [number, number] | null => {
    const indices = ids.map((id) => indexById.get(id)).filter((i): i is number => i != null);
    return indices.length ? [Math.min(...indices), Math.max(...indices)] : null;
  };

  const focus = useMemo<DsmFocus | null>(() => {
    if (!selection || !dsm) return null;
    // The order is part of the key: after a switch to Partition the loop is re-centred.
    if (selectedFinding) {
      const { row, col } = selectedFinding;
      return { key: `finding-${selection.seq}-${dsm.order}`, kind: 'finding', rows: [row, row], cols: [col, col] };
    }
    if (selectedLoop == null) return null;
    const span = loopSpan(dsm.loops[selectedLoop]!.requirement_ids);
    return span ? { key: `loop-${selection.seq}-${dsm.order}`, kind: 'loop', rows: span, cols: span } : null;
    // loopSpan only reads indexById, which follows dsm.
  }, [selection, dsm, selectedLoop, selectedFinding]);

  const nextSeq = () => (selection?.seq ?? 0) + 1;
  const selectLoop = (index: number) => {
    if (!dsm) return;
    if (selectedLoop === index) {
      setSelection(null);
      return;
    }
    const ids = dsm.loops[index]!.requirement_ids;
    setSelection({ kind: 'loop', ids, seq: nextSeq() });
    // In hierarchy order a loop can span distant category blocks; Partition order keeps
    // its requirements next to each other, so the focused area is compact.
    const span = loopSpan(ids);
    if (params.order === 'hierarchy' && span && span[1] - span[0] + 1 > ids.length) {
      update({ ...params, order: 'partition' });
      setNotice('Switched to Partition order so the loop’s requirements are next to each other.');
    } else {
      setNotice(null);
    }
  };
  const selectFinding = (row: number, col: number) => {
    if (!dsm) return;
    setNotice(null);
    setSelection(
      selectedFinding?.row === row && selectedFinding.col === col
        ? null
        : {
            kind: 'finding',
            source: dsm.requirements[row]!.id,
            target: dsm.requirements[col]!.id,
            seq: nextSeq(),
          },
    );
  };

  return (
    <div className="space-y-4">
      <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4 space-y-3">
        <div className="flex flex-wrap items-center gap-2" role="group" aria-label="Link types">
          <span className={label}>Link types</span>
          {DSM_LINK_TYPES.map((t) => {
            const meta = linkTypeMeta(t);
            const on = selected.includes(t);
            return (
              <button
                key={t}
                type="button"
                aria-pressed={on}
                onClick={() => update(toggleLinkType(params, t))}
                className={`inline-flex items-center gap-2 rounded-md border px-2.5 py-1 text-xs font-semibold ${on ? chipOn : chipOff}`}
              >
                <span
                  className={`inline-flex h-4 w-4 items-center justify-center rounded font-mono text-[10px] font-bold text-white dark:text-stitch-canvas ${meta.chip}`}
                >
                  {meta.letter}
                </span>
                {meta.label}
              </button>
            );
          })}
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <span className={label}>Order</span>
          <div className="flex p-1 bg-stitch-surface rounded-lg gap-1 border border-stitch-border">
            {(['hierarchy', 'partition'] as const).map((o) => (
              <button
                key={o}
                type="button"
                aria-pressed={params.order === o}
                onClick={() => update({ ...params, order: o })}
                className={`px-3 py-1 text-xs font-bold rounded-md ${
                  params.order === o
                    ? 'bg-stitch-elevated text-stitch-accent shadow-stitch-inset border border-stitch-border'
                    : 'text-stitch-muted hover:bg-stitch-higher hover:text-stitch-fg'
                }`}
              >
                {o === 'hierarchy' ? 'Hierarchy' : 'Partition'}
              </button>
            ))}
          </div>
          <label className="flex items-center gap-2">
            <span className={label}>Category</span>
            <select
              className="rounded-md border border-stitch-border bg-stitch-surface px-2 py-1 text-xs text-stitch-fg"
              value={params.categoryId ?? ''}
              onChange={(e) =>
                update({ ...params, categoryId: e.target.value ? Number(e.target.value) : null })
              }
            >
              <option value="">All categories</option>
              {categories.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.title}
                </option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-2">
            <span className={label}>Subtree</span>
            <select
              className="max-w-56 rounded-md border border-stitch-border bg-stitch-surface px-2 py-1 text-xs text-stitch-fg"
              value={params.rootId ?? ''}
              onChange={(e) => update({ ...params, rootId: e.target.value ? Number(e.target.value) : null })}
            >
              <option value="">Whole project</option>
              {requirements.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.reference_code} — {r.title}
                </option>
              ))}
            </select>
          </label>
          <span className="ml-auto font-mono text-[11px] text-stitch-muted" data-testid="dsm-stats">
            {dsm
              ? `${dsm.stats.requirements} req · ${dsm.stats.links} links · ${dsm.stats.loops} loop${
                  dsm.stats.loops === 1 ? '' : 's'
                } · ${dsm.stats.upstream_changed} upstream changed${
                  dsm.stats.external_links ? ` · ${dsm.stats.external_links} outside scope` : ''
                }`
              : ''}
          </span>
          <button type="button" className={button} onClick={() => void onExport()} disabled={exporting}>
            {exporting ? 'Exporting…' : 'Export Excel'}
          </button>
          <button type="button" className={button} onClick={() => void load()}>
            Refresh
          </button>
        </div>
      </div>

      {notice ? (
        <p role="status" className="rounded-lg border border-amber-500/40 bg-amber-400/10 px-3 py-2 text-xs text-stitch-fg">
          {notice}
        </p>
      ) : null}

      {error ? (
        <p role="alert" className="rounded-lg border border-stitch-danger/40 bg-stitch-danger/10 px-3 py-2 text-sm text-stitch-fg">
          {error}
        </p>
      ) : null}

      {loading && !dsm ? (
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-sm text-stitch-muted">
          Loading dependency matrix…
        </div>
      ) : dsm ? (
        <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_280px] items-start">
          <div className="min-w-0 space-y-2">
            <DsmGrid
              dsm={dsm}
              basePath={basePath}
              highlightIds={highlightIds}
              focus={focus}
              onOpenRequirement={(id) => navigate(`${basePath}/requirements/${id}`)}
            />
            <p className="text-[11px] text-stitch-muted">
              A mark in row <b>i</b>, column <b>j</b>: requirement i&apos;s current version links to requirement j.
              {params.order === 'partition'
                ? ' Partition order: dependencies come first, so marks above the diagonal are feedback.'
                : ' Framed blocks are categories; rows are nested by parent.'}{' '}
              Click a mark to open the requirement.
            </p>
          </div>
          <DsmSidePanel
            dsm={dsm}
            highlightedLoop={highlightedLoop}
            onHighlightLoop={setHighlightedLoop}
            selectedLoop={selectedLoop}
            selectedFinding={selectedFinding}
            onSelectLoop={selectLoop}
            onSelectFinding={selectFinding}
          />
        </div>
      ) : null}
    </div>
  );
}
