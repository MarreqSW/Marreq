#!/usr/bin/env python3
"""Export the exact checked-out component's contract for retention with its image.

This records evidence; deployment verification still rejects unverified operations.
Never use this to manufacture contracts for older releases from current sources.
"""
import argparse
import json
import subprocess
import sys

from compatibility import ROOT, ContractError, digest, load, policy, version
from check import check


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--component", choices=["backend", "frontend", "mcp"], required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--variant", choices=["server", "cloud"])
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    version(args.version)
    errors, gaps = check()
    if errors:
        raise ContractError("; ".join(errors))
    if args.component == "backend" and not args.variant:
        raise ContractError("backend bundle requires --variant")
    sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    # Verify tracked contract inputs match the commit whose identity is exported.
    paths = list(load(ROOT / "docs/developer/api-contract-source-review.json")) + ["docs/developer/openapi.yaml", "docs/developer/api-contract-source-review.json"]
    tracked = subprocess.run(["git", "ls-files", "--error-unmatch", "--", *paths], cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if tracked.returncode:
        raise ContractError("commit all audited source files before exporting a source-bound bundle")
    result = subprocess.run(["git", "diff", "--quiet", "HEAD", "--", *paths], cwd=ROOT)
    if result.returncode:
        raise ContractError("commit audited contract inputs before exporting a source-bound bundle")
    contract = load(ROOT / "docs/developer/openapi.yaml")
    # Until consumer call coverage is complete, use the entire backend surface.
    # This is conservative and documented; never silently omit dependencies.
    bundle = {"format": "marreq.contract.v1", "component": args.component, "version": args.version, "source_sha": sha, "contract": contract}
    if args.component == "backend":
        bundle.update(variant=args.variant, frontend_compatibility=load(ROOT / "marreq-core/frontend_compatibility.json"))
    else:
        directory = "mcp-server" if args.component == "mcp" else "frontend"
        bundle["compatibility"] = load(ROOT / directory / "compatibility.json")
        policy(bundle["compatibility"])
    from pathlib import Path
    Path(args.output).write_text(json.dumps(bundle, indent=2) + "\n")
    print(json.dumps({"file": args.output, "sha256": digest(args.output), "version": args.version, "source_sha": sha}))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ContractError, OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
