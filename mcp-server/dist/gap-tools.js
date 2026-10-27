/**
 * MCP tools added for issue #416 (tool gaps). Each one wraps one project-scoped
 * backend route; none is a generic REST passthrough. Modes follow the PDR:
 * core = every mode, RX = read_extended and up, DW = draft_write, TW = the
 * MARREQ_TRACE_WRITE flag (same gate as put_verification_matrix).
 */
import { randomUUID } from "node:crypto";
import { z } from "zod";
import { contextAllowsReadExtended } from "./context.js";
import { paginate, paginationFields } from "./pagination.js";
const id = (what) => z.number().int().positive().describe(what);
export function registerGapTools(deps) {
    const { server, ctx, client, forProject, projectField, operationKey, withAudit, jsonContent } = deps;
    const add = (spec) => {
        const shape = {
            ...(spec.global ? {} : projectField),
            ...spec.shape,
            ...(spec.paged ? paginationFields : {}),
        };
        server.registerTool(spec.name, {
            description: spec.description,
            inputSchema: z.object(shape),
            ...(spec.destructive ? { annotations: { destructiveHint: true } } : {}),
        }, async (args) => {
            const toolClient = spec.global ? client : forProject(args?.project_id);
            const { idempotency_key: _key, ...audited } = args ?? {};
            const summary = spec.summary ? spec.summary(args ?? {}) : audited;
            const out = await withAudit(toolClient, spec.name, JSON.stringify(summary), Boolean(spec.write), () => spec.run(toolClient, args ?? {}));
            return { content: [jsonContent(spec.paged ? paginate(out, args ?? {}, 50) : out)] };
        });
    };
    // ----- Core: every mode -----
    add({
        name: "list_linked_use_cases",
        description: "Use cases linked to a requirement or to a verification, with the link ids and suspect flags. Give exactly one of requirement_id or verification_id.",
        shape: {
            requirement_id: id("Requirement id").optional(),
            verification_id: id("Verification id").optional(),
        },
        paged: true,
        run: (c, a) => {
            if ((a.requirement_id == null) === (a.verification_id == null)) {
                throw new Error("Give exactly one of requirement_id or verification_id");
            }
            return a.requirement_id != null
                ? c.listLinkedUseCases("requirement", a.requirement_id)
                : c.listLinkedUseCases("verification", a.verification_id);
        },
    });
    add({
        name: "ask_requirements",
        description: "Answer a question from the project's requirements (retrieval plus an LLM call, so it costs time and money; prefer semantic_search_requirements when you only need matches). Fails with a clear error when answer generation is disabled on the server.",
        shape: {
            query: z.string().min(1),
            k: z.number().int().min(1).max(20).optional().describe("Requirements used as context (default 10, max 20)."),
            status_filter: id("Only requirements with this status id").optional(),
            category_filter: id("Only requirements in this category id").optional(),
            applicability_filter: id("Only requirements with this applicability id").optional(),
            verification_filter: id("Only requirements with this verification method id").optional(),
        },
        summary: (a) => ({ query_length: String(a.query ?? "").length, k: a.k }),
        run: (c, a) => {
            const { project_id: _p, ...payload } = a;
            return c.askRequirements(payload);
        },
    });
    add({
        name: "list_use_case_versions",
        description: "Version history of a use case: version ids and numbers, approval states and change notes. Read a version with get_use_case(version_id).",
        shape: { use_case_id: id("Use case id") },
        paged: true,
        run: (c, a) => c.listUseCaseVersions(a.use_case_id),
    });
    add({
        name: "get_baseline_contents",
        description: "What a baseline recorded besides requirements: its use cases (kind=use_cases) or its verifications (kind=verifications).",
        shape: { baseline_id: id("Baseline id"), kind: z.enum(["use_cases", "verifications"]) },
        paged: true,
        run: (c, a) => c.getBaselineContents(a.baseline_id, a.kind),
    });
    // ----- RX: read_extended and up -----
    if (contextAllowsReadExtended(ctx)) {
        add({
            name: "list_requirement_link_types",
            description: "Link types allowed between requirement versions (for link_requirements).",
            shape: {},
            run: (c) => c.listRequirementLinkTypes(),
        });
        add({
            name: "list_requirement_links",
            description: "Requirement version links of the project, optionally filtered by source version, target version or link type. Use the link id with unlink_requirements.",
            shape: {
                source_version_id: id("Source requirement version id").optional(),
                target_version_id: id("Target requirement version id").optional(),
                link_type: z.string().min(1).optional(),
            },
            paged: true,
            run: (c, a) => c.listRequirementLinks({
                source_version_id: a.source_version_id,
                target_version_id: a.target_version_id,
                link_type: a.link_type,
            }),
        });
        add({
            name: "list_reviewers",
            description: "User ids designated as reviewers of the project.",
            shape: {},
            run: (c) => c.listReviewers(),
        });
        add({
            name: "get_my_permissions",
            description: "The authenticated user's effective permissions in the project. Check it before set_approval or other writes that may be refused.",
            shape: {},
            run: (c) => c.getMyPermissions(),
        });
        add({
            name: "list_saved_views",
            description: "Saved requirement views visible to the user. Pass a view id as view_id to list_requirements.",
            shape: {},
            paged: true,
            run: (c) => c.listSavedViews(),
        });
        add({
            name: "list_attachments",
            description: "Attachments of one requirement or verification (metadata only; files are not downloaded). Over OAuth it needs both requirements:read and verifications:read.",
            shape: { entity_type: z.enum(["requirement", "verification"]), entity_id: id("Requirement or verification id") },
            paged: true,
            run: (c, a) => c.listAttachments(a.entity_type, a.entity_id),
        });
        add({
            name: "get_attention",
            description: "What needs the user's attention across their projects: approvals waiting, drafts to review, suspect links, notifications. Not tied to one project.",
            shape: {},
            global: true,
            run: (c) => c.getAttention(),
        });
        add({
            name: "list_use_case_comments",
            description: "Comments on a use case, optionally for one version or one step/condition (element_id).",
            shape: {
                use_case_id: id("Use case id"),
                version_id: id("Only comments on this version").optional(),
                element_id: z.string().min(1).optional(),
            },
            paged: true,
            run: (c, a) => c.listUseCaseComments(a.use_case_id, { version_id: a.version_id, element_id: a.element_id }),
        });
        add({
            name: "get_close_out",
            description: "Close-out status of the project, or of one requirement when requirement_id is given: what lacks a passing verification.",
            shape: { requirement_id: id("Requirement id").optional() },
            run: (c, a) => c.getCloseOut(a.requirement_id),
        });
        add({
            name: "impacted_tests",
            description: "Verifications linked to a requirement that are marked suspect because the requirement changed. Run it after patch_requirement.",
            shape: { requirement_id: id("Requirement id") },
            paged: true,
            run: (c, a) => c.getImpactedTests(a.requirement_id),
        });
        add({
            name: "list_verification_snapshots",
            description: "Chronological snapshots of a verification, rebuilt from the audit log. Compare two with compare_verification_snapshots.",
            shape: { verification_id: id("Verification id") },
            paged: true,
            run: (c, a) => c.listVerificationSnapshots(a.verification_id),
        });
        add({
            name: "compare_verification_snapshots",
            description: "Structured diff between two snapshots of a verification (v1 = older, v2 = newer snapshot id).",
            shape: { verification_id: id("Verification id"), v1: id("Older snapshot id"), v2: id("Newer snapshot id") },
            run: (c, a) => c.compareVerificationSnapshots(a.verification_id, a.v1, a.v2),
        });
        add({
            name: "diff_baseline_verification_vs_current",
            description: "Diff of a verification as a baseline recorded it against the verification now.",
            shape: { baseline_id: id("Baseline id"), verification_id: id("Verification id") },
            run: (c, a) => c.diffBaselineVerificationVsCurrent(a.baseline_id, a.verification_id),
        });
    }
    // ----- DW: draft_write -----
    if (ctx.mode === "draft_write") {
        add({
            name: "unlink_use_case_element",
            description: "Delete one use case link (a requirement or verification linked to the use case or one of its steps). Get link ids from list_use_case_links. Deletes only the link, never the use case, requirement or verification.",
            shape: { use_case_id: id("Use case id"), link_id: id("Link id") },
            write: true,
            destructive: true,
            run: (c, a) => c.deleteUseCaseLink(a.use_case_id, a.link_id),
        });
        add({
            name: "clear_use_case_link_suspect",
            description: "Clear the suspect flag of a use case link after checking the linked requirement still fits the use case.",
            shape: { use_case_id: id("Use case id"), link_id: id("Link id") },
            write: true,
            run: (c, a) => c.clearUseCaseLinkSuspect(a.use_case_id, a.link_id),
        });
        add({
            name: "create_use_case_comment",
            description: "Comment on a use case, on one version (use_case_version_id) or one step/condition (element_id). @username mentions of project members notify them. Retrying with the same idempotency_key does not post twice.",
            shape: {
                idempotency_key: operationKey,
                use_case_id: id("Use case id"),
                body: z.string().min(1),
                use_case_version_id: id("Version the comment is about").optional(),
                element_id: z.string().min(1).optional(),
            },
            write: true,
            summary: (a) => ({
                use_case_id: a.use_case_id,
                body_len: String(a.body ?? "").length,
                use_case_version_id: a.use_case_version_id,
                element_id: a.element_id,
            }),
            run: (c, a) => c.createUseCaseComment(a.use_case_id, { body: a.body, use_case_version_id: a.use_case_version_id ?? null, element_id: a.element_id ?? null }, a.idempotency_key ?? randomUUID()),
        });
        add({
            name: "discard_use_case_draft",
            description: "Discard the shared draft of a use case (unpublished edits are lost; published versions stay). Read the draft first with get_use_case_diff to be sure it is stale.",
            shape: { use_case_id: id("Use case id") },
            write: true,
            destructive: true,
            run: (c, a) => c.discardUseCaseDraft(a.use_case_id),
        });
        add({
            name: "set_verification_field",
            description: "Change one field of a verification without resending the rest. Fields: name, description, source, status_id, reference_code, parent_id, verification_method_id, author_id, reviewer_id. Status changes keep reviewer enforcement.",
            shape: {
                verification_id: id("Verification id"),
                field: z.enum([
                    "name",
                    "description",
                    "source",
                    "status_id",
                    "reference_code",
                    "parent_id",
                    "verification_method_id",
                    "author_id",
                    "reviewer_id",
                ]),
                value: z.string().describe("New value; ids as digits. parent_id: empty or 0 clears it."),
            },
            write: true,
            run: (c, a) => c.setVerificationField(a.verification_id, a.field, a.value),
        });
    }
    // ----- TW: MARREQ_TRACE_WRITE -----
    if (ctx.traceWrite) {
        add({
            name: "link_requirements",
            description: "Link two requirement versions (source derives from / refines / satisfies / depends on / relates to target; see list_requirement_link_types). Both versions must be in the project; cycles are refused. Retrying with the same idempotency_key returns the same link.",
            shape: {
                idempotency_key: operationKey,
                source_version_id: id("Source requirement version id"),
                target_version_id: id("Target requirement version id"),
                link_type: z.string().min(1),
                rationale: z.string().optional(),
            },
            write: true,
            run: (c, a) => c.createRequirementLink({
                source_version_id: a.source_version_id,
                target_version_id: a.target_version_id,
                link_type: a.link_type,
                rationale: a.rationale ?? null,
            }, a.idempotency_key ?? randomUUID()),
        });
        add({
            name: "unlink_requirements",
            description: "Delete one requirement version link by id (from list_requirement_links). Deletes only the link; baselines taken earlier keep what they recorded.",
            shape: { link_id: id("Link id") },
            write: true,
            destructive: true,
            run: (c, a) => c.deleteRequirementLink(a.link_id),
        });
    }
}
