import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createServer, type Server } from "node:http";
import { once } from "node:events";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { createMarreqServer } from "./index.js";
import { loadTransportConfig, startRemoteServer } from "./remote.js";

const servers: Server[] = [];
let rejectApiCredential = false;

async function listen(server: Server): Promise<number> {
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  servers.push(server);
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("missing address");
  return address.port;
}

async function startApi(): Promise<number> {
  return listen(
    createServer((req, res) => {
      const token = req.headers.authorization?.replace(/^Bearer /, "");
      const known = ["valid-token", "refreshed-token", "different-token"].includes(token ?? "");
      if (!known || (rejectApiCredential && req.url !== "/api/mcp/principal")) {
        res.writeHead(401).end('{"error":"unauthorized"}');
        return;
      }
      res.setHeader("content-type", "application/json");
      if (req.url === "/api/mcp/principal") {
        const other = token === "different-token";
        res.end(JSON.stringify({
          user_id: other ? 2 : 1,
          authentication_type: "delegated_oauth",
          principal_id: other ? "grant:2" : "grant:1",
          client_id: "client-1",
          grant_id: other ? 2 : 1,
        }));
      } else if (req.url === "/api/projects/7/requirements/42") {
        res.end('{"id":42,"title":"Remote requirement"}');
      } else if (req.url === "/api/mcp/audit" && req.method === "POST") {
        res.end('{"status":"ok"}');
      } else {
        res.end("[]");
      }
    })
  );
}

async function startMcp(apiPort: number): Promise<{ port: number; url: URL }> {
  process.env.MARREQ_BASE_URL = `http://127.0.0.1:${apiPort}`;
  process.env.MARREQ_MCP_PUBLIC_URL = `http://127.0.0.1:${apiPort}/mcp`;
  delete process.env.MARREQ_PROJECT_ID;
  delete process.env.MARREQ_API_TOKEN;
  const server = await startRemoteServer(
    { kind: "http", host: "127.0.0.1", port: 0, path: "/mcp", publicUrl: process.env.MARREQ_MCP_PUBLIC_URL },
    createMarreqServer
  );
  servers.push(server);
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("missing address");
  return { port: address.port, url: new URL(`http://127.0.0.1:${address.port}/mcp`) };
}

function client(url: URL, token = "valid-token") {
  const transport = new StreamableHTTPClientTransport(url, {
    requestInit: { headers: { Authorization: `Bearer ${token}` } },
  });
  return {
    transport,
    client: new Client({ name: "remote-test", version: "1.0.0" }),
  };
}

async function rawRpc(url: URL, sessionId: string, token: string, body: object) {
  const response = await fetch(url, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/json, text/event-stream",
      "Content-Type": "application/json",
      "mcp-session-id": sessionId,
    },
    body: JSON.stringify(body),
  });
  const text = await response.text();
  const payload = response.headers.get("content-type")?.includes("text/event-stream")
    ? text.split("\n").find((line) => line.startsWith("data: "))?.slice(6)
    : text;
  return { response, json: JSON.parse(payload ?? "null") };
}

describe("remote Streamable HTTP transport", () => {
  beforeEach(() => {
    process.env.MARREQ_MODE = "read_only";
    rejectApiCredential = false;
  });

  afterEach(async () => {
    await Promise.all(
      servers.splice(0).map(
        (server) => new Promise<void>((resolve) => server.close(() => resolve()))
      )
    );
  });

  it("keeps stdio as the default transport", () => {
    delete process.env.MARREQ_MCP_TRANSPORT;
    expect(loadTransportConfig()).toEqual({ kind: "stdio" });
  });

  it("initializes, discovers tools, and performs a read", async () => {
    const apiPort = await startApi();
    const { url } = await startMcp(apiPort);
    const remote = client(url);
    await remote.client.connect(remote.transport);

    const tools = await remote.client.listTools();
    expect(tools.tools.map((tool) => tool.name)).toContain("list_projects");
    expect(tools.tools.map((tool) => tool.name)).toContain("get_requirement");
    expect(tools.tools.map((tool) => tool.name)).toContain("create_requirement");
    expect(tools.tools.map((tool) => tool.name)).toContain("put_verification_matrix");
    const wire = await rawRpc(url, remote.transport.sessionId!, "valid-token", {
      jsonrpc: "2.0", id: 50, method: "tools/list",
    });
    const wireTools = wire.json.result.tools as Array<Record<string, unknown>>;
    const requirement = wireTools.find((tool) => tool.name === "get_requirement");
    expect(requirement?.securitySchemes).toEqual([
      { type: "oauth2", scopes: ["requirements:read"] },
    ]);
    expect((requirement?._meta as Record<string, unknown> | undefined)?.securitySchemes).toBeUndefined();
    const composite = wireTools.find((tool) => tool.name === "diff_baseline_vs_current");
    expect(composite?.securitySchemes).toEqual([
      { type: "oauth2", scopes: ["requirements:read", "baselines:read"] },
    ]);
    const result = await remote.client.callTool({
      name: "get_requirement",
      arguments: { project_id: 7, requirement_id: "42" },
    });
    expect(JSON.stringify(result.content)).toContain("Remote requirement");
    await remote.transport.terminateSession();
  });

  it("rejects unauthenticated and unknown-session requests", async () => {
    const apiPort = await startApi();
    const { url } = await startMcp(apiPort);
    const unauthenticated = await fetch(url, { method: "POST", body: "{}" });
    expect(unauthenticated.status).toBe(401);
    expect(unauthenticated.headers.get("www-authenticate")).toBe(
      `Bearer resource_metadata="http://127.0.0.1:${apiPort}/.well-known/oauth-protected-resource/mcp"`
    );
    const malformed = await fetch(url, {
      method: "POST",
      headers: { Authorization: "Basic not-bearer" },
      body: "{}",
    });
    expect(malformed.status).toBe(401);
    const unknown = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: "Bearer valid-token",
        "Content-Type": "application/json",
        "mcp-session-id": "not-a-session",
      },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "tools/list" }),
    });
    expect(unknown.status).toBe(404);
  });

  it("does not read local mode flags for the remote tool surface", async () => {
    process.env.MARREQ_MODE = "not-a-mode";
    process.env.MARREQ_TRACE_WRITE = "false";
    const apiPort = await startApi();
    const { url } = await startMcp(apiPort);
    const remote = client(url);
    await remote.client.connect(remote.transport);
    const names = (await remote.client.listTools()).tools.map((tool) => tool.name);
    expect(names).toContain("create_requirement");
    expect(names).toContain("put_verification_matrix");
    await remote.transport.terminateSession();
  });

  it("keeps concurrent sessions independent", async () => {
    const apiPort = await startApi();
    const { url } = await startMcp(apiPort);
    const first = client(url);
    const second = client(url);
    await Promise.all([
      first.client.connect(first.transport),
      second.client.connect(second.transport),
    ]);
    expect(first.transport.sessionId).toBeTruthy();
    expect(second.transport.sessionId).toBeTruthy();
    expect(first.transport.sessionId).not.toBe(second.transport.sessionId);
    await Promise.all([
      first.transport.terminateSession(),
      second.transport.terminateSession(),
    ]);
  });

  it("does not allow another bearer credential to assume a session", async () => {
    const apiPort = await startApi();
    const { url } = await startMcp(apiPort);
    const owner = client(url);
    await owner.client.connect(owner.transport);
    const response = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: "Bearer different-token",
        "Content-Type": "application/json",
        "mcp-session-id": owner.transport.sessionId!,
      },
      body: JSON.stringify({ jsonrpc: "2.0", id: 2, method: "tools/list" }),
    });
    expect(response.status).toBe(403);
    await owner.transport.terminateSession();
  });

  it("continues a session with a refreshed token from the same OAuth grant", async () => {
    const apiPort = await startApi();
    const { url } = await startMcp(apiPort);
    const owner = client(url);
    await owner.client.connect(owner.transport);
    const response = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: "Bearer refreshed-token",
        Accept: "application/json, text/event-stream",
        "Content-Type": "application/json",
        "mcp-session-id": owner.transport.sessionId!,
      },
      body: JSON.stringify({ jsonrpc: "2.0", id: 3, method: "tools/list" }),
    });
    expect(response.status).toBe(200);
    await owner.transport.terminateSession();
  });

  it("returns an MCP reauthorization challenge when a credential expires", async () => {
    const apiPort = await startApi();
    const { url } = await startMcp(apiPort);
    const remote = client(url);
    await remote.client.connect(remote.transport);
    rejectApiCredential = true;
    const result = await remote.client.callTool({
      name: "get_requirement",
      arguments: { project_id: 7, requirement_id: "42" },
    });
    expect(result.isError).toBe(true);
    expect(result._meta?.["mcp/www_authenticate"]).toBe(
      `Bearer resource_metadata="http://127.0.0.1:${apiPort}/.well-known/oauth-protected-resource/mcp", error="invalid_token", error_description="The access token is invalid or expired"`
    );
    await remote.transport.terminateSession();
  });
});
