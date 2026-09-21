import { describe, expect, it } from "vitest";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createMarreqServer } from "./index.js";
import type { SessionContext } from "./context.js";

async function listToolAnnotations(ctx: SessionContext) {
  process.env.MARREQ_MODE = "draft_write";
  const server = createMarreqServer(ctx);
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: "annotation-test", version: "1.0.0" });
  await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);
  const tools = await client.listTools();
  await client.close();
  return Object.fromEntries(
    tools.tools.map((tool) => [tool.name, tool.annotations ?? {}])
  );
}

describe("create tool idempotentHint by transport", () => {
  const base: SessionContext = {
    baseUrl: "http://localhost:8000",
    apiToken: "test-token",
    projectId: 1,
    mode: "draft_write",
    traceWrite: false,
  };

  it("advertises idempotence for remote create tools only", async () => {
    const remote = await listToolAnnotations({
      ...base,
      remote: true,
      mcpPublicUrl: "http://localhost:3000/mcp",
    });
    const stdio = await listToolAnnotations({ ...base, remote: false });

    for (const name of [
      "create_requirement",
      "create_verification",
      "create_baseline",
      "create_requirement_comment",
    ]) {
      expect(remote[name]?.idempotentHint).toBe(true);
      expect(stdio[name]?.idempotentHint).toBe(false);
    }
    expect(remote.patch_requirement?.idempotentHint).toBe(false);
    expect(stdio.patch_requirement?.idempotentHint).toBe(false);
  });
});
