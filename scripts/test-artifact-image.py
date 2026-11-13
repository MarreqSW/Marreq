#!/usr/bin/env python3
"""Smoke-test the OCI labels and the runtime of a disposable CI image."""

import argparse
import json
import runpy
from pathlib import Path
import subprocess
import time
import uuid

provenance = runpy.run_path(str(Path(__file__).with_name("check-deployment-provenance")))
docker = provenance["docker"]



def verify_registry_identity(image, expected):
    """Exercise a push, pull and immutable container reference on the CI runner."""
    name = f"marreq-provenance-{uuid.uuid4().hex[:12]}"
    registry = docker("run", "-d", "--rm", "--name", name,
                      "-p", "127.0.0.1::5000", "registry:3").decode().strip()
    tagged = pinned = container = None
    try:
        host = docker("port", registry, "5000/tcp").decode().strip()
        repository = f"{host}/marreq-frontend"
        tagged = repository + ":ci"
        docker("tag", image, tagged)
        docker("push", tagged)
        metadata = json.loads(docker("image", "inspect", tagged))[0]
        refs = [ref for ref in metadata["RepoDigests"] if ref.startswith(repository + "@")]
        if len(refs) != 1:
            raise SystemExit("Registry push did not produce exactly one repository digest")
        pinned = refs[0]
        docker("pull", pinned)
        container = docker("run", "-d", "--rm", pinned).decode().strip()
        entry = {**expected, "image": repository, "digest": pinned.split("@")[1], "service": "frontend"}
        for attempt in range(30):
            report = provenance["inspect_component"]("frontend", entry, [container])
            if report["status"] != "unverified":
                break
            time.sleep(1)
        if report["status"] != "match":
            raise SystemExit(json.dumps(report, sort_keys=True))
        print("Local registry digest, running image and Frontend identity match")
    finally:
        if container:
            docker("rm", "-f", container)
        for ref in (pinned, tagged):
            if ref:
                subprocess.run(["docker", "image", "rm", ref], capture_output=True, check=False)
        docker("rm", "-f", registry)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("component", choices=provenance["IMAGES"])
    parser.add_argument("image")
    parser.add_argument("version")
    parser.add_argument("source_sha")
    parser.add_argument("--running-container")
    parser.add_argument("--registry-smoke", action="store_true")
    args = parser.parse_args()
    expected = {"component": args.component, "version": args.version, "source_sha": args.source_sha}
    labels = provenance["label_mismatches"](json.loads(docker("image", "inspect", args.image))[0],
                                            args.component, expected)
    if labels:
        raise SystemExit(f"OCI label mismatches: {labels}")
    container = args.running_container
    if not container:
        env = []
        if args.component == "mcp":
            env = ["-e", "MARREQ_MCP_TRANSPORT=http",
                   "-e", "MARREQ_MCP_PUBLIC_URL=http://localhost:3000/mcp",
                   "-e", "MARREQ_MCP_AUDIT_SECRET=disposable-test-audit-secret-at-least-32",
                   "-e", "MARREQ_BASE_URL=http://127.0.0.1:8000"]
        container = docker("run", "-d", "--rm", *env, args.image).decode().strip()
    try:
        for attempt in range(30):
            try:
                observed = provenance["runtime_identity"](container, args.component)
                break
            except OSError:
                if attempt == 29:
                    raise
                time.sleep(1)
        if observed != expected:
            raise SystemExit("Runtime identity mismatch")
        if args.component == "mcp":
            request = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                  "params": {"protocolVersion": "2025-03-26", "capabilities": {},
                                             "clientInfo": {"name": "ci", "version": "1.0.0"}}})
            result = subprocess.run(
                ["docker", "run", "--rm", "-i", "-e", "MARREQ_MCP_TRANSPORT=stdio",
                 "-e", "MARREQ_BASE_URL=http://127.0.0.1:8000",
                 "-e", "MARREQ_API_TOKEN=disposable", "-e", "MARREQ_PROJECT_ID=1", args.image],
                input=request + "\n", capture_output=True, text=True, timeout=20, check=True,
            )
            if json.loads(result.stdout)["result"]["serverInfo"]["version"] != args.version:
                raise SystemExit("MCP STDIO protocol version mismatch")
    finally:
        if not args.running_container:
            docker("rm", "-f", container)
    print(f"{args.component} OCI labels and runtime identity match")
    if args.registry_smoke:
        verify_registry_identity(args.image, expected)


if __name__ == "__main__":
    main()
