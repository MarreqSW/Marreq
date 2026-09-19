import { createHash, randomUUID, timingSafeEqual } from "node:crypto";
import { isInitializeRequest } from "@modelcontextprotocol/sdk/types.js";
import { createMcpExpressApp } from "@modelcontextprotocol/sdk/server/express.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { loadContext } from "./context.js";
function parsePort(raw) {
    const port = Number(raw ?? "3000");
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
        throw new Error("MARREQ_MCP_PORT must be an integer from 1 to 65535");
    }
    return port;
}
function parsePath(raw) {
    const path = raw?.trim() || "/mcp";
    if (!path.startsWith("/") || path.includes("?") || path.includes("#")) {
        throw new Error("MARREQ_MCP_PATH must be an absolute URL path");
    }
    return path;
}
function parsePublicUrl(raw) {
    if (!raw)
        throw new Error("MARREQ_MCP_PUBLIC_URL must be set for HTTP transport");
    const url = new URL(raw);
    const local = url.protocol === "http:" && ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
    if ((url.protocol !== "https:" && !local) || url.search || url.hash || url.pathname.replace(/\/$/, "") !== "/mcp") {
        throw new Error("MARREQ_MCP_PUBLIC_URL must be an HTTPS /mcp URL (or localhost HTTP) without query or fragment");
    }
    return url.toString().replace(/\/$/, "");
}
export function loadTransportConfig() {
    const raw = (process.env.MARREQ_MCP_TRANSPORT ?? "stdio").trim().toLowerCase();
    if (raw === "stdio")
        return { kind: "stdio" };
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
        publicUrl: parsePublicUrl(process.env.MARREQ_MCP_PUBLIC_URL),
        allowedHosts: hosts?.length ? hosts : undefined,
    };
}
function bearerToken(header) {
    if (!header)
        return undefined;
    const match = /^Bearer ([^\s]+)$/.exec(header);
    return match?.[1];
}
function bindToken(token) {
    return createHash("sha256").update(token).digest();
}
function sameBinding(left, right) {
    return left.length === right.length && timingSafeEqual(left, right);
}
function jsonError(res, status, message) {
    res.status(status).json({
        jsonrpc: "2.0",
        error: { code: status === 401 ? -32001 : -32000, message },
        id: null,
    });
}
export async function startRemoteServer(config, createServer) {
    const baseContext = loadContext({ apiTokenRequired: false, projectRequired: false, remote: true });
    const app = createMcpExpressApp({
        host: config.host,
        allowedHosts: config.allowedHosts,
    });
    const sessions = new Map();
    const idleMs = config.sessionIdleMs ?? 30 * 60_000;
    const absoluteMs = config.sessionAbsoluteMs ?? 8 * 60 * 60_000;
    const maxSessions = config.maxSessions ?? 1_000;
    const expired = (session, now = Date.now()) => now - session.lastSeenAt > idleMs || now - session.createdAt > absoluteMs;
    const removeSession = async (id, session) => {
        sessions.delete(id);
        await session.transport.close().catch(() => undefined);
    };
    const cleanup = setInterval(() => {
        const now = Date.now();
        for (const [id, session] of sessions) {
            if (expired(session, now))
                void removeSession(id, session);
        }
    }, Math.max(1_000, Math.min(idleMs, 60_000)));
    cleanup.unref();
    const authenticate = (req, res) => {
        const token = bearerToken(req.headers.authorization);
        if (!token) {
            const publicUrl = new URL(config.publicUrl);
            const metadata = `${publicUrl.origin}/.well-known/oauth-protected-resource${publicUrl.pathname}`;
            res.setHeader("WWW-Authenticate", `Bearer resource_metadata="${metadata}"`);
            jsonError(res, 401, "Bearer authentication required");
            return undefined;
        }
        return { token, binding: bindToken(token) };
    };
    app.post(config.path, async (req, res) => {
        const auth = authenticate(req, res);
        if (!auth)
            return;
        const sessionId = req.headers["mcp-session-id"];
        let session = sessionId ? sessions.get(sessionId) : undefined;
        if (sessionId && session && expired(session)) {
            await removeSession(sessionId, session);
            session = undefined;
        }
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
            if (sessions.size >= maxSessions) {
                jsonError(res, 503, "MCP session capacity reached");
                return;
            }
            const transport = new StreamableHTTPServerTransport({
                sessionIdGenerator: randomUUID,
                onsessioninitialized: (id) => {
                    if (session)
                        sessions.set(id, session);
                },
            });
            const authenticatedClient = new (await import("./client.js")).MarreqClient({
                ...baseContext,
                apiToken: auth.token,
                projectId: 0,
                remote: true,
                mode: "draft_write",
                traceWrite: true,
            });
            try {
                await authenticatedClient.listProjects();
            }
            catch {
                jsonError(res, 401, "Bearer credential was rejected");
                return;
            }
            const server = createServer({
                ...baseContext,
                apiToken: auth.token,
                sessionId: undefined,
                projectId: 0,
                remote: true,
            });
            const now = Date.now();
            session = { transport, server, tokenBinding: auth.binding, createdAt: now, lastSeenAt: now };
            transport.onclose = () => {
                const id = transport.sessionId;
                if (id)
                    sessions.delete(id);
            };
            await server.connect(transport);
        }
        session.lastSeenAt = Date.now();
        try {
            await session.transport.handleRequest(req, res, req.body);
        }
        catch {
            if (!res.headersSent)
                jsonError(res, 500, "MCP request failed");
        }
    });
    const existingSession = (req, res) => {
        const auth = authenticate(req, res);
        if (!auth)
            return undefined;
        const id = req.headers["mcp-session-id"];
        let session = id ? sessions.get(id) : undefined;
        if (id && session && expired(session)) {
            void removeSession(id, session);
            session = undefined;
        }
        if (!session) {
            jsonError(res, 404, "Unknown or expired MCP session");
            return undefined;
        }
        if (!sameBinding(session.tokenBinding, auth.binding)) {
            jsonError(res, 403, "MCP session does not belong to this credential");
            return undefined;
        }
        session.lastSeenAt = Date.now();
        return session;
    };
    app.get(config.path, async (req, res) => {
        const session = existingSession(req, res);
        if (session)
            await session.transport.handleRequest(req, res);
    });
    app.delete(config.path, async (req, res) => {
        const session = existingSession(req, res);
        if (session)
            await session.transport.handleRequest(req, res);
    });
    return await new Promise((resolve, reject) => {
        const listener = app.listen(config.port, config.host, () => resolve(listener));
        listener.on("close", () => clearInterval(cleanup));
        listener.on("error", reject);
    });
}
