/**
 * Session context loaded from environment (user_id, project_id, role, mode).
 * All API calls are project-scoped and use this context.
 *
 * - `read_only`: core read tools (requirements, trace, baselines, coverage).
 * - `read_extended`: core reads plus verifications, audit activity, comments, catalog, matrix read.
 * - `draft_write`: read_extended plus requirement/baseline writes (Phase 2).
 * - `traceWrite`: optional; matrix replace + clear suspect (independent flag, see MARREQ_TRACE_WRITE).
 */
export type MarreqMode = "read_only" | "read_extended" | "draft_write";

export interface SessionContext {
  baseUrl: string;
  apiToken: string;
  projectId: number;
  userId?: number;
  role?: number;
  sessionId?: string;
  mode: MarreqMode;
  /** When true, register `put_verification_matrix` and `clear_suspect` (requires API permissions). */
  traceWrite: boolean;
}

function parseMarreqMode(raw: string | undefined): MarreqMode {
  const m = (raw ?? "read_only").trim().toLowerCase();
  if (m === "read_only") return "read_only";
  if (m === "read_extended") return "read_extended";
  if (m === "draft_write") return "draft_write";
  throw new Error(
    "MARREQ_MODE must be read_only, read_extended, or draft_write"
  );
}

function parseTraceWriteFlag(): boolean {
  const v = process.env.MARREQ_TRACE_WRITE?.trim().toLowerCase();
  return v === "1" || v === "true" || v === "yes";
}

/** True when extended read tools (verifications, activity, catalog, …) should be registered. */
export function contextAllowsReadExtended(ctx: SessionContext): boolean {
  return ctx.mode === "read_extended" || ctx.mode === "draft_write";
}

export function loadContext(): SessionContext {
  const baseUrl = process.env.MARREQ_BASE_URL;
  const apiToken = process.env.MARREQ_API_TOKEN;
  const projectId = process.env.MARREQ_PROJECT_ID;

  if (!baseUrl || !apiToken || !projectId) {
    throw new Error(
      "MARREQ_BASE_URL, MARREQ_API_TOKEN, and MARREQ_PROJECT_ID must be set"
    );
  }

  const mode = parseMarreqMode(process.env.MARREQ_MODE);
  const traceWrite = parseTraceWriteFlag();

  return {
    baseUrl: baseUrl.replace(/\/$/, ""),
    apiToken,
    projectId: parseInt(projectId, 10),
    userId: process.env.MARREQ_USER_ID
      ? parseInt(process.env.MARREQ_USER_ID, 10)
      : undefined,
    role: process.env.MARREQ_ROLE
      ? parseInt(process.env.MARREQ_ROLE, 10)
      : undefined,
    sessionId: process.env.MARREQ_SESSION_ID,
    mode,
    traceWrite,
  };
}
