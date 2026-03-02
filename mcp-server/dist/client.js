export class MarreqClient {
    ctx;
    constructor(ctx) {
        this.ctx = ctx;
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
    async createRequirement(payload) {
        return this.request(`/api/projects/${this.ctx.projectId}/requirements`, { method: "POST", body: JSON.stringify(payload) });
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
    async createBaseline(payload) {
        return this.request(`/api/projects/${this.ctx.projectId}/baselines`, { method: "POST", body: JSON.stringify(payload) });
    }
    async postAudit(payload) {
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
