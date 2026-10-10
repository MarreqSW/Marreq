import { useState } from 'react';
import { createRequirementComment } from '@/api/client';
import type { ProjectMember, RequirementCommentItem } from '@/api/types';
import MentionTextarea from '@/components/MentionTextarea';

const textareaClass =
  'w-full min-h-[72px] text-sm font-medium resize-y bg-stitch-elevated border border-stitch-border rounded-lg px-3 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/30 outline-hidden transition-colors';

export function commentsLockedForApproval(approvalState: string | null | undefined): boolean {
  return (approvalState ?? '').toLowerCase() === 'approved';
}

export default function RequirementCommentComposer({
  requirementId,
  versionId,
  csrfToken,
  locked = false,
  members = [],
  onPosted,
}: {
  requirementId: number;
  versionId: number | null;
  csrfToken: string | null | undefined;
  locked?: boolean;
  /** Project members suggested after `@`. */
  members?: ProjectMember[];
  onPosted: (comment: RequirementCommentItem) => void;
}) {
  const [body, setBody] = useState('');
  const [posting, setPosting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (locked) {
    return (
      <p className="text-xs text-stitch-muted">
        Comments are locked on this approved version.
      </p>
    );
  }

  async function postComment() {
    const token = csrfToken ?? '';
    const trimmed = body.trim();
    if (!token || !trimmed) return;
    setPosting(true);
    setError(null);
    try {
      const comment = await createRequirementComment(
        requirementId,
        {
          body: trimmed,
          requirement_version_id: versionId,
        },
        token,
      );
      onPosted(comment);
      setBody('');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to post comment');
    } finally {
      setPosting(false);
    }
  }

  return (
    <div>
      {error ? (
        <p role="alert" className="text-xs text-red-200 mb-2">
          {error}
        </p>
      ) : null}
      <MentionTextarea
        className={textareaClass}
        placeholder={members.length > 0 ? 'Add a comment… (@ to mention)' : 'Add a comment…'}
        value={body}
        onChange={setBody}
        members={members}
        ariaLabel="Add a comment"
      />
      <button
        type="button"
        disabled={posting || !body.trim() || !csrfToken}
        onClick={() => void postComment()}
        className="mt-2 text-stitch-accent text-[10px] font-bold uppercase tracking-wider hover:underline disabled:opacity-40"
      >
        {posting ? 'Posting…' : 'Add comment'}
      </button>
    </div>
  );
}
