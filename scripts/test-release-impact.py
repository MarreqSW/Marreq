#!/usr/bin/env python3
"""Exercise the release impact CLI against disposable Git histories."""

import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parent
DETECTOR = SCRIPTS / "detect-release-impact"
MAP = "scripts/release-impact-map.json"


class ReleaseImpactTest(unittest.TestCase):
    def setUp(self):
        work = tempfile.TemporaryDirectory()
        self.addCleanup(work.cleanup)
        self.repo = Path(work.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Release Impact Test")
        self.git("config", "user.email", "release-impact@example.invalid")
        self.git("config", "commit.gpgSign", "false")
        (self.repo / "scripts").mkdir()
        shutil.copyfile(SCRIPTS / Path(MAP).name, self.repo / MAP)
        self.base = self.commit("Baseline")

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.repo, text=True).strip()

    def write(self, path, contents="fixture"):
        file = self.repo / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(contents)

    def commit(self, message):
        self.git("add", "-A")
        self.git("commit", "-qm", message)
        return self.git("rev-parse", "HEAD")

    def run_detector(self, base, head, status=0, cwd=None, output_format="json"):
        result = subprocess.run(
            [DETECTOR, "--base", base, "--head", head, "--format", output_format],
            cwd=cwd or self.repo, capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, status, result.stderr)
        return result

    def assert_impact(self, path, artifacts=(), checks=(), infrastructure=()):
        self.write(path, "initial")
        base = self.commit("Add fixture")
        self.write(path, "modified")
        head = self.commit("Modify fixture")
        report = json.loads(self.run_detector(base, head).stdout)
        self.assertEqual(report, {
            "schema_version": 1, "base_sha": base, "head_sha": head,
            "artifacts": dict.fromkeys(artifacts, [path]),
            "checks": dict.fromkeys(checks, [path]),
            "infrastructure": dict.fromkeys(infrastructure, [path]),
            "unknown_inputs": [],
        })

    def test_component_sources(self):
        for path, artifacts, checks in (
            ("marreq-server/src/routes.rs", ("server",), ("rust", "api_contracts")),
            ("marreq-cloud/src/deployment.rs", ("cloud",), ("rust", "api_contracts")),
            ("marreq-core/src/app.rs", ("server", "cloud"), ("rust", "api_contracts")),
            ("marreq-core/src/services/decorated_test_service.rs",
             ("server", "cloud"), ("rust", "api_contracts")),
            ("frontend/src/api/client.ts", ("frontend",), ("frontend", "api_contracts")),
            ("mcp-server/src/client.ts", ("mcp",), ("mcp", "api_contracts")),
        ):
            with self.subTest(path=path):
                self.assert_impact(path, artifacts, checks)

    def test_build_and_runtime_inputs(self):
        rust_checks = ("rust", "api_contracts", "database_migrations")
        for path, artifacts, checks in (
            ("Cargo.lock", ("server", "cloud"), rust_checks),
            ("Cargo.toml", ("server", "cloud"), rust_checks),
            ("marreq-server/Cargo.toml", ("server", "cloud"), rust_checks),
            ("tools/marreq-contracts/Cargo.toml", ("server", "cloud"), rust_checks),
            ("docker/Dockerfile", ("server", "cloud"), ("database_migrations",)),
            ("docker/Dockerfile.dockerignore", ("server", "cloud"), ("database_migrations",)),
            ("docker/docker-entrypoint.sh", ("server", "cloud"), ("database_migrations",)),
            ("marreq-core/migrations/20260101000000_example/up.sql",
             ("server", "cloud"), ("rust", "database_migrations")),
            ("tools/marreq-migrations/build.rs",
             ("server", "cloud"), ("rust", "database_migrations")),
            ("tools/marreq-migrations/src/recovery.rs",
             ("server", "cloud"), ("rust", "database_migrations")),
            ("docker/frontend/Dockerfile", ("frontend",), ("frontend", "api_contracts")),
            ("docker/frontend/nginx.conf.template", ("frontend",), ("frontend", "api_contracts")),
            ("mcp-server/Dockerfile", ("mcp",), ("mcp", "api_contracts")),
            ("mcp-server/MARREQ_SKILL.md", ("mcp",), ("mcp", "api_contracts")),
            (".dockerignore", ("frontend", "mcp"), ("frontend", "mcp")),
        ):
            with self.subTest(path=path):
                self.assert_impact(path, artifacts, checks)

    def test_contracts_and_validation_only(self):
        for path, artifacts, checks in (
            ("docs/developer/openapi/paths/auth.yaml", (), ("api_contracts",)),
            ("tools/marreq-contracts/src/contract.rs", (), ("api_contracts",)),
            ("scripts/generate-api-types.mjs", (), ("api_contracts",)),
            ("package-lock.json", (), ("api_contracts", "frontend")),
            ("frontend/src/api/generated-contract.ts", ("frontend",), ("frontend", "api_contracts")),
            ("mcp-server/compatibility.json", ("mcp",), ("mcp", "api_contracts")),
            ("docs/developer/setup.md", (), ()),
            ("marreq-core/tests/it/main.rs", (), ("rust",)),
            ("marreq-cloud/tests/api_contract_inventory.rs", (), ("rust", "api_contracts")),
            ("frontend/vitest.config.ts", ("frontend",), ("frontend", "api_contracts")),
            ("frontend/README.md", ("frontend",), ("frontend", "api_contracts")),
            ("mcp-server/vitest.config.ts", (), ("mcp",)),
            ("tools/marreq-migrations/released-migrations.json", (), ("database_migrations",)),
        ):
            with self.subTest(path=path):
                self.assert_impact(path, artifacts, checks)

    def test_legacy_openapi_removal(self):
        path = "docs/developer/openapi.yaml"
        self.write(path)
        base = self.commit("Legacy contract")
        self.git("rm", path)
        head = self.commit("Remove legacy contract")
        report = json.loads(self.run_detector(base, head).stdout)
        self.assertEqual(report["artifacts"], {})
        self.assertEqual(report["checks"], {"api_contracts": [path]})

    def test_infrastructure(self):
        self.assert_impact("docker/docker-compose.prod.yml", checks=("deployment",),
                           infrastructure=("deployment",))
        self.assert_impact(".github/workflows/release-artifacts.yml",
                           ("server", "cloud", "frontend", "mcp"),
                           ("release_pipeline", "api_contracts"), ("release",))

    def test_multiple_components_rename_and_delete(self):
        old = "marreq-server/src/old.rs"
        new = "marreq-cloud/src/new.rs"
        deleted = "frontend/src/api/obsolete.ts"
        added = "mcp-server/src/file with space.ts"
        self.write(old)
        self.write(deleted)
        base = self.commit("Before rename and deletion")
        (self.repo / new).parent.mkdir(parents=True)
        self.git("mv", old, new)
        self.git("rm", deleted)
        self.write(added)
        head = self.commit("Rename, delete and add")
        report = json.loads(self.run_detector(base, head).stdout)
        self.assertEqual(report["artifacts"], {
            "server": [old], "cloud": [new], "frontend": [deleted], "mcp": [added],
        })

    def test_empty_diff_and_worktree_independence(self):
        self.write(MAP, "uncommitted invalid map")
        self.write("untracked-build.sh")
        result = self.run_detector(self.base, self.base, cwd=self.repo / "scripts")
        report = json.loads(result.stdout)
        for dimension in ("artifacts", "checks", "infrastructure"):
            self.assertEqual(report[dimension], {})
        self.assertEqual(report["unknown_inputs"], [])

    def test_independent_baselines_and_annotated_tag(self):
        self.git("tag", "-a", "fixture-server", "-m", "Server fixture")
        self.write("marreq-server/src/main.rs")
        cloud_base = self.commit("Server change")
        self.write("marreq-cloud/src/main.rs")
        mcp_base = self.commit("Cloud change")
        self.write("frontend/src/main.tsx")
        head = self.commit("Frontend change")
        for component, baseline, affected in (
            ("server", "fixture-server", True),
            ("cloud", cloud_base, True),
            ("frontend", head, False),
            ("mcp", mcp_base, False),
        ):
            with self.subTest(component=component):
                result = self.run_detector(baseline, head)
                report = json.loads(result.stdout)
                self.assertEqual(component in report["artifacts"], affected)
                self.assertEqual(report["base_sha"], self.git("rev-parse", f"{baseline}^{{commit}}"))

    def test_unknown_additions_and_deletions_fail_closed(self):
        paths = ["scripts/regenerate-lock.py", "tools/new-crate/Cargo.toml",
                 "docker/other/Dockerfile", "shared/generated.json", "marreq-server/build.rs",
                 ".github/workflows/new-release.yml"]
        for path in paths:
            self.write(path)
        unknown_head = self.commit("Add unclassified inputs")
        for base, head in (
            (self.base, unknown_head), (unknown_head, self.base), (unknown_head, unknown_head),
        ):
            result = self.run_detector(base, head, status=2)
            self.assertEqual(json.loads(result.stdout)["unknown_inputs"], sorted(paths))
            self.assertIn("Unclassified or unsupported release inputs", result.stderr)

    def assert_unsupported_modes(self, base, head, paths):
        result = self.run_detector(base, head, status=2)
        report = json.loads(result.stdout)
        self.assertEqual(report["unknown_inputs"], sorted(paths))
        self.assertEqual(report["artifacts"], {})
        self.assertIn("Unclassified or unsupported release inputs", result.stderr)
        for path in paths:
            self.assertIn(repr(path), result.stderr)
        repeated = self.run_detector(base, head, status=2)
        self.assertEqual((repeated.stdout, repeated.stderr), (result.stdout, result.stderr))

    def test_symlink_add_delete_and_replace(self):
        path = "frontend/src/cross-component.ts"
        self.write(path)
        regular = self.commit("Add regular source")
        (self.repo / path).unlink()
        (self.repo / path).symlink_to("../../mcp-server/src/client.ts")
        linked = self.commit("Replace regular source with a symlink")
        self.git("rm", path)
        absent = self.commit("Delete the symlink")
        for base, head in (
            (absent, linked), (linked, absent), (linked, regular), (regular, linked),
            (linked, linked),
        ):
            with self.subTest(base=base, head=head):
                self.assert_unsupported_modes(base, head, [path])

        for base, head in ((self.base, regular), (regular, absent)):
            report = json.loads(self.run_detector(base, head).stdout)
            self.assertEqual(report["unknown_inputs"], [])
            self.assertEqual(report["artifacts"], {"frontend": [path]})

    def test_rename_unsupported_source(self):
        old = "marreq-server/src/linked.rs"
        new = "marreq-cloud/src/linked.rs"
        for path in (old, new):
            (self.repo / path).parent.mkdir(parents=True)
        (self.repo / old).symlink_to("missing")
        base = self.commit("Add the Server symlink")
        self.git("mv", old, new)
        head = self.commit("Move the symlink to Cloud")
        self.assert_unsupported_modes(base, head, [old, new])

    def test_gitlink_removal_and_addition(self):
        path = "frontend/src/vendor"
        self.git("update-index", "--add", "--cacheinfo", f"160000,{self.base},{path}")
        self.git("commit", "-qm", "Add a gitlink")
        head = self.git("rev-parse", "HEAD")
        self.assert_unsupported_modes(self.base, head, [path])
        self.assert_unsupported_modes(head, self.base, [path])

    def test_executable_mode_changes_are_supported(self):
        path = "marreq-core/scripts/db_setup.sh"
        self.write(path)
        base = self.commit("Add non-executable script")
        (self.repo / path).chmod(0o755)
        self.git("update-index", "--chmod=+x", path)
        head = self.commit("Mark script executable")
        for first, second in ((base, head), (head, base)):
            report = json.loads(self.run_detector(first, second).stdout)
            self.assertEqual(report["unknown_inputs"], [])
            self.assertEqual(report["artifacts"], {"cloud": [path], "server": [path]})

    def test_unsupported_entry_only_in_ancestor(self):
        path = self.repo / "frontend/src/old-link.ts"
        path.parent.mkdir(parents=True)
        path.symlink_to("missing")
        self.commit("Historic symlink")
        path.unlink()
        base = self.commit("Remove historic symlink before baseline")
        self.write("mcp-server/src/client.ts")
        head = self.commit("Update MCP")
        report = json.loads(self.run_detector(base, head).stdout)
        self.assertEqual(report["unknown_inputs"], [])
        self.assertEqual(report["artifacts"], {"mcp": ["mcp-server/src/client.ts"]})

    def test_production_can_consume_files_named_like_tests(self):
        # The production consumers are identical at both endpoints. Only the
        # imported/embedded inputs change between releases.
        for path, content in {
            "marreq-core/src/lib.rs": 'pub const VALUE: &str = include_str!("validation/tests.rs");',
            "tools/marreq-migrations/build.rs": 'fn main() { print!("{}", include_str!("src/tests.rs")); }',
            "frontend/src/main.tsx": 'import { value } from "./test/fixture"; document.title = value;',
            "mcp-server/src/index.ts": 'export { value } from "./fixture.test.js";',
        }.items():
            self.write(path, content)
        inputs = {
            "marreq-core/src/validation/tests.rs": 'const VALUE: &str = "before";',
            "tools/marreq-migrations/src/tests.rs": 'const VALUE: &str = "before";',
            "frontend/src/test/fixture.ts": 'export const value = "before";',
            "mcp-server/src/fixture.test.ts": 'export const value = "before";',
        }
        for path, content in inputs.items():
            self.write(path, content)
        base = self.commit("Production consumers and their inputs")
        for path, content in inputs.items():
            self.write(path, content.replace("before", "after"))
        head = self.commit("Change only the inputs")
        report = json.loads(self.run_detector(base, head).stdout)
        core, migrations, frontend, mcp = inputs
        self.assertEqual(report["unknown_inputs"], [])
        self.assertEqual(report["artifacts"], {
            "cloud": [core, migrations], "server": [core, migrations],
            "frontend": [frontend], "mcp": [mcp],
        })
        self.assertEqual(report["checks"]["database_migrations"], [migrations])

    def test_invalid_ref(self):
        for base, head in (("no-such-tag", self.base), (self.base, "missing-head")):
            result = self.run_detector(base, head, status=2)
            self.assertIn("cannot resolve commit", result.stderr)
            self.assertEqual(result.stdout, "")

    def test_invalid_and_incomplete_maps(self):
        original = json.loads((self.repo / MAP).read_text())
        for rules in (
            [{"paths": ["["]}],
            [{"paths": [MAP], "artifacts": ["backend"]}],
            [{"paths": [MAP, MAP]}],
            [{"paths": ["unrelated-file"]}],
        ):
            self.write(MAP, json.dumps({**original, "rules": rules}))
            head = self.commit("Invalid or incomplete map")
            result = self.run_detector(self.base, head, status=2)
            self.assertIn(MAP, result.stderr)

    def test_stable_report_and_human_output(self):
        self.write("mcp-server/src/z.ts")
        self.write("mcp-server/src/a.ts")
        head = self.commit("MCP changes")
        first = self.run_detector(self.base, head).stdout
        self.assertEqual(first, self.run_detector(self.base, head).stdout)
        self.assertEqual(json.loads(first)["artifacts"]["mcp"],
                         ["mcp-server/src/a.ts", "mcp-server/src/z.ts"])
        self.assertIn("mcp: 'mcp-server/src/a.ts'", self.run_detector(
            self.base, head, output_format="text").stdout)


if __name__ == "__main__":
    unittest.main()
