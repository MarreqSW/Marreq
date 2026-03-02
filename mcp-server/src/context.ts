/**
 * Session context loaded from environment (user_id, project_id, role, mode).
 * All API calls are project-scoped and use this context.
 */
export interface SessionContext {
  baseUrl: string;
  apiToken: string;
  projectId: number;
  userId?: number;
  role?: number;
  sessionId?: string;
  mode: "read_only" | "draft_write";
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

  const mode = (process.env.MARREQ_MODE ?? "read_only") as
    | "read_only"
    | "draft_write";
  if (mode !== "read_only" && mode !== "draft_write") {
    throw new Error("MARREQ_MODE must be read_only or draft_write");
  }

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
  };
}
