import type { SessionContext } from "./context.js";

export class MarreqClient {
  constructor(private ctx: SessionContext) {}

  private async request<T>(
    path: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.ctx.baseUrl}${path.startsWith("/") ? path : `/${path}`}`;
    const res = await fetch(url, {
      ...options,
      headers: {
        Authorization: `Bearer ${this.ctx.apiToken}`,
        "Content-Type": "application/json",
        ...options.headers,
      },
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Marreq API ${res.status}: ${text}`);
    }
    if (res.status === 204 || res.headers.get("content-length") === "0") {
      return undefined as T;
    }
    return res.json() as Promise<T>;
  }

  async getRequirement(id: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements/${id}`
    );
  }

  async listRequirements(approvalState?: string, hasTests?: boolean) {
    const params = new URLSearchParams();
    if (approvalState != null) params.set("approval_state", approvalState);
    if (hasTests != null) params.set("has_tests", String(hasTests));
    const q = params.toString();
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements${q ? `?${q}` : ""}`
    );
  }

  async getVersions(requirementId: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements/${requirementId}/versions`
    );
  }

  async compareVersions(requirementId: number, v1: number, v2: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements/${requirementId}/versions/${v1}/diff/${v2}`
    );
  }

  async traceUp(requirementId: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements/${requirementId}/trace_up`
    );
  }

  async traceDown(requirementId: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements/${requirementId}/trace_down`
    );
  }

  async coverageReport() {
    return this.request(
      `/api/projects/${this.ctx.projectId}/coverage_report`
    );
  }

  async getBaseline(baselineId: number) {
    const [meta, requirements, traceability] = await Promise.all([
      this.request(`/api/projects/${this.ctx.projectId}/baselines/${baselineId}`),
      this.request(
        `/api/projects/${this.ctx.projectId}/baselines/${baselineId}/requirements`
      ),
      this.request(
        `/api/projects/${this.ctx.projectId}/baselines/${baselineId}/traceability`
      ),
    ]);
    return { baseline: meta, requirements, traceability };
  }

  async diffBaselines(baselineA: number, baselineB: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/baselines/diff?baseline_a=${baselineA}&baseline_b=${baselineB}`
    );
  }

  /** Phase 2 draft_write: create requirement (project from context). */
  async createRequirement(payload: {
    title: string;
    description: string;
    reference_code: string;
    author_id: number;
    reviewer_id: number;
    category_id: number;
    status_id: number;
    applicability_id: number;
    project_id: number;
    justification?: string | null;
    verification_method_ids: number[];
    custom_fields?: Array<{ field_id: number; value: string }>;
  }) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements`,
      { method: "POST", body: JSON.stringify(payload) }
    );
  }

  /** Phase 2 draft_write: patch requirement (project from context). */
  async patchRequirement(
    requirementId: number,
    patch: {
      title?: string;
      description?: string;
      status_id?: number;
      verification_method_ids?: number[];
      author_id?: number;
      reviewer_id?: number;
      category_id?: number;
      applicability_id?: number;
      custom_fields?: Array<{ field_id: number; value: string }>;
    }
  ) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements/${requirementId}`,
      { method: "PATCH", body: JSON.stringify(patch) }
    );
  }

  /** Phase 2 draft_write: set requirement version approval (reviewed | approved). */
  async setApproval(
    requirementId: number,
    versionId: number,
    state: "reviewed" | "approved"
  ) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements/${requirementId}/versions/${versionId}/approval`,
      { method: "PUT", body: JSON.stringify({ state }) }
    );
  }

  /** Phase 2 draft_write: create baseline (project from context). */
  async createBaseline(payload: { name: string; description?: string | null }) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/baselines`,
      { method: "POST", body: JSON.stringify(payload) }
    );
  }

  async postAudit(payload: {
    project_id: number;
    session_id?: string;
    tool_name: string;
    params_summary?: string;
    result_summary?: string;
    is_write: boolean;
  }) {
    return this.request("/api/mcp/audit", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  /** GET /api/projects/:pid/verifications */
  async listVerificationsByProject() {
    return this.request(`/api/projects/${this.ctx.projectId}/verifications`);
  }

  /** GET /api/verifications/:id — caller should ensure the row belongs to MARREQ_PROJECT_ID. */
  async getVerificationById(verificationId: number) {
    return this.request(`/api/verifications/${verificationId}`);
  }

  /** GET /api/projects/:pid/baselines */
  async listBaselinesByProject() {
    return this.request(`/api/projects/${this.ctx.projectId}/baselines`);
  }

  async getRequirementActivity(requirementId: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements/${requirementId}/activity`
    );
  }

  async getVerificationActivity(verificationId: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/verifications/${verificationId}/activity`
    );
  }

  async listRequirementComments(
    requirementId: number,
    versionId?: number | null
  ) {
    const q =
      versionId != null && versionId > 0
        ? `?version_id=${encodeURIComponent(String(versionId))}`
        : "";
    return this.request(`/api/requirements/${requirementId}/comments${q}`);
  }

  async getVerificationMatrix(verificationId: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/verifications/${verificationId}/matrix`
    );
  }

  async putVerificationMatrix(
    verificationId: number,
    requirementIds: number[]
  ) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/verifications/${verificationId}/matrix`,
      {
        method: "PUT",
        body: JSON.stringify({ requirement_ids: requirementIds }),
      }
    );
  }

  async clearSuspectLink(reqId: number, verificationId: number) {
    return this.request("/api/traceability/clear_suspect", {
      method: "POST",
      body: JSON.stringify({
        req_id: reqId,
        verification_id: verificationId,
      }),
    });
  }

  async diffBaselineVsCurrent(baselineId: number, requirementId: number) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/baselines/${baselineId}/requirements/${requirementId}/diff/current`
    );
  }

  /** Aggregated catalog rows for MARREQ_PROJECT_ID (parallel GETs, filtered client-side where needed). */
  async listProjectCatalog() {
    const pid = this.ctx.projectId;
    const [categories, applicability, reqStatuses, verifStatuses, methods, fields] =
      await Promise.all([
        this.request<unknown[]>("/api/categories"),
        this.request<unknown[]>("/api/applicability"),
        this.request<unknown[]>("/api/status"),
        this.request<unknown[]>("/api/verification-status"),
        this.request<unknown[]>(
          `/api/projects/${pid}/verification-methods`
        ),
        this.request<unknown[]>(`/api/projects/${pid}/custom_fields`),
      ]);

    const byProject = (rows: unknown[]) =>
      Array.isArray(rows)
        ? rows.filter(
            (r) =>
              r &&
              typeof r === "object" &&
              "project_id" in r &&
              (r as { project_id: number }).project_id === pid
          )
        : [];

    return {
      categories: byProject(categories),
      applicability: byProject(applicability),
      requirement_statuses: byProject(reqStatuses),
      verification_statuses: byProject(verifStatuses),
      verification_methods: Array.isArray(methods) ? methods : [],
      custom_fields: Array.isArray(fields) ? fields : [],
    };
  }

  async createRequirementComment(
    requirementId: number,
    body: string,
    requirementVersionId?: number | null
  ) {
    return this.request(`/api/requirements/${requirementId}/comments`, {
      method: "POST",
      body: JSON.stringify({
        body,
        requirement_version_id:
          requirementVersionId != null && requirementVersionId > 0
            ? requirementVersionId
            : null,
      }),
    });
  }

  get projectId() {
    return this.ctx.projectId;
  }

  get sessionId() {
    return this.ctx.sessionId;
  }
}
