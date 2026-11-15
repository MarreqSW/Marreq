import { readFileSync } from "node:fs";

const metadata = new URL("./build-info.json", import.meta.url);

// Tests and direct source consumers can use the development package identity.
function readBuildInfo(): { component: string; version: string; source_sha: string } {
  try {
    return JSON.parse(readFileSync(metadata, "utf8"));
  } catch (error) {
    if (!import.meta.url.endsWith(".ts") || (error as NodeJS.ErrnoException).code !== "ENOENT") {
      throw error;
    }
    const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
    return { component: "mcp", version: pkg.version, source_sha: "unknown" };
  }
}

export const buildInfo = Object.freeze(readBuildInfo());
