/**
 * Unit tests for ReqManClient (read-only and Phase 2 draft_write methods).
 * Uses mocked fetch to assert correct URLs, methods, and bodies.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { ReqManClient } from "./client.js";
import type { SessionContext } from "./context.js";

const baseContext: SessionContext = {
  baseUrl: "http://localhost:8000",
  apiToken: "test-token",
  projectId: 1,
  mode: "read_only",
};

function makeClient(overrides?: Partial<SessionContext>): ReqManClient {
  return new ReqManClient({ ...baseContext, ...overrides });
}

describe("ReqManClient", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  describe("read-only methods", () => {
    it("getRequirement calls GET with project-scoped URL", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify({ id: 1, title: "R1" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const client = makeClient();
      await client.getRequirement(42);

      expect(fetchSpy).toHaveBeenCalledTimes(1);
      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/1/requirements/42",
        expect.objectContaining({
          headers: expect.objectContaining({
            Authorization: "Bearer test-token",
            "Content-Type": "application/json",
          }),
        })
      );
    });

    it("listRequirements calls GET with optional query params", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify([]), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const client = makeClient();
      await client.listRequirements("approved", true);

      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/1/requirements?approval_state=approved&has_tests=true",
        expect.any(Object)
      );
    });

    it("listRequirements with no filters omits query string", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify([]), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const client = makeClient();
      await client.listRequirements();

      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/1/requirements",
        expect.any(Object)
      );
    });
  });

  describe("Phase 2 draft_write methods", () => {
    it("createRequirement calls POST with project-scoped URL and body", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify({ status: "ok", id: 10 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const client = makeClient();
      const payload = {
        title: "New Req",
        description: "Desc",
        reference_code: "REQ-001",
        author_id: 1,
        reviewer_id: 1,
        category_id: 1,
        status_id: 1,
        applicability_id: 1,
        project_id: 1,
        verification_method_ids: [1],
      };
      await client.createRequirement(payload);

      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/1/requirements",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify(payload),
          headers: expect.objectContaining({
            Authorization: "Bearer test-token",
            "Content-Type": "application/json",
          }),
        })
      );
    });

    it("patchRequirement calls PATCH with project-scoped URL and body", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify({ success: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const client = makeClient();
      const patch = { title: "Updated Title" };
      await client.patchRequirement(5, patch);

      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/1/requirements/5",
        expect.objectContaining({
          method: "PATCH",
          body: JSON.stringify(patch),
        })
      );
    });

    it("setApproval calls PUT with project-scoped URL and state body", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(
          JSON.stringify({
            id: 1,
            requirement_id: 5,
            approval_state: "reviewed",
          }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      );

      const client = makeClient();
      await client.setApproval(5, 10, "reviewed");

      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/1/requirements/5/versions/10/approval",
        expect.objectContaining({
          method: "PUT",
          body: JSON.stringify({ state: "reviewed" }),
        })
      );
    });

    it("createBaseline calls POST with project-scoped URL and body", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(
          JSON.stringify({ id: 1, name: "Baseline 1", project_id: 1 }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }
        )
      );

      const client = makeClient();
      await client.createBaseline({
        name: "MCP Baseline",
        description: "From test",
      });

      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/1/baselines",
        expect.objectContaining({
          method: "POST",
          body: JSON.stringify({
            name: "MCP Baseline",
            description: "From test",
          }),
        })
      );
    });

    it("createBaseline with null description sends null", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify({ id: 1, name: "B" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const client = makeClient();
      await client.createBaseline({ name: "B", description: null });

      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/1/baselines",
        expect.objectContaining({
          body: JSON.stringify({ name: "B", description: null }),
        })
      );
    });
  });

  describe("projectId from context", () => {
    it("uses different project ID in URLs when context has projectId 2", async () => {
      const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify({ status: "ok", id: 1 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const client = makeClient({ projectId: 2 });
      await client.listRequirements();

      expect(fetchSpy).toHaveBeenCalledWith(
        "http://localhost:8000/api/projects/2/requirements",
        expect.any(Object)
      );
    });
  });

  describe("error handling", () => {
    it("throws when API returns non-ok", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response("Unauthorized", { status: 401 })
      );

      const client = makeClient();
      await expect(client.getRequirement(1)).rejects.toThrow(
        /ReqMan API 401/
      );
    });
  });
});
