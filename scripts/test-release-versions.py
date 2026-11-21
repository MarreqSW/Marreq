#!/usr/bin/env python3
"""Exercise version validation in disposable Git repositories without registry access."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

sys.dont_write_bytecode = True

from release_versions import COMPONENTS, METADATA


SCRIPTS = Path(__file__).resolve().parent
BASE_VERSION = "0.1.0"
NEXT_VERSION = "0.1.1"
ALL = ("marreq", *COMPONENTS)


class ReleaseVersionsTest(unittest.TestCase):
    def setUp(self):
        temp = TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name) / "repo"
        self.root.mkdir()
        self.evidence = Path(temp.name) / "evidence"
        self.evidence.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "Test")
        self.git("config", "user.email", "test@example.test")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "tag.gpgSign", "false")
        self.git("config", "core.autocrlf", "false")
        for script in ("validate-release-versions", "release_versions.py", "detect-release-impact",
                       "release-impact-map.json", "check-deployment-provenance"):
            target = self.root / "scripts" / script
            target.parent.mkdir(exist_ok=True)
            shutil.copyfile(SCRIPTS / script, target)

        for component in ("server", "cloud"):
            self.write(f"marreq-{component}/Cargo.toml",
                       f'[package]\nname = "marreq-{component}"\nversion = "{BASE_VERSION}"\n')
        self.write("Cargo.lock", f'''version = 4

[[package]]
name = "marreq-cloud"
version = "{BASE_VERSION}"

[[package]]
name = "marreq-server"
version = "{BASE_VERSION}"
dependencies = ["example"]

[[package]]
name = "example"
version = "1.2.3"
source = "registry+https://example.test/index"
''')
        for name in ("frontend", "mcp-server"):
            package = {"name": f"marreq-{name}", "version": BASE_VERSION, "dependencies": {"example": "^2.0.0"}}
            self.write_json(f"{name}/package.json", package)
            self.write_json(f"{name}/package-lock.json", {
                "name": f"marreq-{name}", "version": BASE_VERSION, "lockfileVersion": 3,
                "packages": {"": package, "node_modules/example": {"version": "2.0.1"}},
            })
        self.write("frontend/src/example.ts", "published frontend\n")
        self.legacy = self.commit()
        self.set_versions()
        for name in ALL:
            self.write(f"changelogs/{name}.md", f"# {name}\n\n## [Unreleased]\n\n"
                       f"## [{BASE_VERSION}] - Bootstrap\n\n- Initial registry baseline.\n")
        self.bootstrap = self.commit()
        self.records = {}
        for component in COMPONENTS:
            self.publish_fixture(component, self.legacy)
        self.write_evidence()

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE, text=True).strip()

    def write(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")

    def read(self, path):
        return (self.root / path).read_text(encoding="utf-8")

    def write_json(self, path, value):
        self.write(path, json.dumps(value, indent=2) + "\n")

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")
        return self.git("rev-parse", "HEAD")

    def call(self, *args, error=None):
        result = subprocess.run(
            [sys.executable, str(self.root / "scripts/validate-release-versions"), "--format", "json", *args],
            cwd=self.root, text=True, capture_output=True, check=False,
        )
        self.assertEqual(result.returncode, 0 if error is None else 2, (result.stdout, result.stderr))
        report = json.loads(result.stdout)
        if error is not None:
            self.assertIn(error, report["error"])
        return report

    def set_versions(self, *bumps, value=NEXT_VERSION):
        def field(name):
            return value if name in bumps else BASE_VERSION

        self.write("versions.toml", f'[marreq]\nversion = "{field("marreq")}"\n\n[components]\n'
                   + "".join(f'{name} = "{field(name)}"\n' for name in COMPONENTS))

    def add_entry(self, name, value=NEXT_VERSION):
        path = f"changelogs/{name}.md"
        self.write(path, self.read(path).replace("## [Unreleased]\n", "## [Unreleased]\n\n"
                                                f"## [{value}] - 2026-11-21\n\n- Release fixture.\n", 1))

    def prepare(self, *components):
        self.set_versions("marreq", *components)
        for name in ("marreq", *components):
            self.add_entry(name)
        self.call("sync")
        return self.commit()

    def publish_fixture(self, name, source, value=BASE_VERSION):
        tag = f"marreq-{name}-v{value}"
        self.git("tag", "-f", tag, source)
        backend = name in ("server", "cloud")
        contract = {"format": "marreq.contract.v2", "component": "backend" if backend else name,
                    "version": value, "source_sha": source, "contract": {}}
        if backend:
            contract["variant"] = name
        contract_file = f"{name}.json"
        contents = json.dumps(contract, sort_keys=True).encode()
        (self.evidence / contract_file).write_bytes(contents)
        self.records[name] = {
            "version": value, "source_sha": source, "source_tag": tag,
            "image": f"ghcr.io/marreqsw/marreq-{'mcp-server' if name == 'mcp' else name}",
            "digest": "sha256:" + "a" * 64,
            "contract": {"file": contract_file, "sha256": hashlib.sha256(contents).hexdigest()},
        }

    def write_evidence(self, product=None):
        for backend in ("server", "cloud"):
            components = {}
            for name in (backend, "frontend", "mcp"):
                service = {"frontend": "frontend", "mcp": "mcp-server"}.get(name, f"marreq-{name}")
                if backend == "cloud" and name != "cloud":
                    service = f"{name}-cloud"
                components[name] = {**self.records[name], "service": service}
            self.write_json(self.evidence / f"{backend}-provenance.json", {
                "format": "marreq.deployment-provenance.v1", "variant": backend,
                "components": components, "product_release": product,
            })

    def release(self, head, *, previous=None, tag=f"v{NEXT_VERSION}", error=None):
        return self.call(
            "release", "--head", head, "--tag", tag, "--previous", previous or self.bootstrap,
            "--server-provenance", str(self.evidence / "server-provenance.json"),
            "--cloud-provenance", str(self.evidence / "cloud-provenance.json"), error=error,
        )

    def test_valid_and_explicit_bootstrap(self):
        self.call("check")
        self.assertTrue(self.call("check", "--head", self.bootstrap, "--base", self.legacy)["bootstrap"])
        self.call("tag", "v0.1.0")
        self.call("tag", "v0.1.1", error="must be v0.1.0")
        self.call("sync", "--check")

    def test_registry_rejects_invalid_values_and_schema(self):
        original = self.read("versions.toml")
        for invalid in (
            original.replace('mcp = "0.1.0"\n', ""),
            original + 'core = "0.1.0"\n',
            original + "\n[unknown]\n",
            original + 'mcp = "0.1.0"\n',
            original + "\n[marreq]\n",
            original.replace('"0.1.0"', "1"),
            original.replace('"0.1.0"', '"1.0"'),
            original.replace('"0.1.0"', '"01.0.0"'),
            original.replace('"0.1.0"', '"1.0.0-rc.1"'),
            original.replace('"0.1.0"', '"1.0.0+metadata"'),
            original.replace('"0.1.0"', '"0.1.0'),
            original.replace('"0.1.0"', '"9999999999999999.0.0"'),
        ):
            with self.subTest(invalid=invalid):
                self.write("versions.toml", invalid)
                self.call("check", error="")

    def test_package_inconsistency_and_deterministic_sync(self):
        for path in sorted(METADATA):
            with self.subTest(path=path):
                original = self.read(path)
                # For a npm lockfile this changes just the top-level version.
                self.write(path, original.replace(BASE_VERSION, NEXT_VERSION, 1))
                self.call("check", error=path)
                self.call("sync", "--check", error=path)
                self.assertNotEqual(self.read(path), original)
                self.assertEqual(self.call("sync")["updated_paths"], [path])
                self.assertEqual(self.read(path), original)
        lock = "mcp-server/package-lock.json"
        data = json.loads(self.read(lock))
        data["packages"][""]["version"] = NEXT_VERSION
        self.write_json(lock, data)
        self.call("check", error=lock)
        self.call("sync")
        self.set_versions("server", "mcp")
        before = {path: self.read(path) for path in METADATA}
        self.call("sync")
        after = {path: self.read(path) for path in METADATA}
        self.assertEqual(self.call("sync")["updated_paths"], [])
        self.assertEqual({path: self.read(path) for path in METADATA}, after)
        for path in METADATA:
            self.assertEqual(after[path].replace(NEXT_VERSION, BASE_VERSION), before[path], path)

    def test_invalid_sync_does_not_partially_write(self):
        self.set_versions("server")
        before = self.read("Cargo.lock")
        self.write("mcp-server/package.json", '{"name":"marreq-mcp-server","version":"0.1.0","version":"0.2.0"}')
        self.call("sync", error="duplicate JSON key")
        self.assertEqual(self.read("Cargo.lock"), before)

    def test_sync_preserves_nonstandard_json_layout(self):
        path = "frontend/package-lock.json"
        original = self.read(path).replace('\n  "version": "0.1.0",', '\n "version" : "0.1.0" ,')
        self.write(path, original)
        self.set_versions("frontend")
        self.call("sync")
        self.assertEqual(self.read(path), original.replace('"0.1.0"', '"0.1.1"'))

    def test_changelog_errors(self):
        path = "changelogs/marreq.md"
        original = self.read(path)
        for suffix, error in (
            ("\n## [0.1.0] - Bootstrap\n\n- Duplicate.\n", "duplicate version"),
            ("\n## [Unreleased]\n", "duplicate or misplaced"),
            ("\n## [End]\n", "expected an Unreleased heading"),
            ("\n## [0.0.9] - 2026-02-31\n\n- Invalid date.\n", "invalid release date"),
            ("\n## [0.1.3] - 2026-11-21\n\n- Out of order.\n", "versions must descend"),
        ):
            with self.subTest(error=error):
                self.write(path, original + suffix)
                self.call("check", error=error)
        self.write(path, original)
        self.set_versions("frontend")
        self.call("sync")
        self.call("check", error="changelogs/frontend.md: latest entry")
        self.add_entry("frontend")
        self.set_versions("marreq", "frontend")
        self.call("check", error="changelogs/marreq.md: latest entry")

    def test_pr_regressions(self):
        for name in ("marreq", "cloud"):
            with self.subTest(name=name):
                self.git("reset", "--hard", self.bootstrap)
                self.set_versions(name, value="0.0.9")
                self.write(f"changelogs/{name}.md",
                           self.read(f"changelogs/{name}.md").replace(BASE_VERSION, "0.0.9"))
                self.call("sync")
                self.call("check", "--base", self.bootstrap, error=f"{name}: version regressed")

    def test_duplicate_product_release(self):
        self.release(self.bootstrap, tag=f"v{BASE_VERSION}", error="product version must advance")

    def test_release_rejects_unjustified_version_bump(self):
        self.release(self.prepare("server"), error="retain 0.1.0")

    def test_independent_component_changes(self):
        for component, path in {
            "mcp": "mcp-server/src/example.ts",
            "frontend": "frontend/src/example.ts",
            "server": "marreq-server/src/example.rs",
            "cloud": "marreq-cloud/src/example.rs",
        }.items():
            with self.subTest(component=component):
                self.git("reset", "--hard", self.bootstrap)
                self.write(path, "release input change\n")
                without_bump = self.prepare()
                self.release(without_bump, error=f"{component}: advance 0.1.0")
                self.git("reset", "--hard", self.bootstrap)
                self.write(path, "release input change\n")
                report = self.release(self.prepare(component))
                self.assertEqual({name for name, entry in report["components"].items()
                                  if entry["action"] == "build"}, {component})

    def test_shared_core_lock_and_build_inputs(self):
        for path in ("marreq-core/src/example.rs", "Cargo.lock", "docker/Dockerfile"):
            with self.subTest(path=path):
                self.git("reset", "--hard", self.bootstrap)
                if path == "Cargo.lock":
                    self.write(path, self.read(path).replace("1.2.3", "1.2.4"))
                else:
                    self.write(path, "new build input\n")
                self.release(self.prepare("server"), error="cloud: advance")
                self.set_versions("marreq", "server", "cloud")
                self.add_entry("cloud")
                self.call("sync")
                report = self.release(self.commit())
                self.assertEqual({n for n, row in report["components"].items() if row["action"] == "build"},
                                 {"server", "cloud"})

    def test_docs_and_contract_only_reuse(self):
        self.write("docs/releases.md", "documentation only\n")
        self.write("docs/developer/openapi.yaml", "validation only\n")
        head = self.prepare()
        report = self.release(head)
        self.assertTrue(all(row["action"] == "reuse" for row in report["components"].values()))
        result = subprocess.check_output(
            [sys.executable, str(self.root / "scripts/detect-release-impact"),
             "--base", self.legacy, "--head", head, "--format", "json"], cwd=self.root, text=True,
        )
        self.assertEqual(json.loads(result)["checks"]["api_contracts"], ["docs/developer/openapi.yaml"])

    def test_unknown_and_unsafe_inputs(self):
        (self.root / "frontend/src/unsafe.ts").symlink_to("example.ts")
        self.release(self.prepare(), error="frontend/src/unsafe.ts")
        self.git("reset", "--hard", self.bootstrap)
        self.write("unknown-input", "must fail closed")
        self.release(self.prepare(), error="unknown-input")

    def test_component_baselines_are_independent(self):
        self.write("frontend/src/example.ts", "published in the newer frontend\n")
        self.write("mcp-server/src/example.ts", "still unpublished in the older MCP\n")
        frontend = self.commit()
        self.publish_fixture("frontend", frontend)
        self.write_evidence()
        report = self.release(self.prepare("mcp"))
        self.assertEqual(report["components"]["frontend"]["baseline"]["source_sha"], frontend)
        self.assertEqual(report["components"]["frontend"]["action"], "reuse")
        self.assertEqual(report["components"]["mcp"]["baseline"]["source_sha"], self.legacy)
        self.assertEqual(report["components"]["mcp"]["action"], "build")

    def test_missing_evidence_and_stale_tags(self):
        head = self.prepare()
        evidence = self.evidence / "cloud-provenance.json"
        evidence.unlink()
        self.call("check", "--head", head, "--base", self.bootstrap)
        self.release(head, error="cloud publication provenance")
        self.write_evidence()
        self.git("tag", "marreq-mcp-v0.1.2", self.bootstrap)
        self.release(head, error="newer than the supplied publication baseline")
        self.git("tag", "-d", "marreq-mcp-v0.1.2")
        self.git("tag", f"v{NEXT_VERSION}", self.bootstrap)
        self.release(head, error="already reserved")

    def test_publication_tampering(self):
        head = self.prepare()
        cloud_manifest = self.evidence / "cloud-provenance.json"
        data = json.loads(cloud_manifest.read_text())
        data["components"]["mcp"]["digest"] = "sha256:" + "b" * 64
        self.write_json(cloud_manifest, data)
        self.release(head, error="conflicting publication evidence")
        self.write_evidence()
        self.git("tag", "-f", "marreq-frontend-v0.1.0", self.bootstrap)
        self.release(head, error="disagrees with the retained publication SHA")

    def test_non_ancestor_publication(self):
        head = self.prepare()
        self.git("checkout", "--detach", self.bootstrap)
        self.write("frontend/src/example.ts", "unrelated tagged branch\n")
        self.publish_fixture("frontend", self.commit())
        self.write_evidence()
        self.release(head, error="must be an ancestor")

    def test_subsequent_coordinated_release(self):
        first = self.prepare()
        self.git("tag", "-a", f"v{NEXT_VERSION}", "-m", "first coordinated release", first)
        self.write_evidence(NEXT_VERSION)
        self.set_versions("marreq", value="0.1.2")
        self.add_entry("marreq", value="0.1.2")
        self.release(self.commit(), previous=first, tag="v0.1.2")


if __name__ == "__main__":
    unittest.main()
