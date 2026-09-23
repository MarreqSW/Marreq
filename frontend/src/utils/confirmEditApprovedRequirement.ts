export const EDIT_APPROVED_CONFIRM_MESSAGE =
  "This requirement's current version is Approved. Editing will create a new Draft version. The approved version remains in history. Continue?";

const ACK_PREFIX = 'marreq-edit-approved-ack:';

/** Survives React StrictMode remounts within the same SPA session. */
const promptedRequirementIds = new Set<number>();

export function resetApprovedEditPromptsForTests(): void {
  promptedRequirementIds.clear();
}

export function markApprovedEditPrompted(requirementId: number): void {
  promptedRequirementIds.add(requirementId);
}

export function hasApprovedEditPrompted(requirementId: number): boolean {
  return promptedRequirementIds.has(requirementId);
}

export function isApprovedRequirement(approvalState: string | null | undefined): boolean {
  return (approvalState ?? '').toLowerCase() === 'approved';
}

function ackKey(requirementId: number): string {
  return `${ACK_PREFIX}${requirementId}`;
}

export function setEditApprovedAck(requirementId: number): void {
  try {
    sessionStorage.setItem(ackKey(requirementId), '1');
  } catch {
    // Ignore quota / private-mode failures; the edit page may prompt again.
  }
}

/** Returns true if a prior confirm already covered this requirement, then clears the flag. */
export function consumeEditApprovedAck(requirementId: number): boolean {
  try {
    const key = ackKey(requirementId);
    const had = sessionStorage.getItem(key) === '1';
    sessionStorage.removeItem(key);
    return had;
  } catch {
    return false;
  }
}

export function confirmEditApprovedRequirement(
  approvalState: string | null | undefined,
  requirementId?: number,
): boolean {
  if (!isApprovedRequirement(approvalState)) return true;
  const ok = window.confirm(EDIT_APPROVED_CONFIRM_MESSAGE);
  if (ok && requirementId != null) {
    setEditApprovedAck(requirementId);
  }
  return ok;
}

type EditClickEvent = {
  preventDefault: () => void;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  button: number;
};

/** Intercept left-click on Edit; modified-clicks (new tab) skip the prompt. */
export function preventEditNavigationIfUnconfirmed(
  event: EditClickEvent,
  approvalState: string | null | undefined,
  requirementId: number,
): void {
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || event.button !== 0) {
    return;
  }
  if (!confirmEditApprovedRequirement(approvalState, requirementId)) {
    event.preventDefault();
  }
}
