/**
 * MCP resources (issue #416): requirements, use cases and the catalog as
 * addressable documents, and the workflow guide. Each read reuses the client
 * method and backend scope of the matching tool.
 */

import { readFile } from "node:fs/promises";
import { ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { MarreqClient } from "./client.js";
import type { SessionContext } from "./context.js";

export interface ResourceDeps {
  ctx: SessionContext;
  client: MarreqClient;
  withAudit: (
    client: MarreqClient,
    toolName: string,
    paramsSummary: string,
    isWrite: boolean,
    fn: () => Promise<unknown>
  ) => Promise<unknown>;
}

const JSON_TYPE = "application/json";

function one(value: string | string[] | undefined): string {
  return Array.isArray(value) ? value[0] ?? "" : value ?? "";
}

function positiveInt(raw: string, what: string): number {
  const n = Number(raw);
  if (!Number.isInteger(n) || n <= 0) throw new Error(`Invalid ${what} in resource URI: ${raw}`);
  return n;
}

async function guideText(): Promise<string> {
  try {
    return await readFile(new URL("../MARREQ_SKILL.md", import.meta.url), "utf8");
  } catch {
    return "The Marreq workflow guide (MARREQ_SKILL.md) is not available in this deployment.";
  }
}

export function registerResources(server: McpServer, { client, withAudit }: ResourceDeps) {
  const json = (uri: URL, data: unknown) => ({
    contents: [{ uri: uri.href, mimeType: JSON_TYPE, text: JSON.stringify(data, null, 2) }],
  });

  // The project in the URI is passed through; the backend still checks that the
  // caller has the scope and project permission.
  const read = async (uri: URL, vars: Record<string, string | string[]>, name: string, fn: (c: MarreqClient) => Promise<unknown>) => {
    const projectClient = client.withProject(positiveInt(one(vars.project_id), "project id"));
    const out = await withAudit(projectClient, `resource:${name}`, JSON.stringify({ uri: uri.href }), false, () => fn(projectClient));
    return json(uri, out);
  };

  server.registerResource(
    "requirement",
    new ResourceTemplate("marreq://projects/{project_id}/requirements/{id}", { list: undefined }),
    { title: "Requirement", description: "One requirement with its trace summary.", mimeType: JSON_TYPE },
    (uri, vars) => read(uri, vars, "requirement", (c) => c.getRequirement(positiveInt(one(vars.id), "requirement id")))
  );

  server.registerResource(
    "use-case",
    new ResourceTemplate("marreq://projects/{project_id}/use-cases/{id}", { list: undefined }),
    { title: "Use case", description: "The current version of one use case.", mimeType: JSON_TYPE },
    (uri, vars) => read(uri, vars, "use-case", (c) => c.getUseCase(positiveInt(one(vars.id), "use case id")))
  );

  server.registerResource(
    "catalog",
    new ResourceTemplate("marreq://projects/{project_id}/catalog", { list: undefined }),
    {
      title: "Project catalog",
      description: "Statuses, categories, applicability, verification methods and users of the project.",
      mimeType: JSON_TYPE,
    },
    (uri, vars) => read(uri, vars, "catalog", (c) => c.listProjectCatalog())
  );

  server.registerResource(
    "guide",
    "marreq://guide",
    {
      title: "Marreq workflow guide",
      description: "How to review, create and modify requirements and use cases with the Marreq tools.",
      mimeType: "text/markdown",
    },
    async (uri) => ({ contents: [{ uri: uri.href, mimeType: "text/markdown", text: await guideText() }] })
  );
}
