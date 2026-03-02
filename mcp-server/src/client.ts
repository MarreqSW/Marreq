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

  get projectId() {
    return this.ctx.projectId;
  }

  get sessionId() {
    return this.ctx.sessionId;
  }
}
