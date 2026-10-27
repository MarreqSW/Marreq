#!/usr/bin/env python3
"""Reject breaking changes to audited operations in the PR base contract."""
import copy
import json
from pathlib import Path
import sys

from compatibility import ROOT, compare, load


def check(previous, proposed):
    if "x-marreq-contract-complete" not in previous:
        # The pre-bootstrap document is explicitly partial and has a different
        # /api server base. It is not evidence for a historical consumer release.
        return {"compatible": None, "bootstrap": True, "errors": [], "reason": "Previous OpenAPI predates verified contract inventory; no historical compatibility established."}
    audited = copy.deepcopy(previous)
    audited["paths"] = {path: {method: op for method, op in item.items() if not op.get("x-marreq-unverified")} for path, item in previous["paths"].items()}
    audited["paths"] = {path: item for path, item in audited["paths"].items() if item}
    errors = sorted(set(error for variant in ("server", "cloud") for error in compare(audited, proposed, variant)))
    return {"compatible": not errors, "errors": errors}


if __name__ == "__main__":
    result = check(load(Path(sys.argv[1])), load(ROOT / "docs/developer/openapi.yaml"))
    print(json.dumps(result, indent=2))
    sys.exit(1 if result["errors"] else 0)
