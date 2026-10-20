/**
 * Tool gaps (issue #416): client URLs and bodies, which mode registers each
 * tool, annotations, audit flags, pagination, resources and prompts.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { MarreqClient } from "./client.js";
import { createMarreqServer } from "./index.js";
import { paginate } from "./pagination.js";
import type { SessionContext } from "./context.js";

const ctx: SessionContext = {
  baseUrl: "http://localhost:8000",
  apiToken: "test-token",
  projectId: 1,
  mode: "draft_write",
  traceWrite: true,
};

function respond(body: unknown, status = 200) {
  return vi.spyOn(globalThis, "fetch").mockImplementation(async () =>
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    })
  );
}

function call(spy: ReturnType<typeof respond>, i = 0) {
  const [url, init] = spy.mock.calls[i] as [string, RequestInit];
  return {
    url,
    method: init?.method ?? "GET",
    body: init?.body ? JSON.parse(String(init.body)) : undefined,
    headers: (init?.headers ?? {}) as Record<string, string>,
  };
}

const API = "http://localhost:8000/api/projects/1";

describe("client methods", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("keeps the old list_requirements URL and adds the new filters after it", async () => {
    const spy = respond([]);
    const client = new MarreqClient(ctx);
    await client.listRequirements("draft", true);
    await client.listRequirements(undefined, undefined, { status_id: 2, category_id: 3, q: "link budget", sort_column: "title", sort_dir: "desc", view_id: 9 });
    await client.listRequirements();
    expect(call(spy, 0).url).toBe(`${API}/requirements?approval_state=draft&has_tests=true`);
    expect(call(spy, 1).url).toBe(
      `${API}/requirements?status_id=2&category_id=3&q=link+budget&sort_column=title&sort_dir=desc&view_id=9`
    );
    expect(call(spy, 2).url).toBe(`${API}/requirements`);
  });

  it("reads and writes requirement links", async () => {
    const spy = respond({});
    const client = new MarreqClient(ctx);
    await client.listRequirementLinkTypes();
    await client.listRequirementLinks({ source_version_id: 4, link_type: "DERIVES_FROM" });
    await client.createRequirementLink({ source_version_id: 4, target_version_id: 5, link_type: "DERIVES_FROM" }, "key-link-0000000001");
    await client.deleteRequirementLink(12);
    expect(call(spy, 0).url).toBe(`${API}/requirement-version-links/link-types`);
    expect(call(spy, 1).url).toBe(`${API}/requirement-version-links?source_version_id=4&link_type=DERIVES_FROM`);
    expect(call(spy, 2)).toMatchObject({
      method: "POST",
      url: `${API}/requirement-version-links`,
      body: { source_version_id: 4, target_version_id: 5, link_type: "DERIVES_FROM" },
      headers: { "Idempotency-Key": "key-link-0000000001" },
    });
    expect(call(spy, 3)).toMatchObject({ method: "DELETE", url: `${API}/requirement-version-links/12` });
  });

  it("calls the use case, verification and baseline routes", async () => {
    const spy = respond({});
    const client = new MarreqClient(ctx);
    await client.deleteUseCaseLink(7, 3);
    await client.clearUseCaseLinkSuspect(7, 3);
    await client.listLinkedUseCases("requirement", 11);
    await client.listLinkedUseCases("verification", 21);
    await client.listUseCaseVersions(7);
    await client.listUseCaseComments(7, { version_id: 31, element_id: "s_A1x9" });
    await client.createUseCaseComment(7, { body: "hi", element_id: "s_A1x9" }, "key-comment-00000001");
    await client.discardUseCaseDraft(7);
    expect(call(spy, 0)).toMatchObject({ method: "DELETE", url: `${API}/use-cases/7/links/3` });
    expect(call(spy, 1)).toMatchObject({ method: "POST", url: `${API}/use-cases/7/links/3/clear_suspect` });
    expect(call(spy, 2).url).toBe(`${API}/requirements/11/use-cases`);
    expect(call(spy, 3).url).toBe(`${API}/verifications/21/use-cases`);
    expect(call(spy, 4).url).toBe(`${API}/use-cases/7/versions`);
    expect(call(spy, 5).url).toBe(`${API}/use-cases/7/comments?version_id=31&element_id=s_A1x9`);
    expect(call(spy, 6)).toMatchObject({
      method: "POST",
      body: { body: "hi", element_id: "s_A1x9" },
      headers: { "Idempotency-Key": "key-comment-00000001" },
    });
    expect(call(spy, 7)).toMatchObject({ method: "DELETE", url: `${API}/use-cases/7/draft` });
  });

  it("calls the verification, close-out and baseline routes", async () => {
    const spy = respond({});
    const client = new MarreqClient(ctx);
    await client.getCloseOut();
    await client.getCloseOut(5);
    await client.getImpactedTests(5);
    await client.listVerificationSnapshots(2);
    await client.compareVerificationSnapshots(2, 10, 11);
    await client.setVerificationField(2, "status_id", "3");
    await client.getBaselineContents(8, "use_cases");
    await client.getBaselineContents(8, "verifications");
    await client.diffBaselineVerificationVsCurrent(8, 2);
    await client.askRequirements({ query: "what?", k: 5 });
    await client.listReviewers();
    await client.getMyPermissions();
    await client.listSavedViews();
    await client.listAttachments("verification", 2);
    await client.getAttention();
    const urls = spy.mock.calls.map(([url]) => String(url).replace("http://localhost:8000", ""));
    expect(urls).toEqual([
      "/api/projects/1/close_out",
      "/api/projects/1/requirements/5/close_out",
      "/api/projects/1/requirements/5/impacted_tests",
      "/api/projects/1/verifications/2/snapshots",
      "/api/projects/1/verifications/2/snapshots/10/diff/11",
      "/api/projects/1/verifications/2/field",
      "/api/projects/1/baselines/8/use-cases",
      "/api/projects/1/baselines/8/verifications",
      "/api/projects/1/baselines/8/verifications/2/diff/current",
      "/api/projects/1/requirements/ask",
      "/api/projects/1/reviewers",
      "/api/projects/1/me/permissions",
      "/api/projects/1/saved_views",
      "/api/projects/1/attachments?entity_type=verification&entity_id=2",
      "/api/home/attention",
    ]);
    expect(call(spy, 5)).toMatchObject({ method: "POST", body: { field: "status_id", value: "3" } });
    expect(call(spy, 9)).toMatchObject({ method: "POST", body: { query: "what?", k: 5 } });
  });
});

async function connect(options: Partial<SessionContext> = {}) {
  const server = createMarreqServer({ ...ctx, ...options });
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: "gap-test", version: "1.0.0" });
  await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);
  return client;
}

async function tools(options: Partial<SessionContext> = {}) {
  const client = await connect(options);
  const list = (await client.listTools()).tools;
  await client.close();
  return list;
}

const CORE = ["list_linked_use_cases", "ask_requirements", "list_use_case_versions", "get_baseline_contents"];
const RX = [
  "list_requirement_link_types", "list_requirement_links", "list_reviewers", "get_my_permissions",
  "list_saved_views", "list_attachments", "get_attention", "list_use_case_comments", "get_close_out",
  "impacted_tests", "list_verification_snapshots", "compare_verification_snapshots",
  "diff_baseline_verification_vs_current",
];
const DW = ["unlink_use_case_element", "clear_use_case_link_suspect", "create_use_case_comment", "discard_use_case_draft", "set_verification_field"];
const TW = ["link_requirements", "unlink_requirements"];

describe("tool registration per mode", () => {
  it("registers each tool at its documented mode", async () => {
    const readOnly = (await tools({ mode: "read_only", traceWrite: false })).map((t) => t.name);
    const extended = (await tools({ mode: "read_extended", traceWrite: false })).map((t) => t.name);
    const drafting = (await tools({ mode: "draft_write", traceWrite: false })).map((t) => t.name);
    const tracing = (await tools({ mode: "draft_write", traceWrite: true })).map((t) => t.name);

    for (const name of CORE) for (const names of [readOnly, extended, drafting]) expect(names).toContain(name);
    for (const name of RX) {
      expect(readOnly).not.toContain(name);
      expect(extended).toContain(name);
    }
    for (const name of [...DW, ...TW]) {
      expect(readOnly).not.toContain(name);
      expect(extended).not.toContain(name);
    }
    for (const name of DW) expect(drafting).toContain(name);
    for (const name of TW) {
      expect(drafting).not.toContain(name);
      expect(tracing).toContain(name);
    }
  });

  it("offers the 24 new tools and no deletion of requirements, verifications or use cases", async () => {
    const all = (await tools()).map((t) => t.name);
    for (const name of [...CORE, ...RX, ...DW, ...TW]) expect(all).toContain(name);
    expect([...CORE, ...RX, ...DW, ...TW]).toHaveLength(24);
    expect(all.filter((n) => /^(delete|remove)_/.test(n))).toEqual([]);
    expect(all).not.toContain("unlink_verification");
  });

  it("marks writes, deletions and reads with the right hints", async () => {
    const byName = Object.fromEntries((await tools()).map((t) => [t.name, t.annotations ?? {}]));
    for (const name of ["unlink_requirements", "unlink_use_case_element", "discard_use_case_draft"]) {
      expect(byName[name]).toMatchObject({ readOnlyHint: false, destructiveHint: true });
    }
    for (const name of ["link_requirements", "create_use_case_comment", "clear_use_case_link_suspect", "set_verification_field"]) {
      expect(byName[name]).toMatchObject({ readOnlyHint: false, destructiveHint: false });
    }
    for (const name of [...CORE, ...RX]) {
      expect(byName[name]).toMatchObject({ readOnlyHint: true, destructiveHint: false });
    }
  });
});

describe("tool calls", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("audits a write with is_write and sends only the changed field", async () => {
    const client = await connect();
    const spy = respond({ success: true });
    const out = await client.callTool({
      name: "set_verification_field",
      arguments: { verification_id: 2, field: "status_id", value: "3" },
    });
    expect(out.isError).toBeFalsy();
    const calls = spy.mock.calls.map(([url, init]) => ({ url: String(url), init: init as RequestInit }));
    const field = calls.find((c) => c.url.endsWith("/verifications/2/field"))!;
    expect(JSON.parse(String(field.init.body))).toEqual({ field: "status_id", value: "3" });
    const audit = calls.find((c) => c.url.endsWith("/api/mcp/audit"))!;
    expect(JSON.parse(String(audit.init.body))).toMatchObject({ tool_name: "set_verification_field", is_write: true });
    await client.close();
  });

  it("refuses link ids and requires exactly one id for list_linked_use_cases", async () => {
    const client = await connect();
    respond([]);
    const both = await client.callTool({ name: "list_linked_use_cases", arguments: { requirement_id: 1, verification_id: 2 } });
    expect(both.isError).toBe(true);
    const none = await client.callTool({ name: "list_linked_use_cases", arguments: {} });
    expect(none.isError).toBe(true);
    await client.close();
  });

  it("pages a new list tool from the start and keeps old tools unpaged", async () => {
    const client = await connect();
    const rows = Array.from({ length: 120 }, (_, i) => ({ id: i + 1 }));
    respond(rows);
    const first = await client.callTool({ name: "list_use_case_versions", arguments: { use_case_id: 7 } });
    const page = JSON.parse((first.content as Array<{ text: string }>)[0].text);
    expect(page.items).toHaveLength(50);
    expect(page.total).toBe(120);
    const second = await client.callTool({
      name: "list_use_case_versions",
      arguments: { use_case_id: 7, cursor: page.next_cursor, limit: 100 },
    });
    const next = JSON.parse((second.content as Array<{ text: string }>)[0].text);
    expect(next.items[0].id).toBe(51);
    expect(next.items).toHaveLength(70);
    expect(next.next_cursor).toBeNull();

    const old = await client.callTool({ name: "list_baselines", arguments: {} });
    expect(JSON.parse((old.content as Array<{ text: string }>)[0].text)).toHaveLength(120);
    const oldPaged = await client.callTool({ name: "list_baselines", arguments: { limit: 5 } });
    expect(JSON.parse((oldPaged.content as Array<{ text: string }>)[0].text).items).toHaveLength(5);
    await client.close();
  });
});

describe("paginate", () => {
  const rows = Array.from({ length: 300 }, (_, i) => i);

  it("walks 300 rows in 50-row pages with no gaps or repeats", () => {
    const seen: number[] = [];
    let cursor: string | undefined;
    let pages = 0;
    do {
      const page = paginate(rows, { limit: 50, cursor }) as { items: number[]; next_cursor: string | null };
      seen.push(...page.items);
      cursor = page.next_cursor ?? undefined;
      pages++;
    } while (cursor);
    expect(pages).toBe(6);
    expect(seen).toEqual(rows);
  });

  it("leaves non-lists alone, caps the page size and rejects a forged cursor", () => {
    expect(paginate({ a: 1 }, { limit: 5 })).toEqual({ a: 1 });
    expect((paginate(rows, { limit: 5000 }) as { items: number[] }).items).toHaveLength(200);
    expect(() => paginate(rows, { cursor: "garbage" })).toThrow(/Invalid cursor/);
  });
});

describe("resources and prompts", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("lists the resource templates and the guide, and reads them", async () => {
    const client = await connect();
    const templates = (await client.listResourceTemplates()).resourceTemplates.map((t) => t.uriTemplate);
    expect(templates).toEqual(
      expect.arrayContaining([
        "marreq://projects/{project_id}/requirements/{id}",
        "marreq://projects/{project_id}/use-cases/{id}",
        "marreq://projects/{project_id}/catalog",
      ])
    );
    expect((await client.listResources()).resources.map((r) => r.uri)).toContain("marreq://guide");

    const guide = await client.readResource({ uri: "marreq://guide" });
    expect((guide.contents[0] as { text: string }).text).toContain("Marreq requirements-engineering workflow");

    const spy = respond({ id: 42, title: "Link budget" });
    const requirement = await client.readResource({ uri: "marreq://projects/3/requirements/42" });
    expect(String(spy.mock.calls[0][0])).toBe("http://localhost:8000/api/projects/3/requirements/42");
    expect(JSON.parse((requirement.contents[0] as { text: string }).text)).toMatchObject({ id: 42 });
    await expect(client.readResource({ uri: "marreq://projects/x/requirements/42" })).rejects.toThrow();
    await client.close();
  });

  it("serves four prompts that name the tools they expect", async () => {
    const client = await connect();
    const names = (await client.listPrompts()).prompts.map((p) => p.name);
    expect(names).toEqual(
      expect.arrayContaining(["review_requirement", "derive_children", "coverage_gap_analysis", "use_case_from_requirement"])
    );
    const all = await client.listTools();
    const toolNames = new Set(all.tools.map((t) => t.name));
    for (const [name, args] of [
      ["review_requirement", { project_id: "1", requirement_id: "42" }],
      ["derive_children", { project_id: "1", requirement_id: "42" }],
      ["coverage_gap_analysis", { project_id: "1" }],
      ["use_case_from_requirement", { project_id: "1", requirement_id: "42" }],
    ] as const) {
      const prompt = await client.getPrompt({ name, arguments: args });
      const text = (prompt.messages[0].content as { text: string }).text;
      const line = text.split("\n").find((l) => l.startsWith("Tools:"))!;
      const mentioned = line.slice("Tools:".length).split(",").map((t) => t.trim().replace(/ \(.*\)\.?$/, "").replace(/\.$/, ""));
      expect(mentioned.length).toBeGreaterThan(0);
      for (const tool of mentioned) expect(toolNames, `${name} mentions ${tool}`).toContain(tool);
    }
    await client.close();
  });
});
