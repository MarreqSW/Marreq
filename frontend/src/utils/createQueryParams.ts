/** Parse a positive integer query parameter. Returns null if missing or invalid. */
export function parsePositiveQueryId(raw: string | null): number | null {
  if (raw == null || raw.trim() === '') return null;
  const n = Number(raw);
  if (!Number.isInteger(n) || n <= 0) return null;
  return n;
}

/** Duplicate source id: `from` wins over classic `template`. */
export function duplicateSourceQueryId(searchParams: URLSearchParams): number | null {
  return (
    parsePositiveQueryId(searchParams.get('from')) ??
    parsePositiveQueryId(searchParams.get('template'))
  );
}
