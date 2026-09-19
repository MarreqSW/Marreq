import { createHash, randomUUID, timingSafeEqual } from "node:crypto";
import type { Server } from "node:http";
import type { Request, Response } from "express";
import { isInitializeRequest } from "@modelcontextprotocol/sdk/types.js";
import { createMcpExpressApp } from "@modelcontextprotocol/sdk/server/express.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { loadContext, type SessionContext } from "./context.js";

export interface RemoteTransportConfig {
  kind: "http";
  host: string;
  port: number;
  path: string;
  allowedHosts?: string[];
}

export type TransportConfig = RemoteTransportConfig | { kind: "stdio" };

interface RemoteSession {
  transport: StreamableHTTPServerTransport;
  server: McpServer;
  tokenBinding: Buffer;
}

function parsePort(raw: string | undefined): number {
  const port = Number(raw ?? "3000");
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error("MARREQ_MCP_PORT must be an integer from 1 to 65535");
  }
  return port;
}

function parsePath(raw: string | undefined): string {
  const path = raw?.trim() || "/mcp";
  if (!path.startsWith("/") || path.includes("?") || path.includes("#")) {
    throw new Error("MARREQ_MCP_PATH must be an absolute URL path");
  }
  return path;
}

export function loadTransportConfig(): TransportConfig {
  const raw = (process.env.MARREQ_MCP_TRANSPORT ?? "stdio").trim().toLowerCase();
  if (raw === "stdio") return { kind: "stdio" };
  if (raw !== "http") {
    throw new Error("MARREQ_MCP_TRANSPORT must be stdio or http");
  }
  const hosts = process.env.MARREQ_MCP_ALLOWED_HOSTS
    ?.split(",")
    .map((host) => host.trim())
    .filter(Boolean);
  return {
    kind: "http",
    host: process.env.MARREQ_MCP_HOST?.trim() || "127.0.0.1",
    port: parsePort(process.env.MARREQ_MCP_PORT),
    path: parsePath(process.env.MARREQ_MCP_PATH),
    allowedHosts: hosts?.length ? hosts : undefined,
  };
}

function bearerToken(header: string | undefined): string | undefined {
  if (!header) return undefined;
  const match = /^Bearer ([^\s]+)$/.exec(header);
  return match?.[1];
}

function bindToken(token: string): Buffer {
  return createHash("sha256").update(token).digest();
}

function sameBinding(left: Buffer, right: Buffer): boolean {
  return left.length === right.length && timingSafeEqual(left, right);
}

function jsonError(res: Response, status: number, message: string) {
  res.status(status).json({
    jsonrpc: "2.0",
    error: { code: status === 401 ? -32001 : -32000, message },
    id: null,
  });
}

export async function startRemoteServer(
  config: RemoteTransportConfig,
  createServer: (context: SessionContext) => McpServer
): Promise<Server> {
  const baseContext = loadContext({ apiTokenRequired: false });
  const app = createMcpExpressApp({
    host: config.host,
    allowedHosts: config.allowedHosts,
  });
  const sessions = new Map<string, RemoteSession>();

  const authenticate = (req: Request, res: Response): { token: string; binding: Buffer } | undefined => {
    const token = bearerToken(req.headers.authorization);
    if (!token) {
      res.setHeader("WWW-Authenticate", "Bearer");
      jsonError(res, 401, "Bearer authentication required");
      return undefined;
    }
    return { token, binding: bindToken(token) };
  };

  app.post(config.path, async (req, res) => {
    const auth = authenticate(req, res);
    if (!auth) return;
    const sessionId = req.headers["mcp-session-id"] as string | undefined;
    let session = sessionId ? sessions.get(sessionId) : undefined;

    if (sessionId && !session) {
      jsonError(res, 404, "Unknown or expired MCP session");
      return;
    }
    if (session && !sameBinding(session.tokenBinding, auth.binding)) {
      jsonError(res, 403, "MCP session does not belong to this credential");
      return;
    }
    if (!session) {
      if (!isInitializeRequest(req.body)) {
        jsonError(res, 400, "A valid MCP session is required");
        return;
      }
      const transport = new StreamableHTTPServerTransport({
        sessionIdGenerator: randomUUID,
        onsessioninitialized: (id) => {
          if (session) sessions.set(id, session);
        },
      });
      const server = createServer({
        ...baseContext,
        apiToken: auth.token,
        sessionId: undefined,
      });
      session = { transport, server, tokenBinding: auth.binding };
      transport.onclose = () => {
        const id = transport.sessionId;
        if (id) sessions.delete(id);
      };
      await server.connect(transport);
    }

    try {
      await session.transport.handleRequest(req, res, req.body);
    } catch {
      if (!res.headersSent) jsonError(res, 500, "MCP request failed");
    }
  });

  const existingSession = (req: Request, res: Response): RemoteSession | undefined => {
    const auth = authenticate(req, res);
    if (!auth) return undefined;
    const id = req.headers["mcp-session-id"] as string | undefined;
    const session = id ? sessions.get(id) : undefined;
    if (!session) {
      jsonError(res, 404, "Unknown or expired MCP session");
      return undefined;
    }
    if (!sameBinding(session.tokenBinding, auth.binding)) {
      jsonError(res, 403, "MCP session does not belong to this credential");
      return undefined;
    }
    return session;
  };

  app.get(config.path, async (req, res) => {
    const session = existingSession(req, res);
    if (session) await session.transport.handleRequest(req, res);
  });
  app.delete(config.path, async (req, res) => {
    const session = existingSession(req, res);
    if (session) await session.transport.handleRequest(req, res);
  });

  return await new Promise<Server>((resolve, reject) => {
    const listener = app.listen(config.port, config.host, () => resolve(listener));
    listener.on("error", reject);
  });
}
