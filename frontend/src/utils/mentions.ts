import type { ProjectMember } from '@/api/types';

/**
 * `@username` mentions in comments. Mirrors `marreq-core/src/services/mentions.rs`:
 * a username is 3–50 of `[A-Za-z0-9_]`, and the `@` must start the text or follow a
 * character that is neither a letter, a digit, `_` nor `@` (so e-mail addresses are
 * not mentions).
 */
const MIN_LEN = 3;
const MAX_LEN = 50;
const MAX_SUGGESTIONS = 8;

const isWord = (c: string) => /[A-Za-z0-9_]/.test(c);
const isBoundary = (c: string | undefined) => c === undefined || !(/[\p{L}\p{N}_@]/u.test(c));

export type CommentPart = { kind: 'text'; text: string } | { kind: 'mention'; text: string; username: string };

/** Split a comment body into plain text and `@username` tokens. */
export function splitMentions(body: string): CommentPart[] {
  const parts: CommentPart[] = [];
  let textStart = 0;
  let i = 0;
  while (i < body.length) {
    if (body[i] === '@' && isBoundary(body[i - 1])) {
      let end = i + 1;
      while (end < body.length && isWord(body[end]!)) end++;
      const len = end - i - 1;
      const next = body[end];
      if (len >= MIN_LEN && len <= MAX_LEN && !(next !== undefined && /[\p{L}\p{N}]/u.test(next))) {
        if (i > textStart) parts.push({ kind: 'text', text: body.slice(textStart, i) });
        parts.push({ kind: 'mention', text: body.slice(i, end), username: body.slice(i + 1, end).toLowerCase() });
        textStart = end;
        i = end;
        continue;
      }
    }
    i++;
  }
  if (textStart < body.length) parts.push({ kind: 'text', text: body.slice(textStart) });
  return parts;
}

/** The `@query` being typed just before the caret, if any. */
export function activeMention(text: string, caret: number): { start: number; query: string } | null {
  let start = caret;
  while (start > 0 && isWord(text[start - 1]!)) start--;
  if (start === 0 || text[start - 1] !== '@') return null;
  const at = start - 1;
  if (!isBoundary(text[at - 1])) return null;
  const query = text.slice(start, caret);
  return query.length > MAX_LEN ? null : { start: at, query };
}

/** Members whose username or name matches `query`, username prefixes first. */
export function suggestMembers(members: ProjectMember[], query: string): ProjectMember[] {
  const q = query.toLowerCase();
  const rank = (m: ProjectMember) => {
    const username = m.username.toLowerCase();
    if (username.startsWith(q)) return 0;
    if (m.name.toLowerCase().split(/\s+/).some((w) => w.startsWith(q))) return 1;
    if (username.includes(q) || m.name.toLowerCase().includes(q)) return 2;
    return 3;
  };
  return members
    .map((m) => ({ m, r: rank(m) }))
    .filter(({ r }) => r < 3)
    .sort((a, b) => a.r - b.r || a.m.username.localeCompare(b.m.username))
    .slice(0, MAX_SUGGESTIONS)
    .map(({ m }) => m);
}
