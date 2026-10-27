import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from compatibility import ContractError, ROOT, load
from check import check
from check_baseline import check as check_baseline
from generate import generated
from test_compatibility import spec, response
from verify_transcript import verify


class ProducerTests(unittest.TestCase):
    def test_openapi_has_all_source_routes_and_generated_artifacts(self):
        # Review lock is checked separately and cannot stand in for route/schema checks.
        errors, gaps = check()
        self.assertEqual(errors, [])
        self.assertTrue(gaps)  # Be honest about the bootstrap's incomplete evidence.

    def test_documented_route_missing_from_actual_source_is_rejected(self):
        from inventory import source_routes
        routes = source_routes()
        with patch("check.source_routes", return_value=routes[1:]):
            errors, _ = check()
            self.assertTrue(any("documented route missing in backend" in error for error in errors))

    def test_cloud_route_accidentally_enabled_on_server_is_rejected(self):
        from inventory import source_routes
        routes = copy.deepcopy(source_routes())
        next(route for route in routes if route["variant"] == "cloud")["variant"] = "shared"
        with patch("check.source_routes", return_value=routes):
            errors, _ = check()
            self.assertTrue(any("deployment variant mismatch" in error for error in errors))

    def test_source_change_without_contract_review_is_rejected(self):
        with patch("check.source_evidence", return_value={"changed.rs": "f" * 64}):
            errors, _ = check()
            self.assertTrue(any("API source changed" in error for error in errors))

    def test_release_completeness_cannot_pass_inventory_only_entries(self):
        errors, gaps = check(require_complete=True)
        self.assertTrue(gaps)
        self.assertTrue(any("unverified wire contract" in error for error in errors))

    def test_audited_baseline_rejects_required_response_removal(self):
        old, new = spec(), spec()
        old["x-marreq-contract-complete"] = False
        response(new)["required"].remove("id")
        self.assertTrue(check_baseline(old, new)["errors"])

    def test_generated_types_come_from_the_contract(self):
        document = load(ROOT / "docs/developer/openapi.yaml")
        result = generated(document)
        document["components"]["schemas"]["BuildInfo"]["properties"]["new_required"] = {"type": "integer"}
        document["components"]["schemas"]["BuildInfo"]["required"].append("new_required")
        self.assertNotEqual(result, generated(document))

    def test_empty_integration_transcript_is_not_a_passing_backend_test(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "exchanges.jsonl"
            path.write_text("")
            with self.assertRaisesRegex(ContractError, "lacks critical cases"):
                verify(path)


if __name__ == "__main__":
    unittest.main()
