/**
 * MCP prompts (issue #416). Each prompt names the tools it expects, so a client
 * can plan the calls. They only describe work; no prompt writes anything.
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

const projectArg = z.string().regex(/^\d+$/).describe("Project id from list_projects");
const requirementArg = z.string().regex(/^\d+$/).describe("Requirement id");

function user(text: string) {
  return { messages: [{ role: "user" as const, content: { type: "text" as const, text } }] };
}

export const PROMPT_NAMES = [
  "review_requirement",
  "derive_children",
  "coverage_gap_analysis",
  "use_case_from_requirement",
] as const;

export function registerPrompts(server: McpServer) {
  server.registerPrompt(
    "review_requirement",
    {
      title: "Review a requirement",
      description: "Check one requirement for ambiguity, testability, duplicates, traceability and suspect links.",
      argsSchema: { project_id: projectArg, requirement_id: requirementArg },
    },
    ({ project_id, requirement_id }) =>
      user(
        `Review requirement ${requirement_id} in project ${project_id}; do not change anything.\n` +
          `Tools: get_requirement, get_versions, trace_up, trace_down, get_close_out (with requirement_id), ` +
          `list_linked_use_cases, semantic_search_requirements.\n` +
          `Report: is it atomic, unambiguous and testable; does a similar requirement already exist; does it have a parent ` +
          `and a passing verification; are any links suspect. Give findings first, then proposed edits for me to approve.`
      )
  );

  server.registerPrompt(
    "derive_children",
    {
      title: "Derive child requirements",
      description: "Propose atomic child requirements for a parent, without creating them.",
      argsSchema: { project_id: projectArg, requirement_id: requirementArg },
    },
    ({ project_id, requirement_id }) =>
      user(
        `Propose child requirements for requirement ${requirement_id} in project ${project_id}.\n` +
          `Tools: get_requirement, list_project_catalog, semantic_search_requirements, list_requirement_link_types.\n` +
          `Write each child as one atomic, testable statement, check it against existing requirements for overlap, ` +
          `and say which link type ties it to the parent. List them for my approval; create them with ` +
          `create_requirement (parent_links) only after I say so.`
      )
  );

  server.registerPrompt(
    "coverage_gap_analysis",
    {
      title: "Coverage gap analysis",
      description: "Find requirements without verification and tests made stale by changes.",
      argsSchema: { project_id: projectArg },
    },
    ({ project_id }) =>
      user(
        `Analyse verification coverage in project ${project_id}.\n` +
          `Tools: coverage_report, get_close_out, list_requirements (has_tests=false), impacted_tests, get_verification_matrix.\n` +
          `Rank the gaps by approval state and importance, list suspect tests, and suggest which verification would close ` +
          `each gap. Do not link or create anything until I choose.`
      )
  );

  server.registerPrompt(
    "use_case_from_requirement",
    {
      title: "Use case from a requirement",
      description: "Draft a use case that exercises a requirement, and link it.",
      argsSchema: { project_id: projectArg, requirement_id: requirementArg },
    },
    ({ project_id, requirement_id }) =>
      user(
        `Draft a use case for requirement ${requirement_id} in project ${project_id}.\n` +
          `Tools: get_requirement, list_use_cases, create_use_case, edit_use_case_draft, link_use_case_element.\n` +
          `Check list_use_cases for an existing one first. Outline the actor, preconditions, main steps and alternatives, ` +
          `show me the outline, then create the draft and link it to the requirement. Publish only if I ask.`
      )
  );
}
