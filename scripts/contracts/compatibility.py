#!/usr/bin/env python3
"""Fail-closed, offline compatibility checks for immutable component bundles.

The deployment plan is an input, never inferred from package versions or main.
Only local files are read; artifact downloading/authentication belongs to CI/CD.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import sys

import yaml
from openapi_spec_validator import validate as validate_spec

ROOT = Path(__file__).resolve().parents[2]
METHODS = {"get", "post", "put", "patch", "delete", "head", "options"}


class ContractError(ValueError):
    pass


def load(path):
    with Path(path).open() as stream:
        return yaml.safe_load(stream)


def version(value):
    # No coercion, wildcards, prereleases, or lexicographic comparison. Release
    # candidates need their own explicitly verified deployment plan.
    if not isinstance(value, str) or not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", value):
        raise ContractError(f"expected stable SemVer X.Y.Z, got {value!r}")
    return tuple(map(int, value.split(".")))


def policy(value):
    if not isinstance(value, dict) or set(value) - {"requires_backend_min", "requires_backend_max"}:
        raise ContractError("invalid compatibility metadata fields")
    if "requires_backend_min" not in value:
        raise ContractError("compatibility metadata requires requires_backend_min")
    low = version(value["requires_backend_min"])
    high = version(value["requires_backend_max"]) if "requires_backend_max" in value else None
    if high is not None and low > high:
        raise ContractError("compatibility minimum exceeds maximum")
    return low, high


def resolve(node, document, seen=()):
    """Expand local refs; reject external refs and recursive schemas conservatively."""
    if isinstance(node, list):
        return [resolve(item, document, seen) for item in node]
    if not isinstance(node, dict):
        return node
    if "$ref" in node:
        ref = node["$ref"]
        if not ref.startswith("#/") or ref in seen:
            raise ContractError(f"unsupported external or recursive reference: {ref}")
        target = document
        for key in ref[2:].split("/"):
            target = target[key.replace("~1", "/").replace("~0", "~")]
        target = copy.deepcopy(target)
        target.update({key: value for key, value in node.items() if key != "$ref"})
        return resolve(target, document, (*seen, ref))
    return {key: resolve(value, document, seen) for key, value in node.items()}


def subset(source, target, location):
    """Prove every source value is accepted by target, or fail closed.

    Requests: released consumer -> proposed producer.
    Responses: proposed producer -> released consumer.
    Unsupported changed constraints cannot accidentally establish compatibility.
    """
    if source == target or target is True or target == {} or source is False:
        return []
    if target is False or source is True:
        return [f"{location}: schema acceptance changed"]
    if not isinstance(source, dict) or not isinstance(target, dict):
        return [f"{location}: unsupported schema change"]
    ignored = {"title", "description", "examples", "example", "deprecated", "default", "readOnly", "writeOnly", "$schema", "x-rust-model"}
    supported = {"type", "enum", "const", "properties", "required", "items", "additionalProperties", "minimum", "maximum", "minLength", "maxLength", "minItems", "maxItems", "minProperties", "maxProperties"}
    unknown = (set(source) | set(target)) - supported - ignored
    errors = []
    for key in sorted(unknown):
        if source.get(key) != target.get(key):
            errors.append(f"{location}: changed {key} needs explicit compatibility evidence")
    def types(schema):
        value = schema.get("type", [])
        return set(value if isinstance(value, list) else [value])
    a, b = types(source), types(target)
    if b and (not a or any(t not in b and not (t == "integer" and "number" in b) for t in a)):
        errors.append(f"{location}: type {sorted(a)} is not accepted by {sorted(b)}")
    for key in ("enum", "const"):
        if key in target:
            candidates = source.get("enum", [source["const"]] if "const" in source else None)
            accepted = target[key] if key == "enum" else [target[key]]
            if candidates is None or any(item not in accepted for item in candidates):
                errors.append(f"{location}: incompatible {key}")
    for key in ("minimum", "minLength", "minItems", "minProperties"):
        if key in target and (key not in source or source[key] < target[key]):
            errors.append(f"{location}: tightened {key}")
    for key in ("maximum", "maxLength", "maxItems", "maxProperties"):
        if key in target and (key not in source or source[key] > target[key]):
            errors.append(f"{location}: tightened {key}")
    for key in sorted(set(target.get("required", [])) - set(source.get("required", []))):
        errors.append(f"{location}.{key}: required property is not guaranteed")
    source_props = source.get("properties", {})
    target_props = target.get("properties", {})
    source_extra = source.get("additionalProperties", True)
    target_extra = target.get("additionalProperties", True)
    for key in sorted(set(source_props) | set(target_props)):
        left = source_props.get(key, source_extra)
        right = target_props.get(key, target_extra)
        if left is not False:
            errors.extend(subset(left, right, f"{location}.{key}"))
    errors.extend([] if source_extra is False else subset(source_extra, target_extra, f"{location}.*"))
    if "items" in source or "items" in target:
        errors.extend(subset(source.get("items", True), target.get("items", True), f"{location}[]"))
    return errors


def operations(spec, variant):
    result = {}
    for path, item in spec["paths"].items():
        for method in METHODS & item.keys():
            op = item[method]
            if variant in op.get("x-marreq-variants", ["server", "cloud"]):
                result[(path, method)] = {**op, "parameters": item.get("parameters", []) + op.get("parameters", [])}
    return result


def compare(consumer, backend, variant):
    consumer_resolved = resolve(consumer, consumer)
    backend_resolved = resolve(backend, backend)
    validate_spec(consumer)
    validate_spec(backend)
    required = operations(consumer_resolved, variant)
    offered = operations(backend_resolved, variant)
    errors = []
    for key, old in required.items():
        path, method = key
        at = f"{method.upper()} {path}"
        if key not in offered:
            errors.append(f"{at}: endpoint missing in {variant} backend")
            continue
        new = offered[key]
        for op in (old, new):
            if op.get("x-marreq-unverified"):
                errors.append(f"{at}: contract has unverified behavior; compatibility cannot be established")
        old_security = old.get("security", consumer.get("security", []))
        new_security = new.get("security", backend.get("security", []))
        if old_security != new_security:
            errors.append(f"{at}: authentication/scopes changed")
        if consumer.get("components", {}).get("securitySchemes") != backend.get("components", {}).get("securitySchemes"):
            errors.append(f"{at}: security scheme definitions changed")
        for name in ("x-marreq-policy", "x-marreq-csrf", "x-marreq-bearer-scopes"):
            if old.get(name) != new.get(name):
                errors.append(f"{at}: {name} changed")
        old_params = {(p["in"], p["name"]): p for p in old.get("parameters", [])}
        new_params = {(p["in"], p["name"]): p for p in new.get("parameters", [])}
        for p, param in new_params.items():
            if p not in old_params:
                if param.get("required"):
                    errors.append(f"{at}: new required {p} parameter")
            else:
                if param.get("required") and not old_params[p].get("required"):
                    errors.append(f"{at}: {p} parameter became required")
                errors.extend(subset(old_params[p].get("schema", {}), param.get("schema", {}), f"{at} parameter {p}"))
        if set(old_params) - set(new_params):
            errors.append(f"{at}: accepted parameters removed")
        old_body, new_body = old.get("requestBody", {}), new.get("requestBody", {})
        if new_body.get("required") and not old_body.get("required"):
            errors.append(f"{at}: request body became required")
        for media, content in old_body.get("content", {}).items():
            if media not in new_body.get("content", {}):
                errors.append(f"{at}: request media {media} removed")
            else:
                errors.extend(subset(content.get("schema", {}), new_body["content"][media].get("schema", {}), f"{at} request"))
        old_responses, new_responses = old["responses"], new["responses"]
        # Even error status changes matter: clients branch on 401/403/404/409.
        if old_responses.keys() != new_responses.keys():
            errors.append(f"{at}: response status codes changed")
        for status in old_responses.keys() & new_responses.keys():
            previous = old_responses[status]
            proposed = new_responses[status]
            if previous.get("headers", {}) != proposed.get("headers", {}):
                errors.append(f"{at} {status}: response headers changed")
            old_content, new_content = previous.get("content", {}), proposed.get("content", {})
            if old_content.keys() != new_content.keys():
                errors.append(f"{at} {status}: response media changed")
            for media in old_content.keys() & new_content.keys():
                errors.extend(subset(new_content[media].get("schema", {}), old_content[media].get("schema", {}), f"{at} response {status}"))
    return sorted(set(errors))


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_bundle(descriptor, component, directory):
    required = {"file", "sha256", "version", "source_sha"}
    if not isinstance(descriptor, dict) or set(descriptor) != required:
        raise ContractError(f"{component}: descriptor requires exactly {sorted(required)}")
    version(descriptor["version"])
    if not re.fullmatch(r"[a-f0-9]{40}", descriptor["source_sha"]):
        raise ContractError(f"{component}: immutable source SHA required")
    relative = Path(descriptor["file"])
    if relative.is_absolute() or ".." in relative.parts:
        raise ContractError(f"{component}: bundle file must stay inside the plan directory")
    path = directory / relative
    if not path.resolve().is_relative_to(directory.resolve()):
        raise ContractError(f"{component}: bundle symlink escapes the plan directory")
    if not path.is_file():
        raise ContractError(f"{component}: missing historical artifact {path}; never substitute current main")
    if not re.fullmatch(r"[a-f0-9]{64}", descriptor["sha256"]) or digest(path) != descriptor["sha256"]:
        raise ContractError(f"{component}: bundle SHA-256 mismatch")
    bundle = load(path)
    if bundle.get("format") != "marreq.contract.v1" or bundle.get("component") != component:
        raise ContractError(f"{component}: wrong bundle format/component")
    for key in ("version", "source_sha"):
        if bundle.get(key) != descriptor[key]:
            raise ContractError(f"{component}: bundle {key} disagrees with deployment plan")
    resolve(bundle["contract"], bundle["contract"])
    validate_spec(bundle["contract"])
    return bundle


def verify_manifest(path):
    manifest = load(path)
    if not isinstance(manifest, dict) or set(manifest) != {"format", "variant", "backend", "frontend", "mcp"}:
        raise ContractError("deployment plan requires format, variant, backend, frontend and mcp")
    if manifest["format"] != "marreq.deployment-contract.v1" or manifest["variant"] not in ("server", "cloud"):
        raise ContractError("invalid deployment plan format/variant")
    bundles = {name: read_bundle(manifest[name], name, Path(path).parent) for name in ("backend", "frontend", "mcp")}
    backend = bundles["backend"]
    if backend.get("variant") != manifest["variant"]:
        raise ContractError("backend artifact deployment variant mismatch")
    errors = []
    backend_version = version(backend["version"])
    frontend_range = backend.get("frontend_compatibility", {})
    if set(frontend_range) != {"min_version", "max_version"}:
        raise ContractError("backend frontend compatibility metadata is incomplete")
    lo, hi = version(frontend_range["min_version"]), version(frontend_range["max_version"])
    if lo > hi:
        raise ContractError("backend frontend compatibility range is inverted")
    if not lo <= version(bundles["frontend"]["version"]) <= hi:
        errors.append("frontend: outside backend's supported consumer range")
    for name in ("frontend", "mcp"):
        consumer = bundles[name]
        low, high = policy(consumer.get("compatibility"))
        if backend_version < low or (high is not None and backend_version > high):
            errors.append(f"{name}: backend {backend['version']} is outside declared range")
        errors.extend(f"{name}: {error}" for error in compare(consumer["contract"], backend["contract"], manifest["variant"]))
    return {"compatible": not errors, "variant": manifest["variant"], "versions": {name: bundle["version"] for name, bundle in bundles.items()}, "errors": errors}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--candidate-component", choices=["backend", "frontend", "mcp"])
    parser.add_argument("--candidate-version")
    parser.add_argument("--candidate-source-sha")
    parser.add_argument("--candidate-bundle", type=Path)
    args = parser.parse_args()
    try:
        candidate = (args.candidate_component, args.candidate_version, args.candidate_source_sha)
        if any(candidate):
            if not all(candidate):
                raise ContractError("candidate check requires component, version and source SHA")
            manifest = load(args.manifest)
            descriptor = manifest[args.candidate_component]
            if descriptor["version"] != args.candidate_version or descriptor["source_sha"] != args.candidate_source_sha:
                raise ContractError("deployment plan does not reference the exact tagged candidate; refusing to check a different artifact")
        if args.candidate_bundle:
            if not all(candidate):
                raise ContractError("candidate bundle requires candidate identity")
            if digest(args.candidate_bundle) != descriptor["sha256"]:
                raise ContractError("candidate contract is not the source-exported bundle for this tag")
        result = verify_manifest(args.manifest)
    except (ValueError, OSError, KeyError, TypeError) as error:
        result = {"compatible": False, "errors": [str(error)]}
    except Exception as error:  # validators also have their own exception types
        result = {"compatible": False, "errors": [f"invalid contract: {error}"]}
    print(json.dumps(result, indent=2))
    return 0 if result["compatible"] else 1


if __name__ == "__main__":
    sys.exit(main())
