#!/usr/bin/env python3
"""Validate OpenAPI, route coverage, compatibility metadata, and review evidence."""
import argparse
import hashlib
import json
import sys

from openapi_spec_validator import validate
from compatibility import ROOT, ContractError, load, policy, version, resolve
from generate import generated
from inventory import source_routes


def source_evidence():
    files = set()
    for directory in ("marreq-core/src", "marreq-server/src", "marreq-cloud/src", "frontend/src/api", "mcp-server/src"):
        files.update(file for file in (ROOT / directory).rglob("*") if file.suffix in (".ts", ".rs") and "__tests__" not in file.parts and not file.name.endswith(".test.ts") and file.name != "generated-contract.ts")
    # Direct frontend calls outside the centralized API modules are still
    # dependencies, including binary downloads/imports and OAuth navigation.
    for file in (ROOT / "frontend/src").rglob("*"):
        if file.suffix in (".ts", ".tsx") and "__tests__" not in file.parts:
            content = file.read_text()
            if "/api/" in content or "fetch(" in content:
                files.add(file)
    files.update(ROOT / name for name in ("marreq-core/src/app.rs", "marreq-core/src/build_info.rs", "marreq-core/src/api/mod.rs", "marreq-cloud/src/routes.rs", "marreq-cloud/src/deployment.rs", "marreq-server/src/deployment.rs", "marreq-server/src/routes.rs", "marreq-server/src/main.rs", "marreq-cloud/src/main.rs", "frontend/compatibility.json", "marreq-core/frontend_compatibility.json", "mcp-server/compatibility.json"))
    return {str(file.relative_to(ROOT)): hashlib.sha256(file.read_bytes()).hexdigest() for file in sorted(files) if file.is_file()}


def check(require_complete=False):
    spec = load(ROOT / "docs/developer/openapi.yaml")
    resolve(spec, spec)
    validate(spec)
    for component in ("frontend", "mcp-server"):
        policy(load(ROOT / component / "compatibility.json"))
    backend_range = load(ROOT / "marreq-core/frontend_compatibility.json")
    if set(backend_range) != {"min_version", "max_version"} or version(backend_range["min_version"]) > version(backend_range["max_version"]):
        raise ContractError("invalid backend frontend compatibility range")
    errors = []
    source = {(r["path"], r["method"]): r for r in source_routes()}
    documented = {(p, m): op for p, item in spec["paths"].items() for m, op in item.items()}
    for key in source.keys() - documented.keys():
        errors.append(f"undocumented backend route: {key}")
    for key in documented.keys() - source.keys():
        errors.append(f"documented route missing in backend: {key}")
    for key in source.keys() & documented.keys():
        variants = [source[key]["variant"]] if source[key]["variant"] != "shared" else ["server", "cloud"]
        if documented[key].get("x-marreq-variants") != variants:
            errors.append(f"deployment variant mismatch: {key}")
        if documented[key].get("x-marreq-guards") != source[key]["guards"]:
            errors.append(f"authentication guard mismatch: {key}")
        if documented[key].get("x-marreq-handler") != source[key]["handler"]:
            errors.append(f"handler mismatch: {key}")
    for path, content in generated(spec).items():
        if not path.exists() or path.read_text() != content:
            errors.append(f"generated artifact drift: {path.relative_to(ROOT)}; run scripts/contracts/generate.py")
    lock = ROOT / "docs/developer/api-contract-source-review.json"
    if not lock.exists() or load(lock) != source_evidence():
        errors.append("API source changed since contract review. Audit wire behavior, update OpenAPI/tests, then run check.py --record-source-review. This acknowledgement is not a substitute for schema or integration tests.")
    gaps = [f"{m.upper()} {p}" for (p, m), op in documented.items() if op.get("x-marreq-unverified")]
    if require_complete and spec.get("x-marreq-contract-complete") is not True:
        errors.append("producer contract is explicitly incomplete; finish the P0 audit before publication")
    if require_complete and gaps:
        errors.extend(f"unverified wire contract: {gap}" for gap in gaps)
    return errors, gaps


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record-source-review", action="store_true", help="Explicitly acknowledge source audit; review resulting diff")
    parser.add_argument("--require-complete", action="store_true", help="Release safety: fail on inventory-only operations")
    args = parser.parse_args()
    if args.record_source_review:
        (ROOT / "docs/developer/api-contract-source-review.json").write_text(json.dumps(source_evidence(), indent=2) + "\n")
    errors, gaps = check(args.require_complete)
    print(json.dumps({"valid": not errors, "unverified_operations": len(gaps), "errors": errors}, indent=2))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
