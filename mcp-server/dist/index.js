#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { contextAllowsReadExtended, loadContext } from "./context.js";
import { MarreqClient } from "./client.js";
function jsonContent(data) {
    const text = typeof data === "string" ? data : JSON.stringify(data, null, 2);
    return { type: "text", text };
}
async function withAudit(client, toolName, paramsSummary, isWrite, fn) {
    let resultSummary;
    try {
        const out = await fn();
        resultSummary =
            typeof out === "object"
                ? JSON.stringify(out).slice(0, 500)
                : String(out);
        await client.postAudit({
            project_id: client.projectId,
            session_id: client.sessionId,
            tool_name: toolName,
            params_summary: paramsSummary,
            result_summary: resultSummary,
            is_write: isWrite,
        }).catch(() => { });
        return out;
    }
    catch (err) {
        resultSummary = err instanceof Error ? err.message : String(err);
        await client.postAudit({
            project_id: client.projectId,
            session_id: client.sessionId,
            tool_name: toolName,
            params_summary: paramsSummary,
            result_summary: `error: ${resultSummary}`,
            is_write: isWrite,
        }).catch(() => { });
        throw err;
    }
}
async function main() {
    const ctx = loadContext();
    const client = new MarreqClient(ctx);
    const server = new McpServer({
        name: "marreq-mcp-server",
        version: "0.1.0",
    });
    server.registerTool("get_requirement", {
        description: "Get a requirement by id (project-scoped, with trace summary)",
        inputSchema: z.object({ requirement_id: z.string() }),
    }, async ({ requirement_id }) => {
        const id = parseInt(requirement_id, 10);
        const out = await withAudit(client, "get_requirement", JSON.stringify({ requirement_id }), false, () => client.getRequirement(id));
        return { content: [jsonContent(out)] };
    });
    server.registerTool("list_requirements", {
        description: "List requirements in the project with optional filters (approval_state, has_tests)",
        inputSchema: z.object({
            filter: z
                .object({
                approval_state: z.enum(["draft", "reviewed", "approved"]).optional(),
                has_tests: z.boolean().optional(),
            })
                .optional(),
        }),
    }, async (args) => {
        const f = args?.filter;
        const out = await withAudit(client, "list_requirements", JSON.stringify(args ?? {}), false, () => client.listRequirements(f?.approval_state, f?.has_tests));
        return { content: [jsonContent(out)] };
    });
    server.registerTool("get_versions", {
        description: "Get version history for a requirement",
        inputSchema: z.object({ requirement_id: z.string() }),
    }, async ({ requirement_id }) => {
        const id = parseInt(requirement_id, 10);
        const out = await withAudit(client, "get_versions", JSON.stringify({ requirement_id }), false, () => client.getVersions(id));
        return { content: [jsonContent(out)] };
    });
    server.registerTool("compare_versions", {
        description: "Structured diff between two requirement versions",
        inputSchema: z.object({
            requirement_id: z.string(),
            v1: z.number(),
            v2: z.number(),
        }),
    }, async ({ requirement_id, v1, v2 }) => {
        const id = parseInt(requirement_id, 10);
        const out = await withAudit(client, "compare_versions", JSON.stringify({ requirement_id, v1, v2 }), false, () => client.compareVersions(id, v1, v2));
        return { content: [jsonContent(out)] };
    });
    server.registerTool("trace_up", {
        description: "Get parent requirement(s) for a requirement",
        inputSchema: z.object({ requirement_id: z.string() }),
    }, async ({ requirement_id }) => {
        const id = parseInt(requirement_id, 10);
        const out = await withAudit(client, "trace_up", JSON.stringify({ requirement_id }), false, () => client.traceUp(id));
        return { content: [jsonContent(out)] };
    });
    server.registerTool("trace_down", {
        description: "Get child requirements and linked tests",
        inputSchema: z.object({ requirement_id: z.string() }),
    }, async ({ requirement_id }) => {
        const id = parseInt(requirement_id, 10);
        const out = await withAudit(client, "trace_down", JSON.stringify({ requirement_id }), false, () => client.traceDown(id));
        return { content: [jsonContent(out)] };
    });
    server.registerTool("coverage_report", {
        description: "Requirements without tests, tests without requirements, suspect links (scope: project)",
        inputSchema: z.object({ scope: z.literal("project").optional() }),
    }, async () => {
        const out = await withAudit(client, "coverage_report", '{"scope":"project"}', false, () => client.coverageReport());
        return { content: [jsonContent(out)] };
    });
    server.registerTool("get_baseline", {
        description: "Get baseline metadata, requirements snapshot, and traceability",
        inputSchema: z.object({ baseline_id: z.number() }),
    }, async ({ baseline_id }) => {
        const out = await withAudit(client, "get_baseline", JSON.stringify({ baseline_id }), false, () => client.getBaseline(baseline_id));
        return { content: [jsonContent(out)] };
    });
    server.registerTool("diff_baselines", {
        description: "Compare two baselines (requirements and traceability diff)",
        inputSchema: z.object({
            baseline_a: z.number(),
            baseline_b: z.number(),
        }),
    }, async ({ baseline_a, baseline_b }) => {
        const out = await withAudit(client, "diff_baselines", JSON.stringify({ baseline_a, baseline_b }), false, () => client.diffBaselines(baseline_a, baseline_b));
        return { content: [jsonContent(out)] };
    });
    // read_extended / draft_write: extra read tools (catalog, verifications, audit, matrix read, …)
    if (contextAllowsReadExtended(ctx)) {
        server.registerTool("list_verifications", {
            description: "List verifications (tests) in the project. Requires MARREQ_MODE=read_extended or draft_write.",
            inputSchema: z.object({}),
        }, async () => {
            const out = await withAudit(client, "list_verifications", "{}", false, () => client.listVerificationsByProject());
            return { content: [jsonContent(out)] };
        });
        server.registerTool("get_verification", {
            description: "Get one verification by id. The record must belong to MARREQ_PROJECT_ID. Requires read_extended or draft_write mode.",
            inputSchema: z.object({ verification_id: z.string() }),
        }, async ({ verification_id }) => {
            const id = parseInt(verification_id, 10);
            const out = await withAudit(client, "get_verification", JSON.stringify({ verification_id }), false, async () => {
                const row = (await client.getVerificationById(id));
                if (row?.project_id != null && row.project_id !== ctx.projectId) {
                    throw new Error(`Verification ${id} is not in project ${ctx.projectId}`);
                }
                return row;
            });
            return { content: [jsonContent(out)] };
        });
        server.registerTool("list_baselines", {
            description: "List baselines for the project (metadata only). Use get_baseline for full snapshot.",
            inputSchema: z.object({}),
        }, async () => {
            const out = await withAudit(client, "list_baselines", "{}", false, () => client.listBaselinesByProject());
            return { content: [jsonContent(out)] };
        });
        server.registerTool("get_requirement_activity", {
            description: "Audit log entries for a requirement (create/update history with field summaries).",
            inputSchema: z.object({ requirement_id: z.string() }),
        }, async ({ requirement_id }) => {
            const id = parseInt(requirement_id, 10);
            const out = await withAudit(client, "get_requirement_activity", JSON.stringify({ requirement_id }), false, () => client.getRequirementActivity(id));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("get_verification_activity", {
            description: "Audit log entries for a verification (test).",
            inputSchema: z.object({ verification_id: z.string() }),
        }, async ({ verification_id }) => {
            const id = parseInt(verification_id, 10);
            const out = await withAudit(client, "get_verification_activity", JSON.stringify({ verification_id }), false, () => client.getVerificationActivity(id));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("list_requirement_comments", {
            description: "List comments for a requirement. Optional requirement_version_id filters by version.",
            inputSchema: z.object({
                requirement_id: z.string(),
                requirement_version_id: z.number().optional(),
            }),
        }, async ({ requirement_id, requirement_version_id }) => {
            const id = parseInt(requirement_id, 10);
            const out = await withAudit(client, "list_requirement_comments", JSON.stringify({ requirement_id, requirement_version_id }), false, () => client.listRequirementComments(id, requirement_version_id ?? null));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("get_verification_matrix", {
            description: "Requirement ids linked to a verification in the traceability matrix (read).",
            inputSchema: z.object({ verification_id: z.string() }),
        }, async ({ verification_id }) => {
            const id = parseInt(verification_id, 10);
            const out = await withAudit(client, "get_verification_matrix", JSON.stringify({ verification_id }), false, () => client.getVerificationMatrix(id));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("list_project_catalog", {
            description: "Project-scoped catalog: categories, applicability, requirement/verification statuses, verification methods, custom field definitions.",
            inputSchema: z.object({}),
        }, async () => {
            const out = await withAudit(client, "list_project_catalog", "{}", false, () => client.listProjectCatalog());
            return { content: [jsonContent(out)] };
        });
        server.registerTool("diff_baseline_vs_current", {
            description: "Structured diff between a requirement as captured in a baseline and its current version.",
            inputSchema: z.object({
                baseline_id: z.number(),
                requirement_id: z.string(),
            }),
        }, async ({ baseline_id, requirement_id }) => {
            const rid = parseInt(requirement_id, 10);
            const out = await withAudit(client, "diff_baseline_vs_current", JSON.stringify({ baseline_id, requirement_id }), false, () => client.diffBaselineVsCurrent(baseline_id, rid));
            return { content: [jsonContent(out)] };
        });
    }
    // Phase 2: draft_write tools (only when MARREQ_MODE=draft_write)
    if (ctx.mode === "draft_write") {
        server.registerTool("create_requirement", {
            description: "Create a new requirement in the project (draft). Requires draft_write mode.",
            inputSchema: z.object({
                title: z.string(),
                description: z.string(),
                reference_code: z.string(),
                author_id: z.number(),
                reviewer_id: z.number(),
                category_id: z.number(),
                status_id: z.number(),
                applicability_id: z.number(),
                justification: z.string().nullable().optional(),
                verification_method_ids: z.array(z.number()),
                custom_fields: z
                    .array(z.object({ field_id: z.number(), value: z.string() }))
                    .optional(),
            }),
        }, async (args) => {
            const projectId = ctx.projectId;
            const payload = {
                ...args,
                project_id: projectId,
            };
            const out = await withAudit(client, "create_requirement", JSON.stringify({ ...args, project_id: projectId }), true, () => client.createRequirement(payload));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("patch_requirement", {
            description: "Update a requirement (creates new version). Requires draft_write mode. Changing status_id requires the token user to be in the project's reviewer list (or admin).",
            inputSchema: z.object({
                requirement_id: z.string(),
                patch: z.object({
                    title: z.string().optional(),
                    description: z.string().optional(),
                    status_id: z.number().optional(),
                    verification_method_ids: z.array(z.number()).optional(),
                    author_id: z.number().optional(),
                    reviewer_id: z.number().optional(),
                    category_id: z.number().optional(),
                    applicability_id: z.number().optional(),
                    custom_fields: z
                        .array(z.object({ field_id: z.number(), value: z.string() }))
                        .optional(),
                }),
            }),
        }, async ({ requirement_id, patch }) => {
            const id = parseInt(requirement_id, 10);
            const out = await withAudit(client, "patch_requirement", JSON.stringify({ requirement_id, patch }), true, () => client.patchRequirement(id, patch));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("set_approval", {
            description: "Set requirement version approval state (reviewed or approved). Requires draft_write mode; the token user must be a designated project reviewer (or admin).",
            inputSchema: z.object({
                requirement_id: z.string(),
                version_id: z.number(),
                state: z.enum(["reviewed", "approved"]),
            }),
        }, async ({ requirement_id, version_id, state }) => {
            const reqId = parseInt(requirement_id, 10);
            const out = await withAudit(client, "set_approval", JSON.stringify({ requirement_id, version_id, state }), true, () => client.setApproval(reqId, version_id, state));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("create_baseline", {
            description: "Create a new baseline snapshot for the project. Requires draft_write mode.",
            inputSchema: z.object({
                name: z.string(),
                description: z.string().nullable().optional(),
            }),
        }, async (args) => {
            const payload = {
                name: args.name,
                description: args.description ?? null,
            };
            const out = await withAudit(client, "create_baseline", JSON.stringify(args), true, () => client.createBaseline(payload));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("create_requirement_comment", {
            description: "Add a comment on a requirement. Optional requirement_version_id ties the comment to a version.",
            inputSchema: z.object({
                requirement_id: z.string(),
                body: z.string(),
                requirement_version_id: z.number().optional(),
            }),
        }, async ({ requirement_id, body, requirement_version_id }) => {
            const id = parseInt(requirement_id, 10);
            const out = await withAudit(client, "create_requirement_comment", JSON.stringify({
                requirement_id,
                body_len: body.length,
                requirement_version_id,
            }), true, () => client.createRequirementComment(id, body, requirement_version_id ?? null));
            return { content: [jsonContent(out)] };
        });
    }
    if (ctx.traceWrite) {
        server.registerTool("put_verification_matrix", {
            description: "Replace traceability links for a verification with the given requirement ids (full replace). Requires MARREQ_TRACE_WRITE=true and EditRequirements on the API.",
            inputSchema: z.object({
                verification_id: z.string(),
                requirement_ids: z.array(z.number()),
            }),
        }, async ({ verification_id, requirement_ids }) => {
            const vid = parseInt(verification_id, 10);
            const out = await withAudit(client, "put_verification_matrix", JSON.stringify({ verification_id, requirement_ids }), true, () => client.putVerificationMatrix(vid, requirement_ids));
            return { content: [jsonContent(out)] };
        });
        server.registerTool("clear_suspect", {
            description: "Clear the suspect flag on a requirement↔verification matrix link. Requires MARREQ_TRACE_WRITE=true.",
            inputSchema: z.object({
                req_id: z.number(),
                verification_id: z.number(),
            }),
        }, async ({ req_id, verification_id }) => {
            const out = await withAudit(client, "clear_suspect", JSON.stringify({ req_id, verification_id }), true, () => client.clearSuspectLink(req_id, verification_id));
            return { content: [jsonContent(out)] };
        });
    }
    const transport = new StdioServerTransport();
    await server.connect(transport);
}
main().catch((err) => {
    console.error(err);
    process.exit(1);
});
