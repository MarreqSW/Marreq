export function loadContext() {
    const baseUrl = process.env.REQMAN_BASE_URL;
    const apiToken = process.env.REQMAN_API_TOKEN;
    const projectId = process.env.REQMAN_PROJECT_ID;
    if (!baseUrl || !apiToken || !projectId) {
        throw new Error("REQMAN_BASE_URL, REQMAN_API_TOKEN, and REQMAN_PROJECT_ID must be set");
    }
    const mode = (process.env.REQMAN_MODE ?? "read_only");
    if (mode !== "read_only" && mode !== "draft_write") {
        throw new Error("REQMAN_MODE must be read_only or draft_write");
    }
    return {
        baseUrl: baseUrl.replace(/\/$/, ""),
        apiToken,
        projectId: parseInt(projectId, 10),
        userId: process.env.REQMAN_USER_ID
            ? parseInt(process.env.REQMAN_USER_ID, 10)
            : undefined,
        role: process.env.REQMAN_ROLE
            ? parseInt(process.env.REQMAN_ROLE, 10)
            : undefined,
        sessionId: process.env.REQMAN_SESSION_ID,
        mode,
    };
}
