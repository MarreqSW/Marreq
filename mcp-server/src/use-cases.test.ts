/**
 * Use case client methods and MCP tools (issue #402): URLs, bodies,
 * idempotency keys, and which tools each mode registers.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { MarreqClient } from "./client.js";
import { createMarreqServer } from "./index.js";
import type { SessionContext } from "./context.js";

const ctx: SessionContext = {
  baseUrl: "http://localhost:8000",
  apiToken: "test-token",
  projectId: 1,
  mode: "draft_write",
  traceWrite: false,
};

function respond(body: unknown, status = 200) {
  // A new Response per call: a body can be read once.
  return vi.spyOn(globalThis, "fetch").mockImplementation(async () =>
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    })
  );
}

function call(spy: ReturnType<typeof respond>, i = 0) {
  const [url, init] = spy.mock.calls[i] as [string, RequestInit];
  return { url, method: init.method ?? "GET", body: init.body ? JSON.parse(String(init.body)) : undefined, headers: init.headers as Record<string, string> };
}

describe("use case client methods", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("lists with filters and reads a version", async () => {
    const spy = respond([]);
    const client = new MarreqClient(ctx);
    await client.listUseCases({ q: "export", has_suspect: true });
    await client.getUseCase(7, 31);
    expect(call(spy, 0).url).toBe("http://localhost:8000/api/projects/1/use-cases?q=export&has_suspect=true");
    expect(call(spy, 1).url).toBe("http://localhost:8000/api/projects/1/use-cases/7?version_id=31");
  });

  it("returns null for a missing draft and picks the diff", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(new Response("{}", { status: 404 }));
    const client = new MarreqClient(ctx);
    expect(await client.getUseCaseDraft(7)).toBeNull();
    const spy = respond({});
    await client.getUseCaseDiff(7);
    await client.getUseCaseDiff(7, 31, 32);
    // The spy also recorded the draft request.
    expect(call(spy, 1).url).toBe("http://localhost:8000/api/projects/1/use-cases/7/draft/diff");
    expect(call(spy, 2).url).toBe("http://localhost:8000/api/projects/1/use-cases/7/versions/31/diff/32");
  });

  it("writes with revisions and idempotency keys", async () => {
    const spy = respond({});
    const client = new MarreqClient(ctx);
    await client.createUseCase({ title: "Export requirements" }, "key-create-0000001");
    await client.applyUseCaseOperations(7, null, [{ op: "update_text", id: "s_A1x9", text: "x" }]);
    await client.putUseCaseDraft(7, 3, { title: "T" });
    await client.publishUseCaseDraft(7, 4, "First", "key-publish-000001");
    await client.linkUseCaseElement(7, { element_id: "s_A1x9", requirement_id: 11 });

    expect(call(spy, 0)).toMatchObject({ method: "POST", body: { title: "Export requirements" }, headers: { "Idempotency-Key": "key-create-0000001" } });
    expect(call(spy, 1)).toMatchObject({
      url: "http://localhost:8000/api/projects/1/use-cases/7/draft/operations",
      body: { base_revision: null, operations: [{ op: "update_text", id: "s_A1x9", text: "x" }] },
    });
    expect(call(spy, 2)).toMatchObject({ method: "PUT", body: { base_revision: 3, document: { title: "T" } } });
    expect(call(spy, 3)).toMatchObject({
      url: "http://localhost:8000/api/projects/1/use-cases/7/draft/publish",
      body: { base_revision: 4, change_note: "First" },
      headers: { "Idempotency-Key": "key-publish-000001" },
    });
    expect(call(spy, 4)).toMatchObject({ url: "http://localhost:8000/api/projects/1/use-cases/7/links", body: { element_id: "s_A1x9", requirement_id: 11 } });
  });
});

async function toolNames(mode: SessionContext["mode"]) {
  const server = createMarreqServer({ ...ctx, mode });
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: "use-case-test", version: "1.0.0" });
  await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);
  const tools = await client.listTools();
  await client.close();
  return tools.tools;
}

describe("use case MCP tools", () => {
  const read = ["list_use_cases", "get_use_case", "get_use_case_diff", "list_use_case_links"];
  const write = ["create_use_case", "edit_use_case_draft", "publish_use_case_draft", "link_use_case_element"];

  it("offers reading in every mode and writing in draft_write only, never approval", async () => {
    const readOnly = (await toolNames("read_only")).map((t) => t.name);
    const drafting = (await toolNames("draft_write")).map((t) => t.name);
    for (const name of read) {
      expect(readOnly).toContain(name);
    }
    for (const name of write) {
      expect(readOnly).not.toContain(name);
      expect(drafting).toContain(name);
    }
    expect(drafting.filter((n) => n.includes("use_case") && n.includes("approv"))).toEqual([]);
  });

  it("edits with operations through the tool and refuses both or neither", async () => {
    const server = createMarreqServer(ctx);
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    const client = new Client({ name: "use-case-test", version: "1.0.0" });
    await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);
    const spy = respond({ revision: 2 });
    const out = await client.callTool({
      name: "edit_use_case_draft",
      arguments: { use_case_id: 7, base_revision: 1, operations: [{ op: "remove_branch", id: "b_N0s1" }] },
    });
    expect(out.isError).toBeFalsy();
    const edit = spy.mock.calls
      .map(([url, init]) => ({ url: String(url), init: init as RequestInit }))
      .find((c) => c.url.endsWith("/draft/operations"));
    expect(JSON.parse(String(edit!.init.body))).toEqual({ base_revision: 1, operations: [{ op: "remove_branch", id: "b_N0s1" }] });
    const bad = await client.callTool({ name: "edit_use_case_draft", arguments: { use_case_id: 7, base_revision: 1 } });
    expect(bad.isError).toBe(true);
    await client.close();
  });
});
