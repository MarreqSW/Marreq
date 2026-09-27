/** Text transforms behind the statement editor toolbar (pure, so they are easy to test). */

export type EditResult = { value: string; selectionStart: number; selectionEnd: number };

const PLACEHOLDER: Record<string, string> = {
  '**': 'bold text',
  '*': 'italic text',
  '`': 'code',
};

/**
 * Wrap the selection in `marker` (`**`, `*` or `` ` ``), or unwrap it when it is
 * already wrapped. With no selection, inserts a selected placeholder.
 */
export function toggleWrap(value: string, start: number, end: number, marker: string): EditResult {
  const m = marker.length;
  const before = value.slice(0, start);
  const selected = value.slice(start, end);
  const after = value.slice(end);

  // `**x**` with only `x` selected, or the selection itself includes the markers.
  // A single `*` must not be mistaken for half of a `**` bold marker.
  const endsWithMarker = (t: string) =>
    marker === '*' ? /(^|[^*])\*$|\*\*\*$/.test(t) : t.endsWith(marker);
  const startsWithMarker = (t: string) =>
    marker === '*' ? /^\*($|[^*])|^\*\*\*/.test(t) : t.startsWith(marker);
  const wrappedOutside = endsWithMarker(before) && startsWithMarker(after);
  if (selected && wrappedOutside) {
    return {
      value: before.slice(0, -m) + selected + after.slice(m),
      selectionStart: start - m,
      selectionEnd: end - m,
    };
  }
  if (selected.length > 2 * m && selected.startsWith(marker) && selected.endsWith(marker)) {
    const inner = selected.slice(m, -m);
    return { value: before + inner + after, selectionStart: start, selectionEnd: start + inner.length };
  }

  const text = selected || PLACEHOLDER[marker] || 'text';
  return {
    value: before + marker + text + marker + after,
    selectionStart: start + m,
    selectionEnd: start + m + text.length,
  };
}

const BULLET = /^( {0,3})[-*] /;
const NUMBERED = /^( {0,3})\d{1,9}[.)] /;

/**
 * Toggle a bulleted (`- `) or numbered (`1. `, `2. `, …) list on every line touched
 * by the selection. Lines already in that list style are turned back into text.
 */
export function toggleList(
  value: string,
  start: number,
  end: number,
  kind: 'bullets' | 'numbered',
): EditResult {
  const lineStart = value.lastIndexOf('\n', start - 1) + 1;
  const endNl = value.indexOf('\n', end > start && value[end - 1] === '\n' ? end - 1 : end);
  const lineEnd = endNl === -1 ? value.length : endNl;
  const lines = value.slice(lineStart, lineEnd).split('\n');
  const own = kind === 'bullets' ? BULLET : NUMBERED;
  const other = kind === 'bullets' ? NUMBERED : BULLET;

  const allListed = lines.every((l) => l.trim() === '' || own.test(l));
  let n = 0;
  const next = lines.map((l) => {
    if (l.trim() === '') return l;
    const bare = l.replace(own, '').replace(other, '');
    if (allListed) return bare;
    n += 1;
    return (kind === 'bullets' ? '- ' : `${n}. `) + bare;
  });
  const replaced = next.join('\n');
  return {
    value: value.slice(0, lineStart) + replaced + value.slice(lineEnd),
    selectionStart: lineStart,
    selectionEnd: lineStart + replaced.length,
  };
}

/** Insert `[label](https://)` using the selection as label, and select the URL part. */
export function insertLink(value: string, start: number, end: number): EditResult {
  const label = value.slice(start, end) || 'link text';
  const url = 'https://';
  const inserted = `[${label}](${url})`;
  const urlStart = start + label.length + 3;
  return {
    value: value.slice(0, start) + inserted + value.slice(end),
    selectionStart: urlStart,
    selectionEnd: urlStart + url.length,
  };
}

/** UTF-8 byte length (the backend limit of 2000 counts bytes, markup included). */
export function utf8Length(value: string): number {
  return new TextEncoder().encode(value).length;
}
