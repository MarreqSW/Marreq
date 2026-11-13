#!/usr/bin/env python3
"""Validate strict provenance and exercise the Docker evidence boundary."""

import copy
import hashlib
import json
from pathlib import Path
import runpy
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parent
CHECKER = SCRIPTS / "check-deployment-provenance"
TOOL = runpy.run_path(str(CHECKER))
GIT = "a" * 40
DIGEST = "sha256:" + "b" * 64
IMAGE_ID = "sha256:" + "c" * 64


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        work = tempfile.TemporaryDirectory()
        self.addCleanup(work.cleanup)
        self.root = Path(work.name)
        self.path = self.root / "manifest.json"

    def manifest(self, variant):
        entries = {}
        for component, service in TOOL["SERVICES"][variant].items():
            version = {"server": "0.2.1", "cloud": "0.2.3", "frontend": "0.2.2", "mcp": "0.1.9"}[component]
            sha = "d" * 40 if component == "mcp" else GIT
            bundle = {
                "format": "marreq.contract.v2", "version": version, "source_sha": sha,
                "component": "backend" if component == variant else component, "contract": {},
            }
            if component == variant:
                bundle["variant"] = variant
            name = f"{component}.json"
            data = json.dumps(bundle).encode()
            (self.root / name).write_bytes(data)
            entries[component] = {
                "service": service, "version": version, "source_sha": sha,
                "image": "ghcr.io/marreqsw/" + TOOL["IMAGES"][component],
                "digest": "sha256:" + "e" * 64 if component == "cloud" else DIGEST,
                "source_tag": f"marreq-{component}-v{version}",
                "contract": {"file": name, "sha256": hashlib.sha256(data).hexdigest()},
            }
        return {"format": TOOL["FORMAT"], "variant": variant, "product_release": "1.0.0", "components": entries}

    def cli(self, data, status):
        self.path.write_text(data if isinstance(data, str) else json.dumps(data))
        result = subprocess.run([CHECKER, "--manifest", self.path], capture_output=True, check=False)
        self.assertEqual(result.returncode, status)
        return json.loads(result.stdout)

    def test_both_variants_and_reused_mcp(self):
        server = self.manifest("server")
        cloud = self.manifest("cloud")
        self.assertEqual({**server["components"]["mcp"], "service": "mcp-cloud"}, cloud["components"]["mcp"])
        self.assertEqual(self.cli(server, 0)["status"], "valid_manifest")
        self.assertEqual(self.cli(cloud, 0)["status"], "valid_manifest")

    def test_strict_schema_and_contract_identity(self):
        original = self.manifest("cloud")
        for key, invalid in {
            "digest": "sha256:abc", "source_sha": "unknown", "image": "ghcr.io/marreqsw/marreq-cloud:latest",
            "version": "01.2.3", "source_tag": "marreq-server-v0.2.3", "service": "mcp-cloud",
            "contract": {"file": "../outside.json", "sha256": "0" * 64},
            "extra": "value",
        }.items():
            with self.subTest(key=key):
                manifest = copy.deepcopy(original)
                manifest["components"]["cloud"][key] = invalid
                self.assertEqual(self.cli(manifest, 2)["status"], "invalid_manifest")
        cloud = original["components"]["cloud"]
        cloud["source_sha"] = "f" * 40
        self.cli(original, 2)
        self.cli('{"format": "first", "format": "second"}', 2)
        original["components"].pop("mcp")
        self.cli(original, 2)

    def test_registry_digest_record_requires_contract(self):
        entry = self.manifest("server")["components"]["server"]
        args = [SCRIPTS / "record-published-artifact", "--component", "server",
                "--version", entry["version"], "--source-sha", entry["source_sha"],
                "--source-tag", entry["source_tag"], "--image", entry["image"],
                "--contract", self.root / "server.json", "--digest"]
        good = subprocess.run([*args, DIGEST], capture_output=True, check=True)
        self.assertEqual(json.loads(good.stdout)["contract"], entry["contract"])
        bad = subprocess.run([*args, ""], capture_output=True, check=False)
        self.assertEqual(bad.returncode, 2)
        self.assertEqual(bad.stdout, b"")

    def test_drift_and_unavailable_evidence(self):
        entry = self.manifest("cloud")["components"]["mcp"]
        expected_ref = f"{entry['image']}@{entry['digest']}"
        other = f"{entry['image']}@sha256:" + "e" * 64
        container_id = "f" * 64
        actual = {"Image": IMAGE_ID, "Config": {"Image": expected_ref},
                  "State": {"Running": True, "StartedAt": "start"}}
        image = {"Id": IMAGE_ID, "RepoDigests": [expected_ref], "Config": {"Labels": {
            "org.opencontainers.image.title": "marreq-mcp-server",
            "org.opencontainers.image.source": TOOL["SOURCE"],
            "org.opencontainers.image.version": entry["version"],
            "org.opencontainers.image.revision": entry["source_sha"],
        }}}
        runtime = {"component": "mcp", "version": entry["version"], "source_sha": entry["source_sha"]}
        inspect = TOOL["inspect_component"]
        scenarios = (
            ("identity_matches", actual, image, runtime),
            ("wrong_image_digest", {**actual, "Config": {"Image": other}},
             {**image, "RepoDigests": [other]}, runtime),
            ("registry_digest_unavailable", actual, {**image, "RepoDigests": []}, runtime),
            ("container_not_pinned_to_a_known_digest", {**actual, "Config": {"Image": entry["image"]}},
             image, runtime),
            ("image_metadata_mismatch", actual, {**image, "Config": {"Labels": {}}}, runtime),
            ("runtime_metadata_mismatch", actual, image, {**runtime, "version": "0.0.1"}),
            ("runtime_metadata_mismatch", actual, image, {**runtime, "source_sha": "unknown"}),
        )
        for reason, observed_container, observed_image, observed_runtime in scenarios:
            with self.subTest(reason=reason), patch.dict(inspect.__globals__, {
                "docker": lambda *args: json.dumps([observed_container if args[0] == "container" else observed_image]),
                "runtime_identity": lambda *_: observed_runtime,
            }):
                report = inspect("mcp", entry, [container_id])
                self.assertEqual(report["reason"], reason)
                self.assertNotIn("Config", json.dumps(report))
        self.assertEqual(inspect("mcp", entry, [])["reason"], "missing_container")
        self.assertEqual(inspect("mcp", entry, [container_id] * 2)["reason"], "ambiguous_service")


if __name__ == "__main__":
    unittest.main()
