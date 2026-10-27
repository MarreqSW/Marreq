import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from compatibility import ContractError, ROOT, compare, policy, subset, verify_manifest


def spec():
    obj = {"type": "object", "properties": {"id": {"type": "integer"}, "state": {"type": "string", "enum": ["draft", "approved"]}}, "required": ["id", "state"], "additionalProperties": True}
    return {"openapi": "3.1.0", "info": {"title": "fixture", "version": "1.0.0"}, "paths": {"/api/things/{id}": {"post": {"security": [{"bearerAuth": []}], "parameters": [{"in": "path", "name": "id", "required": True, "schema": {"type": "integer"}}], "requestBody": {"required": True, "content": {"application/json": {"schema": {"type": "object", "required": ["title"], "properties": {"title": {"type": "string"}}, "additionalProperties": False}}}}, "responses": {"200": {"description": "OK", "content": {"application/json": {"schema": obj}}}, "401": {"description": "Unauthorized"}}}}}, "components": {"securitySchemes": {"bearerAuth": {"type": "http", "scheme": "bearer"}}}}


def operation(document):
    return document["paths"]["/api/things/{id}"]["post"]


def response(document):
    return operation(document)["responses"]["200"]["content"]["application/json"]["schema"]


def request(document):
    return operation(document)["requestBody"]["content"]["application/json"]["schema"]


class CompatibilityTests(unittest.TestCase):
    def setUp(self):
        self.old = spec()
        self.new = copy.deepcopy(self.old)

    def check(self):
        return compare(self.old, self.new, "server")

    def test_valid_openapi(self):
        self.assertEqual(self.check(), [])

    def test_removed_route(self):
        self.new["paths"] = {}
        self.assertIn("endpoint missing", self.check()[0])

    def test_incompatible_request_type(self):
        request(self.new)["properties"]["title"] = {"type": "integer"}
        self.assertTrue(self.check())

    def test_incompatible_response_type(self):
        response(self.new)["properties"]["id"] = {"type": "string"}
        self.assertTrue(self.check())

    def test_removed_required_response(self):
        response(self.new)["required"].remove("id")
        self.assertTrue(self.check())

    def test_new_required_request(self):
        request(self.new)["properties"]["reason"] = {"type": "string"}
        request(self.new)["required"].append("reason")
        self.assertTrue(self.check())

    def test_optional_request_addition(self):
        request(self.new)["properties"]["reason"] = {"type": "string"}
        self.assertEqual(self.check(), [])

    def test_response_addition(self):
        response(self.new)["properties"]["extra"] = {"type": "boolean"}
        response(self.new)["required"].append("extra")
        self.assertEqual(self.check(), [])

    def test_response_enum_extension_breaks_exhaustive_consumers(self):
        response(self.new)["properties"]["state"]["enum"].append("reviewed")
        self.assertTrue(self.check())

    def test_request_enum_restriction(self):
        request(self.old)["properties"]["title"]["enum"] = ["a", "b"]
        request(self.new)["properties"]["title"]["enum"] = ["a"]
        self.assertTrue(self.check())

    def test_oauth_scope_mismatch(self):
        operation(self.new)["x-marreq-bearer-scopes"] = ["things:write"]
        self.assertTrue(self.check())

    def test_authentication_mismatch(self):
        operation(self.new)["security"] = []
        self.assertTrue(self.check())

    def test_status_code_mismatch(self):
        operation(self.new)["responses"]["201"] = operation(self.new)["responses"].pop("200")
        self.assertTrue(self.check())

    def test_variants(self):
        operation(self.old)["x-marreq-variants"] = ["cloud"]
        self.new["paths"] = {}
        self.assertEqual(self.check(), [])
        self.assertTrue(compare(self.old, self.new, "cloud"))

    def test_unverified_contract_fails_even_when_identical(self):
        operation(self.old)["x-marreq-unverified"] = True
        self.assertTrue(compare(self.old, self.old, "cloud"))

    def test_unsupported_constraint_changes_fail_closed(self):
        self.assertTrue(subset({"type": "string"}, {"type": "string", "pattern": "a+"}, "request"))

    def test_nullable_response_breaks_nonnullable_consumer(self):
        self.assertTrue(subset({"type": ["string", "null"]}, {"type": "string"}, "response"))

    def test_metadata_invalid_or_incomplete(self):
        for value in ({}, None, {"requires_backend_min": "1.x"}, {"requires_backend_min": "2.0.0", "requires_backend_max": "1.0.0"}, {"requires_backend_min": "1.0.0", "typo": "x"}):
            with self.subTest(value=value), self.assertRaises(ContractError):
                policy(value)

    def deployment(self, directory, incompatible=False, backend_version="1.1.0"):
        plan = {"format": "marreq.deployment-contract.v1", "variant": "server"}
        for component, ver in (("backend", backend_version), ("frontend", "1.0.0"), ("mcp", "1.2.0")):
            contract = spec()
            if incompatible and component == "backend":
                response(contract)["required"].remove("id")
            bundle = {"format": "marreq.contract.v1", "component": component, "version": ver, "source_sha": "a" * 40, "contract": contract}
            if component == "backend":
                bundle.update(variant="server", frontend_compatibility={"min_version": "1.0.0", "max_version": "2.0.0"})
            else:
                bundle["compatibility"] = {"requires_backend_min": "1.0.0", "requires_backend_max": "1.9.0"}
            data = json.dumps(bundle).encode()
            (directory / f"{component}.json").write_bytes(data)
            plan[component] = {"file": f"{component}.json", "sha256": hashlib.sha256(data).hexdigest(), "version": ver, "source_sha": "a" * 40}
        path = directory / "plan.json"
        path.write_text(json.dumps(plan))
        return path

    def test_unchanged_frontend_and_mcp_with_new_compatible_backend(self):
        with tempfile.TemporaryDirectory() as temp:
            result = verify_manifest(self.deployment(Path(temp)))
            self.assertTrue(result["compatible"])
            self.assertNotEqual(result["versions"]["frontend"], result["versions"]["backend"])
            self.assertNotEqual(result["versions"]["mcp"], result["versions"]["backend"])

    def test_unchanged_consumers_with_incompatible_backend(self):
        with tempfile.TemporaryDirectory() as temp:
            result = verify_manifest(self.deployment(Path(temp), incompatible=True))
            self.assertFalse(result["compatible"])
            self.assertTrue(any(e.startswith("frontend:") for e in result["errors"]))
            self.assertTrue(any(e.startswith("mcp:") for e in result["errors"]))

    def test_versions_fail_independently_of_contracts(self):
        with tempfile.TemporaryDirectory() as temp:
            result = verify_manifest(self.deployment(Path(temp), backend_version="2.0.0"))
            self.assertFalse(result["compatible"])

    def test_missing_historical_artifact(self):
        with tempfile.TemporaryDirectory() as temp:
            path = self.deployment(Path(temp))
            (Path(temp) / "mcp.json").unlink()
            with self.assertRaisesRegex(ContractError, "missing historical artifact"):
                verify_manifest(path)

    def test_tampered_artifact(self):
        with tempfile.TemporaryDirectory() as temp:
            path = self.deployment(Path(temp))
            with (Path(temp) / "frontend.json").open("a") as stream:
                stream.write(" ")
            with self.assertRaisesRegex(ContractError, "SHA-256 mismatch"):
                verify_manifest(path)

    def test_descriptor_version_cannot_substitute_another_artifact(self):
        with tempfile.TemporaryDirectory() as temp:
            path = self.deployment(Path(temp))
            plan = json.loads(path.read_text())
            plan["frontend"]["version"] = "1.0.1"
            path.write_text(json.dumps(plan))
            with self.assertRaisesRegex(ContractError, "version disagrees"):
                verify_manifest(path)

    def test_bundle_path_cannot_escape_plan_directory(self):
        with tempfile.TemporaryDirectory() as temp:
            path = self.deployment(Path(temp))
            plan = json.loads(path.read_text())
            plan["mcp"]["file"] = "../mcp.json"
            path.write_text(json.dumps(plan))
            with self.assertRaisesRegex(ContractError, "inside the plan directory"):
                verify_manifest(path)

    def test_backend_variant_cannot_be_substituted(self):
        with tempfile.TemporaryDirectory() as temp:
            path = self.deployment(Path(temp))
            plan = json.loads(path.read_text())
            plan["variant"] = "cloud"
            path.write_text(json.dumps(plan))
            with self.assertRaisesRegex(ContractError, "variant mismatch"):
                verify_manifest(path)

    def test_candidate_plan_must_reference_tagged_source(self):
        with tempfile.TemporaryDirectory() as temp:
            path = self.deployment(Path(temp))
            import sys
            result = subprocess.run([sys.executable, str(ROOT / "scripts/contracts/compatibility.py"), "--manifest", str(path), "--candidate-component", "backend", "--candidate-version", "1.1.0", "--candidate-source-sha", "b" * 40], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn("exact tagged candidate", json.loads(result.stdout)["errors"][0])

    def test_invalid_openapi_does_not_pass_compatibility(self):
        self.new.pop("info")
        with self.assertRaises(Exception):
            self.check()

    def test_external_refs_are_rejected_before_any_validator_network_access(self):
        response(self.new)["properties"]["id"] = {"$ref": "https://example.test/schema.json"}
        with self.assertRaisesRegex(ContractError, "external or recursive"):
            self.check()

    def test_release_gate_zero_for_compatible_independent_artifacts(self):
        with tempfile.TemporaryDirectory() as temp:
            path = self.deployment(Path(temp))
            import sys
            result = subprocess.run([sys.executable, str(ROOT / "scripts/contracts/compatibility.py"), "--manifest", str(path)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0)
            self.assertTrue(json.loads(result.stdout)["compatible"])

    def test_release_gate_nonzero_on_real_violation(self):
        with tempfile.TemporaryDirectory() as temp:
            path = self.deployment(Path(temp), incompatible=True)
            import sys
            result = subprocess.run([sys.executable, str(ROOT / "scripts/contracts/compatibility.py"), "--manifest", str(path)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            self.assertFalse(json.loads(result.stdout)["compatible"])


if __name__ == "__main__":
    unittest.main()
