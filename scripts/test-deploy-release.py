#!/usr/bin/env python3
"""Offline tests for selective production deployment."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

REPO = Path(__file__).resolve().parent.parent
SCRIPTS = REPO / "scripts"
sys.path.insert(0, str(SCRIPTS))
import deploy_release as deploy  # noqa: E402

GIT = "a" * 40
DIGEST_SERVER = "sha256:" + "1" * 64
DIGEST_FRONTEND = "sha256:" + "2" * 64
DIGEST_MCP = "sha256:" + "3" * 64
DIGEST_OTHER = "sha256:" + "4" * 64


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def contract_bytes(component: str, version: str, sha: str) -> bytes:
    payload = {
        "format": "marreq.contract.v2",
        "component": "backend" if component == "server" else component,
        "version": version,
        "source_sha": sha,
        "contract": {},
    }
    if component == "server":
        payload["variant"] = "server"
    return json.dumps(payload).encode()


def evidence_tree(root: Path, tag: str = "v0.2.1") -> Path:
    evidence = root / "releases" / tag
    evidence.mkdir(parents=True)
    components = {
        "server": ("0.2.2", DIGEST_SERVER, "ghcr.io/marreqsw/marreq-server"),
        "frontend": ("0.2.3", DIGEST_FRONTEND, "ghcr.io/marreqsw/marreq-frontend"),
        "mcp": ("0.2.1", DIGEST_MCP, "ghcr.io/marreqsw/marreq-mcp-server"),
        "cloud": ("0.1.1", DIGEST_OTHER, "ghcr.io/marreqsw/marreq-cloud"),
    }
    contracts = {}
    for name, (version, digest, image) in components.items():
        if name == "cloud":
            continue
        data = contract_bytes(name, version, GIT)
        file_name = f"{name}-contract.json"
        (evidence / file_name).write_bytes(data)
        contracts[name] = {"file": file_name, "sha256": hashlib.sha256(data).hexdigest()}
    provenance_components = {}
    for name, service in deploy.COMPONENT_SERVICES.items():
        version, digest, image = components[name][0], components[name][1], components[name][2]
        provenance_components[name] = {
            "service": service,
            "version": version,
            "source_sha": GIT,
            "image": image,
            "digest": digest,
            "source_tag": f"marreq-{name}-v{version}",
            "contract": contracts[name],
        }
    write_json(
        evidence / "server-provenance.json",
        {
            "format": "marreq.deployment-provenance.v1",
            "variant": "server",
            "product_release": tag[1:],
            "components": provenance_components,
        },
    )
    write_json(
        evidence / "server-compatibility.json",
        {
            "format": "marreq.deployment-contract.v2",
            "variant": "server",
            "components": {
                "server": {"version": "0.2.2", "source_sha": GIT},
                "frontend": {"version": "0.2.3", "source_sha": GIT},
                "mcp": {"version": "0.2.1", "source_sha": GIT},
            },
        },
    )
    write_json(
        evidence / "release-manifest.json",
        {
            "schema_version": 1,
            "product": {"tag": tag, "version": tag[1:], "source_sha": GIT, "tag_object": "b" * 40},
            "recorded_at_unix": 1,
            "workflow": {"path": ".github/workflows/release-artifacts.yml", "repository": "MarreqSW/Marreq", "run_id": 1},
            "validation": {"bootstrap": False, "previous_source_sha": "c" * 40},
            "components": {
                name: {
                    "action": "build",
                    "version": version,
                    "source_sha": GIT,
                    "source_tag": f"marreq-{name}-v{version}",
                    "image": image,
                    "digest": digest,
                    "reference": f"{image}@{digest}",
                    "contract": contracts[name],
                }
                for name, (version, digest, image) in components.items()
                if name != "cloud"
            }
            | {
                "cloud": {
                    "action": "build",
                    "version": "0.1.1",
                    "source_sha": GIT,
                    "source_tag": "marreq-cloud-v0.1.1",
                    "image": "ghcr.io/marreqsw/marreq-cloud",
                    "digest": DIGEST_OTHER,
                    "reference": f"ghcr.io/marreqsw/marreq-cloud@{DIGEST_OTHER}",
                    "contract": {"file": "cloud-contract.json", "sha256": "d" * 64},
                }
            },
        },
    )
    return evidence


def make_repo(root: Path) -> Path:
    repo = root / "Marreq"
    (repo / "docker").mkdir(parents=True)
    (repo / "docker" / "docker-compose.light.github.yml").write_text("services: {}\n")
    (repo / "docker" / "docker-compose.persistence.yml").write_text("services: {}\n")
    (repo / "docker" / "docker-compose.release.yml").write_text("services: {}\n")
    evidence_tree(repo)
    return repo


class DeployReleaseTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = make_repo(self.root)
        self.lock = self.root / "ops.lock"
        self.env = {
            "MARREQ_REPO_ROOT": str(self.repo),
            "MARREQ_OPERATIONS_LOCK": str(self.lock),
            "MARREQ_EXPECTED_PG_CLUSTER_ID": "7690530505020129316",
            "MARREQ_MIN_DISK_MB": "1",
            "MARREQ_PUBLIC_URL": "https://app.marreq.com",
            "MARREQ_COMPOSE_PROJECT": "docker",
        }

    def ctx(self, tag="v0.2.1", dry_run=False, rollback=False):
        args = deploy.parse_args(
            (["--dry-run"] if dry_run else [])
            + (["--rollback"] if rollback else [])
            + [tag]
        )
        with mock.patch.dict(os.environ, self.env, clear=False):
            return deploy.DeployContext(args)

    def test_invalid_tag(self):
        with self.assertRaises(deploy.DeployError):
            deploy.validate_tag("v01.2.3")
        with self.assertRaises(deploy.DeployError):
            deploy.validate_tag("marreq-server-v0.2.2")

    def test_valid_manifest(self):
        evidence = self.repo / "releases" / "v0.2.1"
        manifest = deploy.validate_manifest(evidence)
        self.assertEqual(manifest["product"]["tag"], "v0.2.1")

    def test_incomplete_manifest(self):
        evidence = self.repo / "releases" / "v0.2.1"
        manifest = json.loads((evidence / "release-manifest.json").read_text())
        del manifest["components"]["frontend"]
        write_json(evidence / "release-manifest.json", manifest)
        with self.assertRaises(deploy.DeployError):
            deploy.validate_manifest(evidence)

    def test_untrusted_image_repository(self):
        evidence = self.repo / "releases" / "v0.2.1"
        manifest = json.loads((evidence / "release-manifest.json").read_text())
        manifest["components"]["server"]["image"] = "ghcr.io/evil/marreq-server"
        manifest["components"]["server"]["reference"] = (
            "ghcr.io/evil/marreq-server@" + DIGEST_SERVER
        )
        write_json(evidence / "release-manifest.json", manifest)
        with self.assertRaisesRegex(deploy.DeployError, "untrusted image"):
            deploy.validate_manifest(evidence)

    def test_stale_deployment_rejected(self):
        ctx = self.ctx("v0.2.0")
        (ctx.current_dir).mkdir(parents=True)
        write_json(ctx.current_dir / "installed-state.json", {"product_tag": "v0.2.1"})
        with self.assertRaisesRegex(deploy.DeployError, "stale"):
            deploy.reject_stale_request(ctx)

    def test_render_release_override(self):
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")
        existing = {
            "services": {
                "marreq-server": {
                    "image": "old",
                    "environment": {"MARREQ_AUTO_SEED": "false"},
                }
            }
        }
        text = deploy.render_release_override(manifest, existing)
        data = json.loads(text)
        self.assertEqual(
            data["services"]["frontend"]["image"],
            "ghcr.io/marreqsw/marreq-frontend@" + DIGEST_FRONTEND,
        )
        self.assertEqual(data["services"]["marreq-server"]["environment"]["MARREQ_AUTO_SEED"], "false")
        self.assertNotIn("db", data["services"])
        self.assertIn("x-marreq-release-comment", data)
        self.assertNotIn("_comment", data)

    def test_changed_services_detection(self):
        ctx = self.ctx()
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")

        def fake_ref(_ctx, service):
            mapping = {
                "marreq-server": f"ghcr.io/marreqsw/marreq-server@{DIGEST_SERVER}",
                "frontend": f"ghcr.io/marreqsw/marreq-frontend@{DIGEST_OTHER}",
                "mcp-server": f"ghcr.io/marreqsw/marreq-mcp-server@{DIGEST_MCP}",
            }
            return mapping[service]

        with mock.patch.object(deploy, "service_image_ref", side_effect=fake_ref):
            changed = deploy.changed_services(ctx, manifest)
        self.assertEqual(changed, ["frontend"])

    def test_noop_when_unchanged(self):
        ctx = self.ctx(dry_run=True)
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")

        def fake_ref(_ctx, service):
            return {
                "marreq-server": f"ghcr.io/marreqsw/marreq-server@{DIGEST_SERVER}",
                "frontend": f"ghcr.io/marreqsw/marreq-frontend@{DIGEST_FRONTEND}",
                "mcp-server": f"ghcr.io/marreqsw/marreq-mcp-server@{DIGEST_MCP}",
            }[service]

        with mock.patch.object(deploy, "service_image_ref", side_effect=fake_ref), mock.patch.object(
            deploy, "verify_runtime"
        ), mock.patch.object(deploy, "record_installed") as recorded:
            code = deploy.deploy_from_manifest(ctx, manifest)
        self.assertEqual(code, 0)
        recorded.assert_called_once()

    def test_frontend_only_recreates_frontend(self):
        ctx = self.ctx()
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")
        calls = []

        def fake_ref(_ctx, service):
            if service == "frontend":
                return f"ghcr.io/marreqsw/marreq-frontend@{DIGEST_OTHER}"
            return {
                "marreq-server": f"ghcr.io/marreqsw/marreq-server@{DIGEST_SERVER}",
                "mcp-server": f"ghcr.io/marreqsw/marreq-mcp-server@{DIGEST_MCP}",
            }[service]

        with mock.patch.object(deploy, "service_image_ref", side_effect=fake_ref), mock.patch.object(
            deploy, "migration_pending", return_value=False
        ), mock.patch.object(deploy, "retain_previous"), mock.patch.object(
            deploy, "write_release_override"
        ), mock.patch.object(deploy, "pull_images"), mock.patch.object(
            deploy, "recreate_service", side_effect=lambda _c, s: calls.append(s)
        ), mock.patch.object(deploy, "wait_healthy"), mock.patch.object(
            deploy, "verify_runtime"
        ), mock.patch.object(deploy, "verify_persistence"), mock.patch.object(
            deploy, "record_installed"
        ):
            deploy.deploy_from_manifest(ctx, manifest)
        self.assertEqual(calls, ["frontend"])

    def test_mcp_only_recreates_mcp(self):
        ctx = self.ctx()
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")
        calls = []

        def fake_ref(_ctx, service):
            if service == "mcp-server":
                return f"ghcr.io/marreqsw/marreq-mcp-server@{DIGEST_OTHER}"
            return {
                "marreq-server": f"ghcr.io/marreqsw/marreq-server@{DIGEST_SERVER}",
                "frontend": f"ghcr.io/marreqsw/marreq-frontend@{DIGEST_FRONTEND}",
            }[service]

        with mock.patch.object(deploy, "service_image_ref", side_effect=fake_ref), mock.patch.object(
            deploy, "migration_pending", return_value=False
        ), mock.patch.object(deploy, "retain_previous"), mock.patch.object(
            deploy, "write_release_override"
        ), mock.patch.object(deploy, "pull_images"), mock.patch.object(
            deploy, "recreate_service", side_effect=lambda _c, s: calls.append(s)
        ), mock.patch.object(deploy, "wait_healthy"), mock.patch.object(
            deploy, "verify_runtime"
        ), mock.patch.object(deploy, "verify_persistence"), mock.patch.object(
            deploy, "record_installed"
        ):
            deploy.deploy_from_manifest(ctx, manifest)
        self.assertEqual(calls, ["mcp-server"])

    def test_backend_only_and_multi_component_order(self):
        ctx = self.ctx()
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")
        calls = []

        def fake_ref(_ctx, service):
            return f"ghcr.io/marreqsw/other@{DIGEST_OTHER}"

        with mock.patch.object(deploy, "service_image_ref", side_effect=fake_ref), mock.patch.object(
            deploy, "migration_pending", return_value=False
        ), mock.patch.object(deploy, "retain_previous"), mock.patch.object(
            deploy, "write_release_override"
        ), mock.patch.object(deploy, "pull_images"), mock.patch.object(
            deploy, "recreate_service", side_effect=lambda _c, s: calls.append(s)
        ), mock.patch.object(deploy, "wait_healthy"), mock.patch.object(
            deploy, "verify_runtime"
        ), mock.patch.object(deploy, "verify_persistence"), mock.patch.object(
            deploy, "record_installed"
        ):
            deploy.deploy_from_manifest(ctx, manifest)
        self.assertEqual(calls, ["marreq-server", "mcp-server", "frontend"])

    def test_concurrent_lock(self):
        ctx = self.ctx()
        deploy.acquire_lock(ctx)
        self.addCleanup(deploy.release_lock, ctx)
        other = self.ctx()
        with self.assertRaisesRegex(deploy.DeployError, "lock"):
            deploy.acquire_lock(other)

    def test_missing_persistent_compose(self):
        (self.repo / "docker" / "docker-compose.persistence.yml").unlink()
        ctx = self.ctx()
        with self.assertRaisesRegex(deploy.DeployError, "missing production Compose"):
            deploy.ensure_compose_layout(ctx)

    def test_incorrect_postgres_cluster(self):
        ctx = self.ctx()
        with mock.patch.object(deploy, "postgres_cluster_id", return_value="1"), self.assertRaisesRegex(
            deploy.DeployError, "cluster identity mismatch"
        ):
            deploy.verify_persistence(ctx)

    def test_insufficient_disk_space(self):
        ctx = self.ctx()
        ctx.min_disk_mb = 10**12
        with self.assertRaisesRegex(deploy.DeployError, "insufficient disk"):
            deploy.verify_disk_space(ctx)

    def test_pending_migration_gate(self):
        ctx = self.ctx()
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")

        def fake_ref(_ctx, service):
            return f"ghcr.io/marreqsw/changed@{DIGEST_OTHER}"

        with mock.patch.object(deploy, "service_image_ref", side_effect=fake_ref), mock.patch.object(
            deploy, "migration_pending", return_value=True
        ), self.assertRaisesRegex(deploy.DeployError, "pending database migrations"):
            deploy.deploy_from_manifest(ctx, manifest)

    def test_failed_image_pull(self):
        ctx = self.ctx()
        with mock.patch.object(
            deploy, "run", side_effect=deploy.DeployError("pull failed")
        ), self.assertRaisesRegex(deploy.DeployError, "pull failed"):
            deploy.pull_images(ctx, [f"ghcr.io/marreqsw/marreq-server@{DIGEST_SERVER}"])

    def test_failed_health_check(self):
        ctx = self.ctx()
        with mock.patch.object(deploy, "run", return_value=subprocess.CompletedProcess(
            args=[], returncode=0, stdout="starting starting", stderr=""
        )), mock.patch.object(deploy.time, "time", side_effect=[0, 0, 200]), self.assertRaisesRegex(
            deploy.DeployError, "did not become healthy"
        ):
            deploy.wait_healthy(ctx, "frontend")

    def test_partial_failure_reports_completed(self):
        ctx = self.ctx()
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")
        calls = []

        def boom(_ctx, service):
            calls.append(service)
            if service == "mcp-server":
                raise deploy.DeployError("recreate failed")

        def fake_ref(_ctx, service):
            return f"ghcr.io/marreqsw/changed@{DIGEST_OTHER}"

        with mock.patch.object(deploy, "service_image_ref", side_effect=fake_ref), mock.patch.object(
            deploy, "migration_pending", return_value=False
        ), mock.patch.object(deploy, "retain_previous"), mock.patch.object(
            deploy, "write_release_override"
        ), mock.patch.object(deploy, "pull_images"), mock.patch.object(
            deploy, "recreate_service", side_effect=boom
        ), mock.patch.object(deploy, "wait_healthy"), self.assertRaisesRegex(
            deploy.DeployError, "recreate failed"
        ):
            deploy.deploy_from_manifest(ctx, manifest)
        self.assertEqual(calls, ["marreq-server", "mcp-server"])

    def test_safe_rollback_requires_previous(self):
        ctx = self.ctx(rollback=True)
        with self.assertRaisesRegex(deploy.DeployError, "no retained previous"):
            deploy.rollback(ctx)

    def test_safe_rollback_dry_run(self):
        ctx = self.ctx(tag="v0.2.1", dry_run=True, rollback=True)
        ctx.previous_dir.mkdir(parents=True)
        shutil.copy2(
            self.repo / "docker" / "docker-compose.release.yml",
            ctx.previous_dir / "docker-compose.release.yml",
        )
        write_json(
            ctx.previous_dir / "installed-state.json",
            {
                "product_tag": "v0.2.0",
                "source_sha": GIT,
                "components": {
                    "server": {
                        "service": "marreq-server",
                        "version": "0.2.1",
                        "source_sha": GIT,
                        "reference": f"ghcr.io/marreqsw/marreq-server@{DIGEST_SERVER}",
                    },
                    "frontend": {
                        "service": "frontend",
                        "version": "0.2.2",
                        "source_sha": GIT,
                        "reference": f"ghcr.io/marreqsw/marreq-frontend@{DIGEST_FRONTEND}",
                    },
                    "mcp": {
                        "service": "mcp-server",
                        "version": "0.2.0",
                        "source_sha": GIT,
                        "reference": f"ghcr.io/marreqsw/marreq-mcp-server@{DIGEST_MCP}",
                    },
                },
            },
        )
        with mock.patch.object(deploy, "migration_pending", return_value=False), mock.patch.object(
            deploy, "verify_persistence"
        ), mock.patch.object(
            deploy,
            "service_image_ref",
            return_value=f"ghcr.io/marreqsw/marreq-server@{DIGEST_OTHER}",
        ):
            self.assertEqual(deploy.rollback(ctx), 0)

    def test_unsafe_rollback_dry_run_rejects_schema(self):
        ctx = self.ctx(tag="v0.2.1", dry_run=True, rollback=True)
        ctx.previous_dir.mkdir(parents=True)
        shutil.copy2(
            self.repo / "docker" / "docker-compose.release.yml",
            ctx.previous_dir / "docker-compose.release.yml",
        )
        write_json(
            ctx.previous_dir / "installed-state.json",
            {
                "product_tag": "v0.2.0",
                "source_sha": GIT,
                "components": {
                    "server": {
                        "service": "marreq-server",
                        "version": "0.2.1",
                        "source_sha": GIT,
                        "reference": f"ghcr.io/marreqsw/marreq-server@{DIGEST_SERVER}",
                    },
                    "frontend": {
                        "service": "frontend",
                        "version": "0.2.2",
                        "source_sha": GIT,
                        "reference": f"ghcr.io/marreqsw/marreq-frontend@{DIGEST_FRONTEND}",
                    },
                    "mcp": {
                        "service": "mcp-server",
                        "version": "0.2.0",
                        "source_sha": GIT,
                        "reference": f"ghcr.io/marreqsw/marreq-mcp-server@{DIGEST_MCP}",
                    },
                },
            },
        )
        with mock.patch.object(deploy, "migration_pending", return_value=True), self.assertRaisesRegex(
            deploy.DeployError, "refusing application rollback"
        ):
            deploy.rollback(ctx)

    def test_shell_wrapper_help(self):
        result = subprocess.run(
            [str(SCRIPTS / "deploy-release.sh"), "--help"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("dry-run", result.stdout)

    def test_forced_command_rejects_shell(self):
        env = {**os.environ, "SSH_ORIGINAL_COMMAND": "bash", "MARREQ_DEPLOY_SCRIPT": str(SCRIPTS / "deploy-release.sh")}
        result = subprocess.run(
            ["bash", str(SCRIPTS / "production-deploy-entry.sh")],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )
        self.assertEqual(result.returncode, 2)

    def test_configuration_drift_wrong_digest(self):
        ctx = self.ctx()
        manifest = deploy.validate_manifest(self.repo / "releases" / "v0.2.1")
        with mock.patch.object(
            deploy,
            "service_image_ref",
            return_value=f"ghcr.io/marreqsw/marreq-server@{DIGEST_OTHER}",
        ):
            changed = deploy.changed_services(ctx, manifest)
        self.assertIn("marreq-server", changed)


if __name__ == "__main__":
    unittest.main()
