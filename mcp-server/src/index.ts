#!/usr/bin/env node
import { pathToFileURL } from "node:url";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { contextAllowsReadExtended, loadContext } from "./context.js";
import type { SessionContext } from "./context.js";
import { MarreqClient } from "./client.js";
import { loadTransportConfig, startRemoteServer } from "./remote.js";

function jsonContent(data: unknown) {
  const text =
    typeof data === "string" ? data : JSON.stringify(data, null, 2);
  return { type: "text" as const, text };
}

async function withAudit(
  client: MarreqClient,
  toolName: string,
  paramsSummary: string,
  isWrite: boolean,
  fn: () => Promise<unknown>
) {
  let resultSummary: string;
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
    });
    return out;
  } catch (err) {
    resultSummary = err instanceof Error ? err.message : String(err);
    await client.postAudit({
      project_id: client.projectId,
      session_id: client.sessionId,
      tool_name: toolName,
      params_summary: paramsSummary,
      result_summary: `error: ${resultSummary}`,
      is_write: isWrite,
    }).catch(() => {});
    throw err;
  }
}

export function createMarreqServer(ctx: SessionContext) {
  const client = new MarreqClient(ctx);
  const projectField = { project_id: z.number().int().positive().optional() };
  const forProject = (projectId: number | undefined) => {
    if (!ctx.remote) return client;
    if (!projectId) throw new Error("project_id is required in remote mode");
    return client.withProject(projectId);
  };

  const server = new McpServer({
    name: "marreq-mcp-server",
    version: "0.1.0",
  });

  server.registerTool(
    "list_projects",
    {
      description: "List only projects accessible to the authenticated Marreq user.",
      inputSchema: z.object({}),
      annotations: { readOnlyHint: true },
    },
    async () => ({ content: [jsonContent(await client.listProjects())] })
  );

  server.registerTool(
    "get_requirement",
    {
      description: "Get a requirement by id (project-scoped, with trace summary)",
      inputSchema: z.object({ ...projectField, requirement_id: z.string() }),
    },
    async ({ project_id, requirement_id }) => {
      const id = parseInt(requirement_id, 10);
      const toolClient = forProject(project_id);
      const out = await withAudit(
        toolClient,
        "get_requirement",
        JSON.stringify({ requirement_id }),
        false,
        () => toolClient.getRequirement(id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "list_requirements",
    {
      description:
        "List requirements in the project with optional filters (approval_state, has_tests)",
      inputSchema: z.object({
        ...projectField,
        filter: z
          .object({
            approval_state: z.enum(["draft", "reviewed", "approved"]).optional(),
            has_tests: z.boolean().optional(),
          })
          .optional(),
      }),
    },
    async (args) => {
      const f = args?.filter;
      const toolClient = forProject(args?.project_id);
      const out = await withAudit(
        toolClient,
        "list_requirements",
        JSON.stringify(args ?? {}),
        false,
        () =>
          toolClient.listRequirements(
            f?.approval_state,
            f?.has_tests
          )
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "get_versions",
    {
      description: "Get version history for a requirement",
      inputSchema: z.object({ ...projectField, requirement_id: z.string() }),
    },
    async ({ project_id, requirement_id }) => {
      const id = parseInt(requirement_id, 10);
      const toolClient = forProject(project_id);
      const out = await withAudit(
        toolClient,
        "get_versions",
        JSON.stringify({ requirement_id }),
        false,
        () => toolClient.getVersions(id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "semantic_search_requirements",
    {
      description: "Semantic requirement search when embeddings are enabled; returns an explicit disabled response otherwise.",
      inputSchema: z.object({ ...projectField, query: z.string().min(1), limit: z.number().int().min(1).max(50).optional() }),
      annotations: { readOnlyHint: true },
    },
    async ({ project_id, query, limit }) => {
      const toolClient = forProject(project_id);
      const out = await withAudit(toolClient, "semantic_search_requirements", JSON.stringify({ query_length: query.length, limit }), false, () => toolClient.semanticSearchRequirements(query, limit));
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "compare_versions",
    {
      description: "Structured diff between two requirement versions",
      inputSchema: z.object({
        ...projectField,
        requirement_id: z.string(),
        v1: z.number(),
        v2: z.number(),
      }),
    },
    async ({ project_id, requirement_id, v1, v2 }) => {
      const id = parseInt(requirement_id, 10);
      const toolClient = forProject(project_id);
      const out = await withAudit(
        toolClient,
        "compare_versions",
        JSON.stringify({ requirement_id, v1, v2 }),
        false,
        () => toolClient.compareVersions(id, v1, v2)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "trace_up",
    {
      description: "Get parent requirement(s) for a requirement",
      inputSchema: z.object({ ...projectField, requirement_id: z.string() }),
    },
    async ({ project_id, requirement_id }) => {
      const id = parseInt(requirement_id, 10);
      const toolClient = forProject(project_id);
      const out = await withAudit(
        toolClient,
        "trace_up",
        JSON.stringify({ requirement_id }),
        false,
        () => toolClient.traceUp(id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "trace_down",
    {
      description: "Get child requirements and linked tests",
      inputSchema: z.object({ ...projectField, requirement_id: z.string() }),
    },
    async ({ project_id, requirement_id }) => {
      const id = parseInt(requirement_id, 10);
      const toolClient = forProject(project_id);
      const out = await withAudit(
        toolClient,
        "trace_down",
        JSON.stringify({ requirement_id }),
        false,
        () => toolClient.traceDown(id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "coverage_report",
    {
      description:
        "Requirements without tests, tests without requirements, suspect links (scope: project)",
      inputSchema: z.object({ ...projectField, scope: z.literal("project").optional() }),
    },
    async ({ project_id }) => {
      const toolClient = forProject(project_id);
      const out = await withAudit(
        toolClient,
        "coverage_report",
        '{"scope":"project"}',
        false,
        () => toolClient.coverageReport()
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "get_baseline",
    {
      description: "Get baseline metadata, requirements snapshot, and traceability",
      inputSchema: z.object({ ...projectField, baseline_id: z.number() }),
    },
    async ({ project_id, baseline_id }) => {
      const toolClient = forProject(project_id);
      const out = await withAudit(
        toolClient,
        "get_baseline",
        JSON.stringify({ baseline_id }),
        false,
        () => toolClient.getBaseline(baseline_id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "diff_baselines",
    {
      description: "Compare two baselines (requirements and traceability diff)",
      inputSchema: z.object({
        ...projectField,
        baseline_a: z.number(),
        baseline_b: z.number(),
      }),
    },
    async ({ project_id, baseline_a, baseline_b }) => {
      const toolClient = forProject(project_id);
      const out = await withAudit(
        toolClient,
        "diff_baselines",
        JSON.stringify({ baseline_a, baseline_b }),
        false,
        () => toolClient.diffBaselines(baseline_a, baseline_b)
      );
      return { content: [jsonContent(out)] };
    }
  );

  // read_extended / draft_write: extra read tools (catalog, verifications, audit, matrix read, …)
  if (contextAllowsReadExtended(ctx)) {
    server.registerTool(
      "list_verifications",
      {
        description:
          "List verifications (tests) in the project. Requires MARREQ_MODE=read_extended or draft_write.",
        inputSchema: z.object({ ...projectField }),
      },
      async ({ project_id }) => {
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "list_verifications",
          "{}",
          false,
          () => toolClient.listVerificationsByProject()
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "get_verification",
      {
        description:
          "Get one verification by id. The record must belong to MARREQ_PROJECT_ID. Requires read_extended or draft_write mode.",
        inputSchema: z.object({ ...projectField, verification_id: z.string() }),
      },
      async ({ project_id, verification_id }) => {
        const id = parseInt(verification_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "get_verification",
          JSON.stringify({ verification_id }),
          false,
          async () => {
            const row = (await toolClient.getVerificationById(id)) as {
              project_id?: number;
            };
            const expectedProject = project_id ?? ctx.projectId;
            if (row?.project_id != null && row.project_id !== expectedProject) {
              throw new Error(
                `Verification ${id} is not in project ${expectedProject}`
              );
            }
            return row;
          }
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "list_baselines",
      {
        description:
          "List baselines for the project (metadata only). Use get_baseline for full snapshot.",
        inputSchema: z.object({ ...projectField }),
      },
      async ({ project_id }) => {
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "list_baselines",
          "{}",
          false,
          () => toolClient.listBaselinesByProject()
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "get_requirement_activity",
      {
        description:
          "Audit log entries for a requirement (create/update history with field summaries).",
        inputSchema: z.object({ ...projectField, requirement_id: z.string() }),
      },
      async ({ project_id, requirement_id }) => {
        const id = parseInt(requirement_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "get_requirement_activity",
          JSON.stringify({ requirement_id }),
          false,
          () => toolClient.getRequirementActivity(id)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "get_verification_activity",
      {
        description: "Audit log entries for a verification (test).",
        inputSchema: z.object({ ...projectField, verification_id: z.string() }),
      },
      async ({ project_id, verification_id }) => {
        const id = parseInt(verification_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "get_verification_activity",
          JSON.stringify({ verification_id }),
          false,
          () => toolClient.getVerificationActivity(id)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "list_requirement_comments",
      {
        description:
          "List comments for a requirement. Optional requirement_version_id filters by version.",
        inputSchema: z.object({
          ...projectField,
          ...projectField,
          requirement_id: z.string(),
          requirement_version_id: z.number().optional(),
        }),
      },
      async ({ project_id, requirement_id, requirement_version_id }) => {
        const id = parseInt(requirement_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "list_requirement_comments",
          JSON.stringify({ requirement_id, requirement_version_id }),
          false,
          () =>
            toolClient.listRequirementComments(id, requirement_version_id ?? null)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "get_verification_matrix",
      {
        description:
          "Requirement ids linked to a verification in the traceability matrix (read).",
        inputSchema: z.object({ ...projectField, verification_id: z.string() }),
      },
      async ({ project_id, verification_id }) => {
        const id = parseInt(verification_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "get_verification_matrix",
          JSON.stringify({ verification_id }),
          false,
          () => toolClient.getVerificationMatrix(id)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "list_project_catalog",
      {
        description:
          "Project-scoped catalog: categories, applicability, requirement/verification statuses, verification methods, custom field definitions.",
        inputSchema: z.object({ ...projectField }),
      },
      async ({ project_id }) => {
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "list_project_catalog",
          "{}",
          false,
          () => toolClient.listProjectCatalog()
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "diff_baseline_vs_current",
      {
        description:
          "Structured diff between a requirement as captured in a baseline and its current version.",
        inputSchema: z.object({
          ...projectField,
          baseline_id: z.number(),
          requirement_id: z.string(),
        }),
      },
      async ({ project_id, baseline_id, requirement_id }) => {
        const rid = parseInt(requirement_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "diff_baseline_vs_current",
          JSON.stringify({ baseline_id, requirement_id }),
          false,
          () => toolClient.diffBaselineVsCurrent(baseline_id, rid)
        );
        return { content: [jsonContent(out)] };
      }
    );
  }

  // Phase 2: draft_write tools (only when MARREQ_MODE=draft_write)
  if (ctx.mode === "draft_write") {
    server.registerTool(
      "create_requirement",
      {
        description:
          "Create a new draft requirement. reference_code is the persistent idempotency identity: retry with the same reference, title, and description returns the existing requirement; conflicting content is rejected.",
        inputSchema: z.object({
          ...projectField,
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
      },
      async (args) => {
        const projectId = ctx.remote ? args.project_id : ctx.projectId;
        if (!projectId) throw new Error("project_id is required in remote mode");
        const toolClient = forProject(projectId);
        const payload = {
          ...args,
          project_id: projectId,
        };
        const out = await withAudit(
          toolClient,
          "create_requirement",
          JSON.stringify({ ...args, project_id: projectId }),
          true,
          () => toolClient.createRequirement(payload)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "patch_requirement",
      {
        description:
          "Update a requirement (creates new version). Requires draft_write mode. Changing status_id requires the token user to be in the project's reviewer list (or admin).",
        inputSchema: z.object({
          ...projectField,
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
      },
      async ({ project_id, requirement_id, patch }) => {
        const id = parseInt(requirement_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "patch_requirement",
          JSON.stringify({ requirement_id, patch }),
          true,
          () => toolClient.patchRequirement(id, patch)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "set_approval",
      {
        description:
          "Set requirement version approval state (reviewed or approved). Requires draft_write mode; the token user must be a designated project reviewer (or admin).",
        inputSchema: z.object({
          ...projectField,
          requirement_id: z.string(),
          version_id: z.number(),
          state: z.enum(["reviewed", "approved"]),
        }),
      },
      async ({ project_id, requirement_id, version_id, state }) => {
        const reqId = parseInt(requirement_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "set_approval",
          JSON.stringify({ requirement_id, version_id, state }),
          true,
          () => toolClient.setApproval(reqId, version_id, state)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "create_baseline",
      {
        description:
          "Create a new baseline snapshot for the project. Requires draft_write mode.",
        inputSchema: z.object({
          ...projectField,
          name: z.string(),
          description: z.string().nullable().optional(),
        }),
      },
      async (args) => {
        const toolClient = forProject(args.project_id);
        const payload = {
          name: args.name,
          description: args.description ?? null,
        };
        const out = await withAudit(
          toolClient,
          "create_baseline",
          JSON.stringify(args),
          true,
          () => toolClient.createBaseline(payload)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "create_requirement_comment",
      {
        description:
          "Add a comment on a requirement. Optional requirement_version_id ties the comment to a version.",
        inputSchema: z.object({
          ...projectField,
          requirement_id: z.string(),
          body: z.string(),
          requirement_version_id: z.number().optional(),
        }),
      },
      async ({ project_id, requirement_id, body, requirement_version_id }) => {
        const id = parseInt(requirement_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "create_requirement_comment",
          JSON.stringify({
            requirement_id,
            body_len: body.length,
            requirement_version_id,
          }),
          true,
          () =>
            toolClient.createRequirementComment(
              id,
              body,
              requirement_version_id ?? null
            )
        );
        return { content: [jsonContent(out)] };
      }
    );
  }

  if (ctx.traceWrite) {
    server.registerTool(
      "put_verification_matrix",
      {
        description:
          "Replace traceability links for a verification with the given requirement ids (full replace). Requires MARREQ_TRACE_WRITE=true and EditRequirements on the API.",
        inputSchema: z.object({
          ...projectField,
          verification_id: z.string(),
          requirement_ids: z.array(z.number()),
        }),
      },
      async ({ project_id, verification_id, requirement_ids }) => {
        const vid = parseInt(verification_id, 10);
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "put_verification_matrix",
          JSON.stringify({ verification_id, requirement_ids }),
          true,
          () => toolClient.putVerificationMatrix(vid, requirement_ids)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "clear_suspect",
      {
        description:
          "Clear the suspect flag on a requirement↔verification matrix link. Requires MARREQ_TRACE_WRITE=true.",
        inputSchema: z.object({
          ...projectField,
          req_id: z.number(),
          verification_id: z.number(),
        }),
      },
      async ({ project_id, req_id, verification_id }) => {
        const toolClient = forProject(project_id);
        const out = await withAudit(
          toolClient,
          "clear_suspect",
          JSON.stringify({ req_id, verification_id }),
          true,
          () => toolClient.clearSuspectLink(req_id, verification_id)
        );
        return { content: [jsonContent(out)] };
      }
    );
  }

  return server;
}

async function main() {
  const transportConfig = loadTransportConfig();
  if (transportConfig.kind === "http") {
    await startRemoteServer(transportConfig, createMarreqServer);
    return;
  }

  const server = createMarreqServer(loadContext());
  await server.connect(new StdioServerTransport());
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((err) => {
    console.error(err instanceof Error ? err.message : "MCP server failed");
    process.exitCode = 1;
  });
}
