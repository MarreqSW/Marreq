export class MarreqAuthenticationError extends Error {
    status;
    constructor(status) {
        super("Marreq authorization is required");
        this.status = status;
    }
}
export class MarreqClient {
    ctx;
    constructor(ctx) {
        this.ctx = ctx;
    }
    withProject(projectId) {
        return new MarreqClient({ ...this.ctx, projectId });
    }
    async listProjects() {
        return this.request("/api/projects");
    }
    async request(path, options = {}) {
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
            if (res.status === 401 || res.status === 403) {
                throw new MarreqAuthenticationError(res.status);
            }
            throw new Error(`Marreq API ${res.status}: ${text}`);
        }
        if (res.status === 204 || res.headers.get("content-length") === "0") {
            return undefined;
        }
        return res.json();
    }
    async getRequirement(id) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${id}`);
    }
    async listRequirements(approvalState, hasTests) {
        const params = new URLSearchParams();
        if (approvalState != null)
            params.set("approval_state", approvalState);
        if (hasTests != null)
            params.set("has_tests", String(hasTests));
        const q = params.toString();
        return this.request(`/api/projects/${this.ctx.projectId}/requirements${q ? `?${q}` : ""}`);
    }
    async semanticSearchRequirements(query, limit) {
        const params = new URLSearchParams({ q: query });
        if (limit != null)
            params.set("k", String(limit));
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/semantic_search?${params}`);
    }
    async getVersions(requirementId) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/versions`);
    }
    async compareVersions(requirementId, v1, v2) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/versions/${v1}/diff/${v2}`);
    }
    async traceUp(requirementId) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/trace_up`);
    }
    async traceDown(requirementId) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/trace_down`);
    }
    async coverageReport() {
        return this.request(`/api/projects/${this.ctx.projectId}/coverage_report`);
    }
    async getBaseline(baselineId) {
        const [meta, requirements, traceability] = await Promise.all([
            this.request(`/api/projects/${this.ctx.projectId}/baselines/${baselineId}`),
            this.request(`/api/projects/${this.ctx.projectId}/baselines/${baselineId}/requirements`),
            this.request(`/api/projects/${this.ctx.projectId}/baselines/${baselineId}/traceability`),
        ]);
        return { baseline: meta, requirements, traceability };
    }
    async diffBaselines(baselineA, baselineB) {
        return this.request(`/api/projects/${this.ctx.projectId}/baselines/diff?baseline_a=${baselineA}&baseline_b=${baselineB}`);
    }
    /** Phase 2 draft_write: create requirement (project from context). */
    async createRequirement(payload, idempotencyKey) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements`, { method: "POST", headers: { "Idempotency-Key": idempotencyKey }, body: JSON.stringify(payload) });
    }
    async createVerification(payload, idempotencyKey) {
        return this.request(`/api/projects/${this.ctx.projectId}/verifications`, {
            method: "POST", headers: { "Idempotency-Key": idempotencyKey }, body: JSON.stringify(payload),
        });
    }
    async updateVerification(id, payload) {
        return this.request(`/api/projects/${this.ctx.projectId}/verifications/${id}`, {
            method: "PUT", body: JSON.stringify(payload),
        });
    }
    /** Phase 2 draft_write: patch requirement (project from context). */
    async patchRequirement(requirementId, patch) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}`, { method: "PATCH", body: JSON.stringify(patch) });
    }
    /** Phase 2 draft_write: set requirement version approval (reviewed | approved). */
    async setApproval(requirementId, versionId, state) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/versions/${versionId}/approval`, { method: "PUT", body: JSON.stringify({ state }) });
    }
    /** Phase 2 draft_write: create baseline (project from context). */
    async createBaseline(payload, idempotencyKey) {
        return this.request(`/api/projects/${this.ctx.projectId}/baselines`, { method: "POST", headers: { "Idempotency-Key": idempotencyKey }, body: JSON.stringify(payload) });
    }
    async postAudit(payload) {
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
    async getVerificationById(verificationId) {
        return this.request(`/api/projects/${this.ctx.projectId}/verifications/${verificationId}`);
    }
    /** GET /api/projects/:pid/baselines */
    async listBaselinesByProject() {
        return this.request(`/api/projects/${this.ctx.projectId}/baselines`);
    }
    async getRequirementActivity(requirementId) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/activity`);
    }
    async getVerificationActivity(verificationId) {
        return this.request(`/api/projects/${this.ctx.projectId}/verifications/${verificationId}/activity`);
    }
    async listRequirementComments(requirementId, versionId) {
        const q = versionId != null && versionId > 0
            ? `?version_id=${encodeURIComponent(String(versionId))}`
            : "";
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/comments${q}`);
    }
    async getVerificationMatrix(verificationId) {
        return this.request(`/api/projects/${this.ctx.projectId}/verifications/${verificationId}/matrix`);
    }
    async putVerificationMatrix(verificationId, requirementIds) {
        return this.request(`/api/projects/${this.ctx.projectId}/verifications/${verificationId}/matrix`, {
            method: "PUT",
            body: JSON.stringify({ requirement_ids: requirementIds }),
        });
    }
    async clearSuspectLink(reqId, verificationId) {
        return this.request(`/api/projects/${this.ctx.projectId}/traceability/clear_suspect`, {
            method: "POST",
            body: JSON.stringify({
                req_id: reqId,
                verification_id: verificationId,
            }),
        });
    }
    async diffBaselineVsCurrent(baselineId, requirementId) {
        return this.request(`/api/projects/${this.ctx.projectId}/baselines/${baselineId}/requirements/${requirementId}/diff/current`);
    }
    /** Aggregated catalog rows for MARREQ_PROJECT_ID (parallel GETs, filtered client-side where needed). */
    async listProjectCatalog() {
        return this.request(`/api/projects/${this.ctx.projectId}/catalog`);
    }
    async createRequirementComment(requirementId, body, requirementVersionId, idempotencyKey) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements/${requirementId}/comments`, {
            method: "POST",
            headers: { "Idempotency-Key": idempotencyKey },
            body: JSON.stringify({
                body,
                requirement_version_id: requirementVersionId != null && requirementVersionId > 0
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
