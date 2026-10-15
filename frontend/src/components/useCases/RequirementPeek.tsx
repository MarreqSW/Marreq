import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { getRequirementByProject } from '@/api/client';
import type { RequirementDetailPayload, UseCaseLink } from '@/api/types';
import StatementText from '@/components/StatementText';
import { approvalChip } from '@/utils/useCaseDocument';

type Props = {
  projectId: number;
  basePath: string;
  link: UseCaseLink | null;
  onClose: () => void;
};

/** A linked requirement in a side panel, keeping the reading position (screen 7 of #388). */
export default function RequirementPeek({ projectId, basePath, link, onClose }: Props) {
  const [req, setReq] = useState<RequirementDetailPayload | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const rid = link?.requirement_id ?? null;

  useEffect(() => {
    if (rid == null) return;
    setReq(null);
    setErr(null);
    getRequirementByProject(projectId, rid)
      .then(setReq)
      .catch((e: unknown) => setErr(e instanceof Error ? e.message : 'Failed to load the requirement'));
  }, [projectId, rid]);

  useEffect(() => {
    if (!link) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [link, onClose]);

  if (!link || rid == null) return null;
  const chip = approvalChip(req?.approval_state);
  return (
    <aside
      role="complementary"
      aria-label={`Requirement ${link.target_reference_code}`}
      className="fixed inset-y-0 right-0 z-60 flex w-full max-w-md flex-col border-l border-stitch-border bg-stitch-surface shadow-2xl"
    >
      <header className="flex items-start gap-2 border-b border-stitch-border px-5 py-4">
        <div className="min-w-0">
          <div className="mb-1 flex items-center gap-2">
            <span className="font-mono text-xs font-semibold text-stitch-accent">{link.target_reference_code}</span>
            {req ? (
              <span className={`rounded-sm px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider ${chip.className}`}>
                {chip.label}
              </span>
            ) : null}
          </div>
          <h2 className="font-headline text-lg font-bold text-stitch-fg">{req?.title ?? link.target_title}</h2>
        </div>
        <button
          type="button"
          aria-label="Close"
          onClick={onClose}
          className="ml-auto rounded-lg p-1.5 text-stitch-muted hover:bg-stitch-elevated hover:text-stitch-fg"
        >
          <span className="material-symbols-outlined">close</span>
        </button>
      </header>
      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-5 py-4 text-sm">
        {link.suspect ? (
          <p className="flex items-start gap-1.5 rounded-md bg-amber-500/10 px-3 py-2 text-xs text-amber-800 dark:text-amber-300">
            <span className="material-symbols-outlined text-sm">link_off</span>
            {link.target_reference_code} changed since it was linked
            {link.suspect_reason ? ` (${link.suspect_reason})` : ''}. Check the step still matches, then clear the flag.
          </p>
        ) : null}
        {err ? <p role="alert" className="text-red-700 dark:text-red-300">{err}</p> : null}
        {req ? <StatementText source={req.description} className="text-stitch-fg" /> : !err ? <p className="text-stitch-muted">Loading…</p> : null}
        {link.rationale ? (
          <p className="text-xs text-stitch-muted">
            <strong>Why linked:</strong> {link.rationale}
          </p>
        ) : null}
      </div>
      <footer className="border-t border-stitch-border px-5 py-3">
        <Link
          to={`${basePath}/requirements/${rid}`}
          className="inline-flex items-center gap-1 text-xs font-bold uppercase tracking-wider text-stitch-accent hover:underline"
        >
          Open requirement
          <span className="material-symbols-outlined text-sm">open_in_new</span>
        </Link>
      </footer>
    </aside>
  );
}
