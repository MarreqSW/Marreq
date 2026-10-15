import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { listBaselineUseCases } from '@/api/client';
import type { BaselineUseCase } from '@/api/types';
import { approvalChip } from '@/utils/useCaseDocument';

/** The use case versions a baseline recorded, and which changed since (issue #402). */
export default function BaselineUseCasesSection({
  projectId,
  baselineId,
  basePath,
}: {
  projectId: number;
  baselineId: number;
  basePath: string;
}) {
  const [rows, setRows] = useState<BaselineUseCase[] | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    Promise.resolve()
      .then(() => listBaselineUseCases(projectId, baselineId))
      .then((r) => live && setRows(r))
      .catch((e: unknown) => {
        if (!live) return;
        setRows([]);
        setErr(e instanceof Error ? e.message : 'Failed to load use cases');
      });
    return () => {
      live = false;
    };
  }, [projectId, baselineId]);

  return (
    <section className="mb-8">
      <h3 className="mb-3 text-sm font-bold uppercase tracking-widest text-stitch-fg">
        Use cases ({rows?.length ?? '…'})
      </h3>
      {err ? (
        <p role="alert" className="mb-2 text-xs text-red-400">
          {err}
        </p>
      ) : null}
      <div className="max-h-96 overflow-hidden overflow-y-auto rounded-xl border border-stitch-border bg-stitch-surface">
        <table className="w-full text-left text-xs">
          <thead className="sticky top-0 border-b border-stitch-border bg-stitch-elevated">
            <tr className="uppercase text-stitch-muted">
              <th className="px-3 py-2">Code</th>
              <th className="px-3 py-2">Title</th>
              <th className="px-3 py-2">Recorded</th>
              <th className="px-3 py-2">Now</th>
              <th className="px-3 py-2">Links</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {rows == null ? null : rows.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-3 py-6 text-center text-stitch-muted">
                  No published use cases in baseline
                </td>
              </tr>
            ) : (
              rows.map((r) => {
                const chip = approvalChip(r.approval_state);
                const changed = r.current_version_id != null && r.current_version_id !== r.version_id;
                return (
                  <tr key={r.use_case_id} className="hover:bg-white/3">
                    <td className="px-3 py-2">
                      <Link
                        to={`${basePath}/use-cases/${r.use_case_id}?version=${r.version_id}`}
                        className="font-mono text-stitch-accent hover:underline"
                      >
                        {r.reference_code}
                      </Link>
                    </td>
                    <td className="px-3 py-2 text-stitch-fg">{r.title}</td>
                    <td className="px-3 py-2">
                      <span className="font-mono">v{r.version_number}</span>{' '}
                      <span className={`rounded-sm px-1 py-px text-[9px] font-bold uppercase ${chip.className}`}>
                        {chip.label}
                      </span>
                    </td>
                    <td className="px-3 py-2">
                      {changed ? (
                        <Link
                          to={`${basePath}/use-cases/${r.use_case_id}?view=review&from=${r.version_id}`}
                          className="font-semibold text-amber-700 hover:underline dark:text-amber-300"
                        >
                          v{r.current_version_number}: compare
                        </Link>
                      ) : (
                        <span className="text-stitch-muted">unchanged</span>
                      )}
                    </td>
                    <td className="px-3 py-2 text-stitch-muted">{r.link_count}</td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
