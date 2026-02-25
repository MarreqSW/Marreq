#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { loadContext } from "./context.js";
import { ReqManClient } from "./client.js";

function jsonContent(data: unknown) {
  const text =
    typeof data === "string" ? data : JSON.stringify(data, null, 2);
  return { type: "text" as const, text };
}

async function withAudit(
  client: ReqManClient,
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
    }).catch(() => {});
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

async function main() {
  const ctx = loadContext();
  const client = new ReqManClient(ctx);

  const server = new McpServer({
    name: "reqman-mcp-server",
    version: "0.1.0",
  });

  server.registerTool(
    "get_requirement",
    {
      description: "Get a requirement by id (project-scoped, with trace summary)",
      inputSchema: z.object({ requirement_id: z.string() }),
    },
    async ({ requirement_id }) => {
      const id = parseInt(requirement_id, 10);
      const out = await withAudit(
        client,
        "get_requirement",
        JSON.stringify({ requirement_id }),
        false,
        () => client.getRequirement(id)
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
      const out = await withAudit(
        client,
        "list_requirements",
        JSON.stringify(args ?? {}),
        false,
        () =>
          client.listRequirements(
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
      inputSchema: z.object({ requirement_id: z.string() }),
    },
    async ({ requirement_id }) => {
      const id = parseInt(requirement_id, 10);
      const out = await withAudit(
        client,
        "get_versions",
        JSON.stringify({ requirement_id }),
        false,
        () => client.getVersions(id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "compare_versions",
    {
      description: "Structured diff between two requirement versions",
      inputSchema: z.object({
        requirement_id: z.string(),
        v1: z.number(),
        v2: z.number(),
      }),
    },
    async ({ requirement_id, v1, v2 }) => {
      const id = parseInt(requirement_id, 10);
      const out = await withAudit(
        client,
        "compare_versions",
        JSON.stringify({ requirement_id, v1, v2 }),
        false,
        () => client.compareVersions(id, v1, v2)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "trace_up",
    {
      description: "Get parent requirement(s) for a requirement",
      inputSchema: z.object({ requirement_id: z.string() }),
    },
    async ({ requirement_id }) => {
      const id = parseInt(requirement_id, 10);
      const out = await withAudit(
        client,
        "trace_up",
        JSON.stringify({ requirement_id }),
        false,
        () => client.traceUp(id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "trace_down",
    {
      description: "Get child requirements and linked tests",
      inputSchema: z.object({ requirement_id: z.string() }),
    },
    async ({ requirement_id }) => {
      const id = parseInt(requirement_id, 10);
      const out = await withAudit(
        client,
        "trace_down",
        JSON.stringify({ requirement_id }),
        false,
        () => client.traceDown(id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "coverage_report",
    {
      description:
        "Requirements without tests, tests without requirements, suspect links (scope: project)",
      inputSchema: z.object({ scope: z.literal("project").optional() }),
    },
    async () => {
      const out = await withAudit(
        client,
        "coverage_report",
        '{"scope":"project"}',
        false,
        () => client.coverageReport()
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "get_baseline",
    {
      description: "Get baseline metadata, requirements snapshot, and traceability",
      inputSchema: z.object({ baseline_id: z.number() }),
    },
    async ({ baseline_id }) => {
      const out = await withAudit(
        client,
        "get_baseline",
        JSON.stringify({ baseline_id }),
        false,
        () => client.getBaseline(baseline_id)
      );
      return { content: [jsonContent(out)] };
    }
  );

  server.registerTool(
    "diff_baselines",
    {
      description: "Compare two baselines (requirements and traceability diff)",
      inputSchema: z.object({
        baseline_a: z.number(),
        baseline_b: z.number(),
      }),
    },
    async ({ baseline_a, baseline_b }) => {
      const out = await withAudit(
        client,
        "diff_baselines",
        JSON.stringify({ baseline_a, baseline_b }),
        false,
        () => client.diffBaselines(baseline_a, baseline_b)
      );
      return { content: [jsonContent(out)] };
    }
  );

  // Phase 2: draft_write tools (only when REQMAN_MODE=draft_write)
  if (ctx.mode === "draft_write") {
    server.registerTool(
      "create_requirement",
      {
        description:
          "Create a new requirement in the project (draft). Requires draft_write mode.",
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
      },
      async (args) => {
        const projectId = ctx.projectId;
        const payload = {
          ...args,
          project_id: projectId,
        };
        const out = await withAudit(
          client,
          "create_requirement",
          JSON.stringify({ ...args, project_id: projectId }),
          true,
          () => client.createRequirement(payload)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "patch_requirement",
      {
        description:
          "Update a requirement (creates new version). Requires draft_write mode.",
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
      },
      async ({ requirement_id, patch }) => {
        const id = parseInt(requirement_id, 10);
        const out = await withAudit(
          client,
          "patch_requirement",
          JSON.stringify({ requirement_id, patch }),
          true,
          () => client.patchRequirement(id, patch)
        );
        return { content: [jsonContent(out)] };
      }
    );

    server.registerTool(
      "set_approval",
      {
        description:
          "Set requirement version approval state (reviewed or approved). Requires draft_write mode and project owner/manager role.",
        inputSchema: z.object({
          requirement_id: z.string(),
          version_id: z.number(),
          state: z.enum(["reviewed", "approved"]),
        }),
      },
      async ({ requirement_id, version_id, state }) => {
        const reqId = parseInt(requirement_id, 10);
        const out = await withAudit(
          client,
          "set_approval",
          JSON.stringify({ requirement_id, version_id, state }),
          true,
          () => client.setApproval(reqId, version_id, state)
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
          name: z.string(),
          description: z.string().nullable().optional(),
        }),
      },
      async (args) => {
        const payload = {
          name: args.name,
          description: args.description ?? null,
        };
        const out = await withAudit(
          client,
          "create_baseline",
          JSON.stringify(args),
          true,
          () => client.createBaseline(payload)
        );
        return { content: [jsonContent(out)] };
      }
    );
  }

  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
