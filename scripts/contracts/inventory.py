"""Source audit helpers, corroborated by Rocket's compiled route inventory.

This extracts route signatures, not Rust syntax or inferred serialization schemas.
The checked-in OpenAPI remains the authoritative wire contract.
"""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]


def source_routes(root=ROOT):
    routes = []
    for directory, variant in (("marreq-core/src/api", "shared"), ("marreq-cloud/src/api", "cloud"), ("marreq-server/src/api", "server")):
        for file in sorted((root / directory).glob("*.rs")):
            text = file.read_text()
            # Rocket route attributes always precede a free handler function.
            pattern = r'#\[(get|post|put|patch|delete)\(\s*"([^"]+)"(.*?)\)\]\s*pub (?:async )?fn (\w+)\s*\((.*?)\)\s*(?:->\s*(.*?))?\s*\{'
            for match in re.finditer(pattern, text, re.S):
                method, uri, attributes, name, arguments, returns = match.groups()
                base = "" if file.stem == "oauth" else "/api"
                path, _, query = uri.partition("?")
                path = base + re.sub(r"<([\w]+)(?:\.\.)?>", r"{\1}", path)
                guards = re.findall(r"\b(?:ApiUser(?:OrBearer)?|SessionUser|OptionalSessionUser|AdminOnly|Project[A-Z][A-Za-z]*|ProjectsRead|StdioMcpAuditAuth|McpAuditAuth|McpPrincipalAuth|InternalMcpAudit)\b", arguments)
                routes.append({"path": path, "method": method, "handler": f"{file.stem}::{name}", "source": str(file.relative_to(root)), "variant": variant, "arguments": arguments.strip(), "returns": (returns or "()").strip(), "query": query, "guards": guards, "data": re.findall(r'data\s*=\s*"<([^>]+)>"', attributes)})
    return routes
