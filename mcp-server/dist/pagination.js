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
function encodeCursor(offset) {
    return Buffer.from(JSON.stringify({ o: offset }), "utf8").toString("base64url");
}
function decodeCursor(cursor) {
    try {
        const { o } = JSON.parse(Buffer.from(cursor, "base64url").toString("utf8"));
        if (typeof o === "number" && Number.isInteger(o) && o >= 0)
            return o;
    }
    catch {
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
export function paginate(data, args, defaultLimit) {
    if (!Array.isArray(data))
        return data;
    if (args.limit == null && args.cursor == null && defaultLimit == null)
        return data;
    const limit = Math.min(args.limit ?? defaultLimit ?? DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE);
    const offset = args.cursor != null ? decodeCursor(args.cursor) : 0;
    const end = offset + limit;
    const page = {
        items: data.slice(offset, end),
        total: data.length,
        next_cursor: end < data.length ? encodeCursor(end) : null,
    };
    return page;
}
