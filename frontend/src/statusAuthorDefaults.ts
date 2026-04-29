/**
 * Mirrors `author_default_requirement_status_id` / `initial_verification_status_id`
 * (see `marreq-core/src/api/prelude.rs`): non-reviewers may create only with these statuses.
 */
export function authorDefaultRequirementStatusId(statuses: { id: number; tag: string }[]): number | null {
  if (statuses.length === 0) return null;
  const draft = statuses.find((s) => s.tag.toLowerCase() === 'draft');
  if (draft) return draft.id;
  return Math.min(...statuses.map((s) => s.id));
}

export function initialVerificationStatusIdForAuthor(statuses: { id: number; tag: string }[]): number | null {
  if (statuses.length === 0) return null;
  const nr = statuses.find((s) => s.tag.toLowerCase() === 'nr');
  if (nr) return nr.id;
  return Math.min(...statuses.map((s) => s.id));
}
