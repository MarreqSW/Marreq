#!/usr/bin/env python3
"""Validate real Rocket responses and requests recorded by the DB integration test.

No credentials are recorded. Inventory-only operations are counted separately.
"""
import argparse
import copy
import tempfile
from pathlib import Path
import json
import re
import sys

from jsonschema import Draft202012Validator, ValidationError
from compatibility import ROOT, ContractError, load, operations, resolve


def verify(path):
    spec = load(ROOT / "docs/developer/openapi.yaml")
    declared = operations(resolve(spec, spec), "server")
    records = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    checked, inventory_only, seen = 0, 0, set()
    for record in records:
        matches = [(p, op) for (p, m), op in declared.items() if m.upper() == record["method"] and re.fullmatch(re.sub(r"\{[^}]+\}", "[^/]+", re.escape(p).replace(r"\{", "{").replace(r"\}", "}")), record["path"].split("?")[0])]
        if matches:
            specificity = min(p.count("{") for p, _ in matches)
            matches = [(p, op) for p, op in matches if p.count("{") == specificity]
        if len(matches) != 1:
            raise ContractError(f"ambiguous or missing operation for {record['method']} {record['path']}")
        path_template, op = matches[0]
        if op.get("x-marreq-unverified"):
            inventory_only += 1
            continue
        status = str(record["status"])
        response = op["responses"].get(status, op["responses"].get("default"))
        if response is None:
            raise ContractError(f"{path_template}: undocumented actual status {status}")
        body = record["response"]
        schema = response.get("content", {}).get("application/json", {}).get("schema")
        if schema is not None:
            Draft202012Validator(schema).validate(body)
        if record.get("request") is not None and status.startswith("2"):
            request_schema = op.get("requestBody", {}).get("content", {}).get("application/json", {}).get("schema")
            if request_schema is None:
                raise ContractError(f"{path_template}: undocumented JSON request")
            Draft202012Validator(request_schema).validate(record["request"])
        checked += 1
        seen.add((record["method"], path_template, status))
    required = {("GET", "/api/meta/build", "200"), ("GET", "/api/meta/deployment", "200"), ("GET", "/api/projects/{project_id}/use-cases", "200"), ("POST", "/api/projects/{project_id}/use-cases", "201"), ("PUT", "/api/projects/{project_id}/use-cases/{id}/draft", "200"), ("GET", "/api/projects/{project_id}/use-cases", "403")}
    missing = required - seen
    if missing:
        raise ContractError(f"integration transcript lacks critical cases: {sorted(missing)}")
    return {"validated_exchanges": checked, "inventory_only_exchanges": inventory_only}


def negative_controls(path):
    """Demonstrate rejection using corrupted copies of actual backend traffic."""
    records = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    cases = []
    for label in ("removed guaranteed response property", "incompatible response type", "incompatible MCP request"):
        mutated = copy.deepcopy(records)
        if label == "incompatible MCP request":
            entry = next(r for r in mutated if r["method"] == "POST" and r["status"] == 201 and r["path"].endswith("/use-cases"))
            entry["request"]["title"] = 123
        else:
            entry = next(r for r in mutated if r["path"] == "/api/meta/build")
            if label == "removed guaranteed response property":
                del entry["response"]["backend_version"]
            else:
                entry["response"]["backend_version"] = 123
        with tempfile.TemporaryDirectory() as directory:
            candidate = Path(directory) / "negative.jsonl"
            candidate.write_text("\n".join(json.dumps(r) for r in mutated) + "\n")
            try:
                verify(candidate)
            except ValidationError:
                cases.append(label)
            else:
                raise ContractError(f"negative control was silently accepted: {label}")
    return cases


if __name__ == "__main__":
    from pathlib import Path
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("transcript", type=Path)
    args = parser.parse_args()
    result = verify(args.transcript)
    result["rejected_negative_controls"] = negative_controls(args.transcript)
    print(json.dumps(result, indent=2))
