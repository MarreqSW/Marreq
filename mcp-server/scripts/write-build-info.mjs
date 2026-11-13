import { readFileSync, writeFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const version = (process.env.MARREQ_VERSION || pkg.version).replace(/^v/, "");
const source_sha = process.env.MARREQ_GIT_SHA || "unknown";
writeFileSync(new URL("../dist/build-info.json", import.meta.url),
  JSON.stringify({ component: "mcp", version, source_sha }));
