import { z } from "zod";

export const MAX_PAGE_SIZE = 200;
export const DEFAULT_PAGE_SIZE = 50;

/** Optional `limit` and opaque `cursor` inputs shared by list tools. */
export const paginationFields = {
  limit: z
    .number()
    .int()
    .min(1)
    .max(MAX_PAGE_SIZE)
    .optional()
    .describe(`Page size (max ${MAX_PAGE_SIZE}). Pass next_cursor from the previous page as cursor to continue.`),
  cursor: z.string().optional().describe("Opaque cursor from a previous page's next_cursor."),
};

export interface Page<T> {
  items: T[];
  total: number;
  next_cursor: string | null;
}

function encodeCursor(offset: number): string {
  return Buffer.from(JSON.stringify({ o: offset }), "utf8").toString("base64url");
}

function decodeCursor(cursor: string): number {
  try {
    const { o } = JSON.parse(Buffer.from(cursor, "base64url").toString("utf8")) as { o?: unknown };
    if (typeof o === "number" && Number.isInteger(o) && o >= 0) return o;
  } catch {
    // fall through to the error below
  }
  throw new Error("Invalid cursor: pass the next_cursor value of a previous page unchanged");
}

/**
 * Slices a list response in the MCP server (the backend still returns the whole
 * list). Tools that predate paging keep their full output until the caller sends
 * `limit` or `cursor`; new list tools pass `defaultLimit` to page from the start.
 * Anything that is not an array is returned as is.
 */
export function paginate(
  data: unknown,
  args: { limit?: number; cursor?: string },
  defaultLimit?: number
): unknown {
  if (!Array.isArray(data)) return data;
  if (args.limit == null && args.cursor == null && defaultLimit == null) return data;
  const limit = Math.min(args.limit ?? defaultLimit ?? DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE);
  const offset = args.cursor != null ? decodeCursor(args.cursor) : 0;
  const end = offset + limit;
  const page: Page<unknown> = {
    items: data.slice(offset, end),
    total: data.length,
    next_cursor: end < data.length ? encodeCursor(end) : null,
  };
  return page;
}
