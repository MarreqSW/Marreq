/**
 * List all requirements via ReqMan API (same as MCP list_requirements) and find duplicates.
 * Run with same env as MCP server: REQMAN_BASE_URL, REQMAN_API_TOKEN, REQMAN_PROJECT_ID.
 *
 * Usage: npx tsx src/check-duplicates.ts   OR   npm run build && node dist/check-duplicates.js
 */

import { loadContext } from "./context.js";
import { ReqManClient } from "./client.js";

interface Requirement {
  id: number;
  title: string;
  description: string;
  reference_code: string;
  project_id: number;
  approval_state?: string;
  [key: string]: unknown;
}

function normalize(s: string): string {
  return s
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

function main() {
  const ctx = loadContext();
  const client = new ReqManClient(ctx);

  client
    .listRequirements()
    .then((reqs: unknown) => {
      const list = reqs as Requirement[];
      const total = list.length;
      console.log(`Total requirements: ${total}\n`);

      // 1) Duplicates by reference_code (case-insensitive, trimmed)
      const byRef = new Map<string, Requirement[]>();
      for (const r of list) {
        const key = normalize(r.reference_code) || "(empty)";
        const list = byRef.get(key) ?? [];
        list.push(r);
        byRef.set(key, list);
      }
      const refDuplicates = [...byRef.entries()].filter(([, list]) => list.length > 1);

      // 2) Duplicates by title (case-insensitive, whitespace-normalized)
      const byTitle = new Map<string, Requirement[]>();
      for (const r of list) {
        const key = normalize(r.title) || "(empty)";
        const list = byTitle.get(key) ?? [];
        list.push(r);
        byTitle.set(key, list);
      }
      const titleDuplicates = [...byTitle.entries()].filter(([, list]) => list.length > 1);

      // 3) Duplicates by title + description (content duplicate)
      const byContent = new Map<string, Requirement[]>();
      for (const r of list) {
        const key = normalize(r.title) + "\n" + normalize(r.description);
        const list = byContent.get(key) ?? [];
        list.push(r);
        byContent.set(key, list);
      }
      const contentDuplicates = [...byContent.entries()].filter(([, list]) => list.length > 1);

      // Output
      console.log("=== Duplicates by reference_code ===");
      if (refDuplicates.length === 0) {
        console.log("None.");
      } else {
        for (const [ref, list] of refDuplicates) {
          console.log(`  "${ref}" (${list.length}):`);
          for (const r of list) {
            console.log(`    - id=${r.id}  title="${(r.title || "").slice(0, 50)}..."  ref="${r.reference_code}"`);
          }
          console.log("");
        }
      }

      console.log("=== Duplicates by title ===");
      if (titleDuplicates.length === 0) {
        console.log("None.");
      } else {
        for (const [title, list] of titleDuplicates) {
          console.log(`  "${title.slice(0, 60)}${title.length > 60 ? "..." : ""}" (${list.length}):`);
          for (const r of list) {
            console.log(`    - id=${r.id}  ref="${r.reference_code}"  title="${(r.title || "").slice(0, 40)}..."`);
          }
          console.log("");
        }
      }

      console.log("=== Duplicates by title+description (same content) ===");
      if (contentDuplicates.length === 0) {
        console.log("None.");
      } else {
        for (const [, list] of contentDuplicates) {
          const first = list[0];
          console.log(`  Title: "${(first.title || "").slice(0, 50)}..." (${list.length} reqs):`);
          for (const r of list) {
            console.log(`    - id=${r.id}  ref="${r.reference_code}"`);
          }
          console.log("");
        }
      }

      const refCount = refDuplicates.reduce((s, [, list]) => s + list.length, 0);
      const titleCount = titleDuplicates.reduce((s, [, list]) => s + list.length, 0);
      const contentCount = contentDuplicates.reduce((s, [, list]) => s + list.length, 0);
      console.log("--- Summary ---");
      console.log(`Reference code duplicate groups: ${refDuplicates.length} (${refCount} requirements)`);
      console.log(`Title duplicate groups: ${titleDuplicates.length} (${titleCount} requirements)`);
      console.log(`Title+description duplicate groups: ${contentDuplicates.length} (${contentCount} requirements)`);
    })
    .catch((err: Error) => {
      console.error("Error:", err.message);
      process.exit(1);
    });
}

main();
