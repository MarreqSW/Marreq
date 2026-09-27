/**
 * "Marreq statement Markdown": the Markdown subset used for requirement statements
 * (issue #256). Mirrors `marreq-core/src/rich_text.rs`; keep both in sync.
 *
 * Blocks: paragraphs (blank line between them; a single newline is a line break),
 * bulleted lists (`- ` / `* `), numbered lists (`1. `; the first number sets the start).
 * Inline: `**bold**`, `*italic*` / `_italic_`, `` `code` ``, `[label](url)` with
 * `http:`, `https:` or `mailto:` URLs only. `\` escapes a marker character.
 */

export type Inline =
  | { kind: 'text'; text: string }
  | { kind: 'strong'; children: Inline[] }
  | { kind: 'em'; children: Inline[] }
  | { kind: 'code'; text: string }
  | { kind: 'link'; href: string; children: Inline[] }
  | { kind: 'br' };

export type Block =
  | { kind: 'paragraph'; inlines: Inline[] }
  | { kind: 'bullets'; items: Inline[][] }
  | { kind: 'numbered'; start: number; items: Inline[][] };

const ESCAPABLE = new Set(['\\', '*', '_', '`', '[', ']', '(', ')', '-', '.', '#', '!']);

/** True for link targets that are safe to render (`http:`, `https:`, `mailto:`). */
export function isSafeHref(href: string): boolean {
  if (/[\s\u0000-\u001f\u007f]/.test(href)) return false;
  const h = href.trim().toLowerCase();
  return h.startsWith('http://') || h.startsWith('https://') || h.startsWith('mailto:');
}

function bulletItem(line: string): string | null {
  const m = /^( {0,3})[-*] (.*)$/.exec(line);
  return m ? m[2] : null;
}

function numberedItem(line: string): { n: number; rest: string } | null {
  const m = /^( {0,3})(\d{1,9})[.)] (.*)$/.exec(line);
  return m ? { n: Number(m[2]), rest: m[3] } : null;
}

/** Parse statement Markdown into blocks. */
export function parseStatement(src: string): Block[] {
  const lines = src.replace(/\r\n/g, '\n').split('\n');
  const blocks: Block[] = [];
  let para: string[] = [];
  let list: { numbered: boolean; start: number; items: Inline[][] } | null = null;

  const flushPara = () => {
    if (para.length) {
      blocks.push({ kind: 'paragraph', inlines: parseInline(para.join('\n')) });
      para = [];
    }
  };
  const flushList = () => {
    if (list) {
      blocks.push(
        list.numbered
          ? { kind: 'numbered', start: list.start, items: list.items }
          : { kind: 'bullets', items: list.items },
      );
      list = null;
    }
  };

  for (const line of lines) {
    const bullet = bulletItem(line);
    const numbered = bullet === null ? numberedItem(line) : null;
    if (line.trim() === '') {
      flushPara();
      flushList();
    } else if (bullet !== null) {
      flushPara();
      if (list?.numbered) flushList();
      list ??= { numbered: false, start: 1, items: [] };
      list.items.push(parseInline(bullet));
    } else if (numbered) {
      flushPara();
      if (list && !list.numbered) flushList();
      list ??= { numbered: true, start: numbered.n, items: [] };
      list.items.push(parseInline(numbered.rest));
    } else {
      flushList();
      para.push(line);
    }
  }
  flushPara();
  flushList();
  return blocks;
}

function pushText(out: Inline[], s: string) {
  if (!s) return;
  const last = out[out.length - 1];
  if (last?.kind === 'text') last.text += s;
  else out.push({ kind: 'text', text: s });
}

/** Index of `delim` in `chars` at or after `from`, skipping escapes; -1 if absent. */
function findClosing(chars: string[], from: number, delim: string): number {
  const d = [...delim];
  for (let i = from; i + d.length <= chars.length; ) {
    if (chars[i] === '\\') {
      i += 2;
      continue;
    }
    if (d.every((c, k) => chars[i + k] === c)) return i;
    i += 1;
  }
  return -1;
}

const isWord = (c: string | undefined) => c !== undefined && /[\p{L}\p{N}]/u.test(c);

/** Parse inline markup (a paragraph or list item; `\n` becomes a line break). */
export function parseInline(src: string): Inline[] {
  const chars = [...src];
  const out: Inline[] = [];
  let i = 0;
  while (i < chars.length) {
    const c = chars[i];
    if (c === '\\' && ESCAPABLE.has(chars[i + 1])) {
      pushText(out, chars[i + 1]);
      i += 2;
      continue;
    }
    if (c === '\n') {
      out.push({ kind: 'br' });
      i += 1;
      continue;
    }
    if (c === '`') {
      const end = chars.indexOf('`', i + 1);
      if (end > i + 1) {
        out.push({ kind: 'code', text: chars.slice(i + 1, end).join('') });
        i = end + 1;
        continue;
      }
    }
    if (c === '*' && chars[i + 1] === '*') {
      const end = findClosing(chars, i + 2, '**');
      if (end > i + 2) {
        out.push({ kind: 'strong', children: parseInline(chars.slice(i + 2, end).join('')) });
        i = end + 2;
        continue;
      }
    } else if (c === '*' || c === '_') {
      const next = chars[i + 1];
      const opens =
        next !== undefined &&
        !/\s/.test(next) &&
        next !== c &&
        (c === '*' || !isWord(chars[i - 1]));
      if (opens) {
        let j = i + 1;
        let close = -1;
        for (;;) {
          const end = findClosing(chars, j, c);
          if (end < 0) break;
          const beforeWs = /\s/.test(chars[end - 1]);
          const doubled = chars[end + 1] === c;
          const wordAfter = c === '_' && isWord(chars[end + 1]);
          if (!beforeWs && !doubled && !wordAfter) {
            close = end;
            break;
          }
          j = end + (doubled ? 2 : 1);
        }
        if (close >= 0) {
          out.push({ kind: 'em', children: parseInline(chars.slice(i + 1, close).join('')) });
          i = close + 1;
          continue;
        }
      }
    } else if (c === '[') {
      const close = findClosing(chars, i + 1, ']');
      if (close >= 0 && chars[close + 1] === '(') {
        const paren = chars.indexOf(')', close + 2);
        if (paren >= 0) {
          const href = chars.slice(close + 2, paren).join('');
          const end = paren + 1;
          if (close > i + 1 && isSafeHref(href)) {
            out.push({
              kind: 'link',
              href: href.trim(),
              children: parseInline(chars.slice(i + 1, close).join('')),
            });
          } else {
            // Unsafe or empty link: keep the source visible as text.
            pushText(out, chars.slice(i, end).join(''));
          }
          i = end;
          continue;
        }
      }
    }
    pushText(out, c);
    i += 1;
  }
  return out;
}

function inlinePlain(nodes: Inline[]): string {
  return nodes
    .map((n) => {
      switch (n.kind) {
        case 'text':
        case 'code':
          return n.text;
        case 'br':
          return '\n';
        default:
          return inlinePlain(n.children);
      }
    })
    .join('');
}

/**
 * Statement text without markup (one-line previews, search). List items keep their
 * own lines, prefixed with `•` or their number.
 */
export function statementToPlainText(src: string): string {
  return parseStatement(src)
    .map((b) => {
      if (b.kind === 'paragraph') return inlinePlain(b.inlines);
      if (b.kind === 'bullets') return b.items.map((it) => `• ${inlinePlain(it)}`).join('\n');
      return b.items.map((it, k) => `${b.start + k}. ${inlinePlain(it)}`).join('\n');
    })
    .join('\n');
}
