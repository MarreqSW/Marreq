#!/usr/bin/env python3
"""
Check doc/requirements.csv trace paths for compliance.
Verifies that each requirement's Trace paths exist in the repo (file or directory).
"""
import csv
import glob as glob_module
import os
import sys

REPO_ROOT = os.path.dirname(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
)
CSV_PATH = os.path.join(REPO_ROOT, "doc", "requirements.csv")

# Trace paths in CSV that map to different filenames in the repo
PATH_ALIASES = {
    "src/api/tests.rs": "backend/src/api/test_cases.rs",
    "src/routes/html/project/tests.rs": "backend/src/routes/html/project/test_cases.rs",
    "src/logger": "backend/src/logger.rs",
    "templates/index.hbs": "backend/templates/index.html.hbs",
    "templates/requirements/requirements.hbs": "backend/templates/requirements/requirements.html.hbs",
    "templates/tests/tests.hbs": "backend/templates/tests/tests.html.hbs",
    "templates/matrix/matrix.hbs": "backend/templates/matrix/matrix.html.hbs",
    "src/routes/html/project/project.rs": "backend/src/routes/html/project/project_routes.rs",
    "src/repository/cache/cache.rs": "backend/src/repository/cache/repository.rs",
    "src/services/project_member_service.rs": "backend/src/services/project_service.rs",
    "src/services/semantic_search/semantic_search_service.rs": "backend/src/services/semantic_search/search_service.rs",
}


def extract_trace_paths(justification: str) -> list[str]:
    """Extract trace paths from Justification column. Strips parentheticals like (SessionUser)."""
    if not justification or "Trace:" not in justification:
        return []
    trace_part = justification.split("Trace:")[-1].strip()
    raw_paths = [p.strip().split("(")[0].strip() for p in trace_part.split(";")]
    paths = []
    for p in raw_paths:
        if not p:
            continue
        # Sometimes "path1 path2" appears; split on space and take first token as path if second is a single word (e.g. "tests")
        if " " in p:
            first, rest = p.split(None, 1)
            paths.append(first)
            if rest and not rest.startswith("(") and " " not in rest and "/" not in rest:
                paths.append(rest)  # e.g. "tests" as directory
        else:
            paths.append(p)
    return paths


def resolve_path(path: str) -> str:
    """Resolve path using aliases; prefix legacy root paths with backend/."""
    if path in PATH_ALIASES:
        return PATH_ALIASES[path]
    if path.startswith("backend/"):
        return path
    if path.startswith(("src/", "templates/", "migrations/")):
        return f"backend/{path}"
    return path


def path_exists(base: str, path: str) -> bool:
    """Check if path exists as file or directory under base. Handles globs."""
    if "*" in path:
        full_pattern = os.path.join(base, path)
        return len(glob_module.glob(full_pattern)) > 0
    full = os.path.join(base, path)
    return os.path.isfile(full) or os.path.isdir(full)


def main() -> int:
    os.chdir(REPO_ROOT)
    missing_by_ref: dict[str, list[tuple[str, str]]] = {}  # ref -> [(raw_path, resolved_path)]
    total = 0
    with_open = 0

    with open(CSV_PATH, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            ref = row.get("Reference", "").strip()
            just = row.get("Justification", "")
            if not ref:
                continue
            total += 1
            paths = extract_trace_paths(just)
            if not paths:
                continue
            missing = []
            for raw in paths:
                resolved = resolve_path(raw)
                if not path_exists(REPO_ROOT, resolved):
                    missing.append((raw, resolved))
            if missing:
                with_open += 1
                missing_by_ref[ref] = missing

    # Report
    print("# Requirements compliance report (trace path existence)")
    print(f"\nTotal requirements: {total}")
    print(f"Requirements with at least one missing trace path: {with_open}")
    print(f"Requirements with all trace paths present: {total - with_open}")

    if missing_by_ref:
        print("\n## Requirements with missing or incorrect trace paths\n")
        for ref in sorted(missing_by_ref.keys()):
            entries = missing_by_ref[ref]
            print(f"- **{ref}**")
            for raw, resolved in entries:
                note = f" (CSV says `{raw}`; repo has `{resolved}`)" if raw != resolved and path_exists(REPO_ROOT, resolved) else ""
                print(f"  - `{raw}`" + note)
            print()
        return 1
    print("\nAll traced paths exist.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
