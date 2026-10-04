/**
 * Unsaved requirement-editor drafts kept in this browser (issue #255).
 *
 * A draft never becomes a requirement version by itself: only Save does that.
 * Drafts are keyed by user, project and requirement (or "new"), expire after
 * {@link DRAFT_MAX_AGE_MS}, and are removed on sign-out. Every storage access
 * is guarded: private windows, blocked storage or a full quota simply mean
 * "no draft" on read and an error on write.
 */

const PREFIX = 'marreq-draft:v1:';

export const DRAFT_MAX_AGE_MS = 30 * 24 * 60 * 60 * 1000;

/**
 * Users who signed out in this page: an editor unmounting during sign-out
 * must not write its pending draft back after {@link clearUserDrafts}.
 */
const signedOut = new Set<number>();

function draftUserId(key: string): number | null {
  const match = new RegExp(`^${PREFIX}u(\\d+):`).exec(key);
  return match ? Number(match[1]) : null;
}

export type RequirementDraft<V> = {
  /** ISO time of the last write. */
  savedAt: string;
  /** The requirement version the draft was edited from (`null` for a new requirement). */
  baseVersionId: number | null;
  values: V;
};

export function requirementDraftKey(
  userId: number,
  projectId: number,
  requirementId: number | 'new',
): string {
  return `${PREFIX}u${userId}:p${projectId}:${requirementId}`;
}

function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function readDraft<V>(key: string, now = Date.now()): RequirementDraft<V> | null {
  const store = storage();
  if (!store) return null;
  try {
    const raw = store.getItem(key);
    if (!raw) return null;
    const draft = JSON.parse(raw) as RequirementDraft<V>;
    const savedAt = Date.parse(draft?.savedAt);
    if (!draft || typeof draft !== 'object' || !draft.values || Number.isNaN(savedAt)) return null;
    if (now - savedAt > DRAFT_MAX_AGE_MS) {
      store.removeItem(key);
      return null;
    }
    return draft;
  } catch {
    return null;
  }
}

/** Store a draft; returns false when the browser refused (quota, blocked storage). */
export function writeDraft<V>(key: string, draft: RequirementDraft<V>): boolean {
  const user = draftUserId(key);
  if (user != null && signedOut.has(user)) return true;
  const store = storage();
  if (!store) return false;
  try {
    store.setItem(key, JSON.stringify(draft));
    return true;
  } catch {
    return false;
  }
}

export function clearDraft(key: string): void {
  try {
    storage()?.removeItem(key);
  } catch {
    // Nothing to do: the draft expires on its own.
  }
}

/**
 * Remove every draft of one user (on sign-out, so a shared browser keeps
 * nothing) and refuse new ones until {@link allowUserDrafts}.
 */
export function clearUserDrafts(userId: number): void {
  signedOut.add(userId);
  const store = storage();
  if (!store) return;
  try {
    const mine = `${PREFIX}u${userId}:`;
    const keys: string[] = [];
    for (let i = 0; i < store.length; i += 1) {
      const key = store.key(i);
      if (key?.startsWith(mine)) keys.push(key);
    }
    for (const key of keys) store.removeItem(key);
  } catch {
    // Best effort.
  }
}

/** The user is signed in again: drafts may be kept. */
export function allowUserDrafts(userId: number): void {
  signedOut.delete(userId);
}

/** "12:03" today, or "3 Oct, 12:03" for an older draft. */
export function formatDraftTime(iso: string | Date, now = new Date()): string {
  const at = typeof iso === 'string' ? new Date(iso) : iso;
  const time = at.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  if (at.toDateString() === now.toDateString()) return time;
  return `${at.toLocaleDateString([], { day: 'numeric', month: 'short' })}, ${time}`;
}
