import { useEffect, useRef, useState } from 'react';
import { createUseCaseComment } from '@/api/client';
import type { ProjectMember, UseCaseComment } from '@/api/types';
import CommentBody from '@/components/CommentBody';
import MentionTextarea from '@/components/MentionTextarea';
import { formatRelative } from '@/utils/useCaseDocument';
import type { LinkableElement } from './LinkDialog';

type Props = {
  projectId: number;
  useCaseId: number;
  comments: UseCaseComment[];
  members: ProjectMember[];
  elements: LinkableElement[];
  /** The version new comments are attached to (the one shown). */
  versionId: number | null;
  canComment: boolean;
  /** Comments are locked on the shown (approved) version. */
  locked: boolean;
  csrfToken: string | null;
  /** Show only one element's thread, and comment on it. */
  focusElement: string | null;
  onFocusElement: (id: string | null) => void;
  onPosted: (comment: UseCaseComment) => void;
};

/** Comments on the use case and its steps, with @mentions (screen 8 of #388, issue #401). */
export default function UseCaseDiscussion({
  projectId,
  useCaseId,
  comments,
  members,
  elements,
  versionId,
  canComment,
  locked,
  csrfToken,
  focusElement,
  onFocusElement,
  onPosted,
}: Props) {
  const [body, setBody] = useState('');
  const [posting, setPosting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const ref = useRef<HTMLElement>(null);

  useEffect(() => {
    if (focusElement) ref.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }, [focusElement]);

  const labelOf = (id: string | null) => elements.find((e) => e.id === id)?.label ?? null;
  const shown = focusElement ? comments.filter((c) => c.element_id === focusElement) : comments;

  async function post() {
    const text = body.trim();
    if (!text || !csrfToken) return;
    setPosting(true);
    setError(null);
    try {
      const comment = await createUseCaseComment(
        projectId,
        useCaseId,
        { body: text, use_case_version_id: versionId, element_id: focusElement },
        csrfToken,
      );
      setBody('');
      onPosted(comment);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to post the comment');
    } finally {
      setPosting(false);
    }
  }

  return (
    <section ref={ref} id="discussion" className="mt-6 overflow-hidden rounded-lg border border-stitch-border bg-stitch-surface shadow-sm">
      <div className="flex flex-wrap items-center gap-2 border-b border-stitch-border bg-stitch-elevated/50 px-4 py-2.5">
        <span className="material-symbols-outlined text-lg text-stitch-accent">forum</span>
        <h3 className="font-headline text-sm font-bold text-stitch-accent">Discussion ({comments.length})</h3>
        {focusElement ? (
          <span className="ml-2 inline-flex items-center gap-1 rounded-full bg-stitch-accent/10 px-2 py-0.5 text-xs text-stitch-accent">
            On {labelOf(focusElement) ?? 'this element'}
            <button
              type="button"
              aria-label="Show all comments"
              onClick={() => onFocusElement(null)}
              className="rounded-full hover:text-stitch-fg"
            >
              <span className="material-symbols-outlined text-sm">close</span>
            </button>
          </span>
        ) : null}
      </div>
      <ul className="divide-y divide-stitch-border">
        {shown.length === 0 ? (
          <li className="px-4 py-4 text-sm text-stitch-muted">
            {focusElement ? 'No comments on this element yet.' : 'No comments yet.'}
          </li>
        ) : (
          shown.map((c) => (
            <li key={c.id} className="px-4 py-3">
              <div className="mb-1 flex flex-wrap items-center gap-2 text-xs">
                <span className="font-semibold text-stitch-fg">{c.author_name}</span>
                <span className="text-stitch-muted">{formatRelative(c.created_at)}</span>
                {c.element_id ? (
                  <button
                    type="button"
                    onClick={() => onFocusElement(c.element_id)}
                    className="rounded-sm bg-stitch-elevated px-1.5 py-0.5 font-mono text-[10px] font-semibold text-stitch-fg hover:text-stitch-accent"
                  >
                    {labelOf(c.element_id) ?? c.element_label ?? 'removed element'}
                  </button>
                ) : null}
              </div>
              <CommentBody body={c.body} members={members} className="whitespace-pre-wrap text-sm text-stitch-fg" />
            </li>
          ))
        )}
      </ul>
      {canComment ? (
        <div className="border-t border-stitch-border px-4 py-3">
          {locked ? (
            <p className="text-xs text-stitch-muted">Comments are locked on this approved version.</p>
          ) : (
            <>
              <label className="mb-2 flex items-center gap-2 text-xs text-stitch-muted">
                Comment on
                <select
                  value={focusElement ?? ''}
                  onChange={(e) => onFocusElement(e.target.value || null)}
                  className="rounded-md border border-stitch-border bg-stitch-elevated px-2 py-1 text-xs text-stitch-fg"
                >
                  <option value="">the whole use case</option>
                  {elements.map((el) => (
                    <option key={el.id} value={el.id}>
                      {el.label}
                    </option>
                  ))}
                </select>
              </label>
              {error ? (
                <p role="alert" className="mb-2 text-xs text-red-700 dark:text-red-300">
                  {error}
                </p>
              ) : null}
              <MentionTextarea
                className="min-h-[72px] w-full resize-y rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg outline-hidden focus:border-stitch-accent"
                placeholder={members.length > 0 ? 'Add a comment… (@ to mention)' : 'Add a comment…'}
                value={body}
                onChange={setBody}
                members={members}
                ariaLabel="Add a comment"
              />
              <button
                type="button"
                disabled={posting || !body.trim() || !csrfToken}
                onClick={() => void post()}
                className="mt-2 text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline disabled:opacity-40"
              >
                {posting ? 'Posting…' : 'Add comment'}
              </button>
            </>
          )}
        </div>
      ) : null}
    </section>
  );
}
