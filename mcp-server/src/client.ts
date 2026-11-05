import type { components } from "./generated-contract.js";
type McpPrincipalResponse = components["schemas"]["McpPrincipalResponse"];
type UseCaseRow = components["schemas"]["use_cases_UseCaseRow_response"];
type UseCaseDetail = components["schemas"]["use_cases_UseCaseDetail_response"];
type UseCaseDraft = components["schemas"]["use_cases_DraftView_response"];
type UseCaseDraftChange = components["schemas"]["use_cases_DraftChangeView_response"];
import type { SessionContext } from "./context.js";
import { currentBearer } from "./credentials.js";

export class MarreqAuthenticationError extends Error {
  constructor(public readonly status: number, public readonly challenge?: string) {
    super("Marreq authorization is required");
  }
}

export class MarreqClient {
  constructor(private ctx: SessionContext) {}

  withProject(projectId: number): MarreqClient {
    return new MarreqClient({ ...this.ctx, projectId });
  }

  async listProjects() {
    return this.request("/api/projects");
  }

  async getPrincipal(): Promise<McpPrincipalResponse> {
    return this.request("/api/mcp/principal");
  }

  private async request<T>(
    path: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.ctx.baseUrl}${path.startsWith("/") ? path : `/${path}`}`;
    const res = await fetch(url, {
      ...options,
      headers: {
        Authorization: `Bearer ${this.ctx.remote ? currentBearer() ?? this.ctx.apiToken : this.ctx.apiToken}`,
        "Content-Type": "application/json",
        ...options.headers,
      },
    });
    if (!res.ok) {
      const text = await res.text();
      if (res.status === 401 || (res.status === 403 && res.headers.has("www-authenticate"))) {
        throw new MarreqAuthenticationError(res.status, res.headers.get("www-authenticate") ?? undefined);
      }
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

  async listRequirements(
    approvalState?: string,
    hasTests?: boolean,
    extra: {
      status_id?: number;
      category_id?: number;
      q?: string;
      sort_column?: string;
      sort_dir?: "asc" | "desc";
      view_id?: number;
    } = {}
  ) {
    const params = new URLSearchParams();
    if (approvalState != null) params.set("approval_state", approvalState);
    if (hasTests != null) params.set("has_tests", String(hasTests));
    for (const [key, value] of Object.entries(extra)) {
      if (value != null && value !== "") params.set(key, String(value));
    }
    const q = params.toString();
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements${q ? `?${q}` : ""}`
    );
  }

  async semanticSearchRequirements(query: string, limit?: number) {
    const params = new URLSearchParams({ q: query });
    if (limit != null) params.set("k", String(limit));
    return this.request(`/api/projects/${this.ctx.projectId}/requirements/semantic_search?${params}`);
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

  async dependencyMatrix(
    options: { linkTypes?: string[]; order?: string; categoryId?: number; rootId?: number } = {}
  ) {
    const params = new URLSearchParams();
    if (options.linkTypes != null) params.set("link_types", options.linkTypes.join(","));
    if (options.order != null) params.set("order", options.order);
    if (options.categoryId != null) params.set("category_id", String(options.categoryId));
    if (options.rootId != null) params.set("root_id", String(options.rootId));
    const q = params.toString();
    return this.request(`/api/projects/${this.ctx.projectId}/dsm${q ? `?${q}` : ""}`);
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
    project_id?: number;
    justification?: string | null;
    verification_method_ids: number[];
    custom_fields?: Array<{ field_id: number; value: string }>;
    parent_links?: Array<{ target_version_id: number; link_type: string; rationale?: string | null }>;
  }, idempotencyKey: string) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/requirements`,
      { method: "POST", headers: { "Idempotency-Key": idempotencyKey }, body: JSON.stringify(payload) }
    );
  }

  async createVerification(payload: {
    reference_code: string; name: string; description: string; source: string;
    status_id: number; parent_id?: number | null; project_id: number;
    verification_method_id?: number | null; author_id: number; reviewer_id: number;
  }, idempotencyKey: string) {
    return this.request(`/api/projects/${this.ctx.projectId}/verifications`, {
      method: "POST", headers: { "Idempotency-Key": idempotencyKey }, body: JSON.stringify(payload),
    });
  }

  async updateVerification(id: number, payload: {
    id?: number; reference_code: string; name: string; description: string; source: string;
    status_id: number; parent_id?: number | null; project_id: number;
    verification_method_id?: number | null; author_id: number; reviewer_id: number;
  }) {
    return this.request(`/api/projects/${this.ctx.projectId}/verifications/${id}`, {
      method: "PUT", body: JSON.stringify(payload),
    });
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
      justification?: string;
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
  async createBaseline(payload: { name: string; description?: string | null }, idempotencyKey: string) {
    return this.request(
      `/api/projects/${this.ctx.projectId}/baselines`,
      { method: "POST", headers: { "Idempotency-Key": idempotencyKey }, body: JSON.stringify(payload) }
    );
  }

  async postAudit(payload: {
    project_id?: number;
    session_id?: string;
    tool_name: string;
    params_summary?: string;
    result_summary?: string;
    is_write: boolean;
  }) {
    // Remote MCP shares MARREQ_MCP_AUDIT_SECRET with Rocket and posts to the
    // internal path. Local stdio uses a personal API token on the legacy path
    // so operators do not need the server-internal secret.
    const auditSecret = process.env.MARREQ_MCP_AUDIT_SECRET;
    const path = auditSecret ? "/api/mcp/internal/audit" : "/api/mcp/audit";
    return this.request(path, {
      method: "POST",
      headers: auditSecret
        ? { "X-Marreq-MCP-Audit-Secret": auditSecret }
        : {},
      body: JSON.stringify(payload),
    });
  }

  /** GET /api/projects/:pid/verifications */
  async listVerificationsByProject() {
    return this.request(`/api/projects/${this.ctx.projectId}/verifications`);
  }

  /** GET /api/verifications/:id — caller should ensure the row belongs to MARREQ_PROJECT_ID. */
  async getVerificationById(verificationId: number) {
    return this.request(`/api/projects/${this.ctx.projectId}/verifications/${verificationId}`);
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
    return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/comments${q}`);
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
    return this.request(`/api/projects/${this.ctx.projectId}/traceability/clear_suspect`, {
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
    return this.request(`/api/projects/${this.ctx.projectId}/catalog`);
  }

  async createRequirementComment(
    requirementId: number,
    body: string,
    requirementVersionId: number | null | undefined,
    idempotencyKey: string
  ) {
    return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/comments`, {
      method: "POST",
      headers: { "Idempotency-Key": idempotencyKey },
      body: JSON.stringify({
        body,
        requirement_version_id:
          requirementVersionId != null && requirementVersionId > 0
            ? requirementVersionId
            : null,
      }),
    });
  }

  // ----- Use cases (issue #402) -----

  private useCases(path = "") {
    return `/api/projects/${this.ctx.projectId}/use-cases${path}`;
  }

  async listUseCases(query: { q?: string; approval_state?: string; has_suspect?: boolean } = {}): Promise<UseCaseRow[]> {
    const params = new URLSearchParams();
    if (query.q) params.set("q", query.q);
    if (query.approval_state) params.set("approval_state", query.approval_state);
    if (query.has_suspect) params.set("has_suspect", "true");
    const q = params.toString();
    return this.request(this.useCases(q ? `?${q}` : ""));
  }

  async getUseCase(id: number, versionId?: number): Promise<UseCaseDetail> {
    return this.request(this.useCases(`/${id}${versionId != null ? `?version_id=${versionId}` : ""}`));
  }

  /** The shared draft, or null when there is none. */
  async getUseCaseDraft(id: number): Promise<UseCaseDraft | null> {
    try {
      return await this.request<UseCaseDraft>(this.useCases(`/${id}/draft`));
    } catch (e) {
      if (e instanceof Error && e.message.startsWith("Marreq API 404")) return null;
      throw e;
    }
  }

  /** Without versions: the draft against its base. With both: version `from` against `to`. */
  async getUseCaseDiff(id: number, fromVersionId?: number, toVersionId?: number) {
    if (fromVersionId != null && toVersionId != null) {
      return this.request(this.useCases(`/${id}/versions/${fromVersionId}/diff/${toVersionId}`));
    }
    return this.request(this.useCases(`/${id}/draft/diff`));
  }

  async listUseCaseLinks(id: number) {
    return this.request(this.useCases(`/${id}/links`));
  }

  async createUseCase(
    payload: { reference_code?: string; title?: string; primary_actor?: string; document?: unknown },
    idempotencyKey: string
  ) {
    return this.request<UseCaseDetail>(this.useCases(), {
      method: "POST",
      headers: { "Idempotency-Key": idempotencyKey },
      body: JSON.stringify(payload),
    });
  }

  async applyUseCaseOperations(id: number, baseRevision: number | null, operations: unknown[]) {
    return this.request(this.useCases(`/${id}/draft/operations`), {
      method: "POST",
      body: JSON.stringify({ base_revision: baseRevision, operations }),
    });
  }

  async putUseCaseDraft(id: number, baseRevision: number | null, document: unknown): Promise<UseCaseDraftChange> {
    return this.request(this.useCases(`/${id}/draft`), {
      method: "PUT",
      body: JSON.stringify({ base_revision: baseRevision, document }),
    });
  }

  async publishUseCaseDraft(
    id: number,
    baseRevision: number,
    changeNote: string | null,
    idempotencyKey: string
  ) {
    return this.request(this.useCases(`/${id}/draft/publish`), {
      method: "POST",
      headers: { "Idempotency-Key": idempotencyKey },
      body: JSON.stringify({ base_revision: baseRevision, change_note: changeNote }),
    });
  }

  async linkUseCaseElement(
    id: number,
    payload: {
      element_id?: string | null;
      requirement_id?: number;
      verification_id?: number;
      rationale?: string | null;
    }
  ) {
    return this.request(this.useCases(`/${id}/links`), {
      method: "POST",
      body: JSON.stringify(payload),
    });
  }

  // ----- Tool gaps (issue #416) -----

  private project(path: string) {
    return `/api/projects/${this.ctx.projectId}${path}`;
  }

  private withQuery(path: string, query: Record<string, string | number | boolean | undefined | null>) {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      if (value != null && value !== "") params.set(key, String(value));
    }
    const q = params.toString();
    return q ? `${path}?${q}` : path;
  }

  async listRequirementLinkTypes() {
    return this.request(this.project("/requirement-version-links/link-types"));
  }

  async listRequirementLinks(query: { source_version_id?: number; target_version_id?: number; link_type?: string } = {}) {
    return this.request(this.withQuery(this.project("/requirement-version-links"), query));
  }

  async createRequirementLink(
    payload: {
      source_version_id: number;
      target_version_id: number;
      link_type: string;
      rationale?: string | null;
    },
    idempotencyKey: string
  ) {
    return this.request(this.project("/requirement-version-links"), {
      method: "POST",
      headers: { "Idempotency-Key": idempotencyKey },
      body: JSON.stringify(payload),
    });
  }

  async deleteRequirementLink(linkId: number) {
    return this.request(this.project(`/requirement-version-links/${linkId}`), { method: "DELETE" });
  }

  async deleteUseCaseLink(useCaseId: number, linkId: number) {
    return this.request(this.useCases(`/${useCaseId}/links/${linkId}`), { method: "DELETE" });
  }

  async clearUseCaseLinkSuspect(useCaseId: number, linkId: number) {
    return this.request(this.useCases(`/${useCaseId}/links/${linkId}/clear_suspect`), { method: "POST" });
  }

  async listLinkedUseCases(kind: "requirement" | "verification", id: number) {
    return this.request(this.project(`/${kind}s/${id}/use-cases`));
  }

  async askRequirements(payload: {
    query: string;
    k?: number;
    status_filter?: number;
    category_filter?: number;
    applicability_filter?: number;
    verification_filter?: number;
  }) {
    return this.request(this.project("/requirements/ask"), { method: "POST", body: JSON.stringify(payload) });
  }

  async listReviewers() {
    return this.request(this.project("/reviewers"));
  }

  async getMyPermissions() {
    return this.request(this.project("/me/permissions"));
  }

  async listSavedViews() {
    return this.request(this.project("/saved_views"));
  }

  async listAttachments(entityType: "requirement" | "verification", entityId: number) {
    return this.request(
      this.withQuery(this.project("/attachments"), { entity_type: entityType, entity_id: entityId })
    );
  }

  /** What needs the user's attention across projects (not project-scoped). */
  async getAttention() {
    return this.request("/api/home/attention");
  }

  async listUseCaseVersions(useCaseId: number) {
    return this.request(this.useCases(`/${useCaseId}/versions`));
  }

  async listUseCaseComments(useCaseId: number, query: { version_id?: number; element_id?: string } = {}) {
    return this.request(this.withQuery(this.useCases(`/${useCaseId}/comments`), query));
  }

  async createUseCaseComment(
    useCaseId: number,
    payload: { body: string; use_case_version_id?: number | null; element_id?: string | null },
    idempotencyKey: string
  ) {
    return this.request(this.useCases(`/${useCaseId}/comments`), {
      method: "POST",
      headers: { "Idempotency-Key": idempotencyKey },
      body: JSON.stringify(payload),
    });
  }

  async discardUseCaseDraft(useCaseId: number) {
    return this.request(this.useCases(`/${useCaseId}/draft`), { method: "DELETE" });
  }

  /** Project close-out, or one requirement's when requirementId is given. */
  async getCloseOut(requirementId?: number) {
    return this.request(
      this.project(requirementId != null ? `/requirements/${requirementId}/close_out` : "/close_out")
    );
  }

  async getImpactedTests(requirementId: number) {
    return this.request(this.project(`/requirements/${requirementId}/impacted_tests`));
  }

  async listVerificationSnapshots(verificationId: number) {
    return this.request(this.project(`/verifications/${verificationId}/snapshots`));
  }

  async compareVerificationSnapshots(verificationId: number, v1: number, v2: number) {
    return this.request(this.project(`/verifications/${verificationId}/snapshots/${v1}/diff/${v2}`));
  }

  async setVerificationField(verificationId: number, field: string, value: string) {
    return this.request(this.project(`/verifications/${verificationId}/field`), {
      method: "POST",
      body: JSON.stringify({ field, value }),
    });
  }

  async getBaselineContents(baselineId: number, kind: "use_cases" | "verifications") {
    return this.request(
      this.project(`/baselines/${baselineId}/${kind === "use_cases" ? "use-cases" : "verifications"}`)
    );
  }

  async diffBaselineVerificationVsCurrent(baselineId: number, verificationId: number) {
    return this.request(this.project(`/baselines/${baselineId}/verifications/${verificationId}/diff/current`));
  }

  get projectId() {
    return this.ctx.projectId;
  }

  get sessionId() {
    return this.ctx.sessionId;
  }
}
