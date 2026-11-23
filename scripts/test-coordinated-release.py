#!/usr/bin/env python3
"""Exercise the publication state machine with offline GitHub and GHCR fixtures."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

REPO = Path(__file__).resolve().parent.parent
HEAD = "a" * 40
OLD = "b" * 40
ANNOTATION = "c" * 40
TAG = "v0.1.2"
COMPONENTS = ("server", "cloud", "frontend", "mcp")


def checksum(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value))


def image(component):
    return "ghcr.io/marreqsw/" + ("marreq-mcp-server" if component == "mcp" else f"marreq-{component}")


def fixture_command(tool, args):
    state_path = Path(os.environ["FIXTURE_STATE"])
    state = json.loads(state_path.read_text())
    release = state["release"]
    aliases, blobs, tags = (state[k] for k in ("aliases", "blobs", "tags"))
    fault = os.environ.get("FIXTURE_FAULT")
    work = Path(os.environ["RELEASE_WORK"])

    def option(name):
        return args[args.index(name) + 1]

    if tool == "gh":
        if args[:2] == ["release", "upload"]:
            file = Path(args[3])
            assert not any(a["name"] == file.name for a in release["assets"])
            if fault == "asset" and file.name == "cloud-contract.json":
                sys.exit(1)
            asset_id = len(release["assets"]) + 1
            shutil.copyfile(file, state_path.parent / str(asset_id))
            release["assets"].append(
                {"name": file.name, "id": asset_id, "state": "uploaded", "digest": checksum(file.read_bytes())}
            )
        else:
            route = next(arg.split("/", 3)[3] for arg in args if arg.startswith("repos/"))
            if route == "branches/main":
                print(json.dumps({"protected": fault != "unprotected", "commit": {"sha": OLD if fault == "stale" else HEAD}}))
            elif route.startswith("git/ref/tags/"):
                print(json.dumps({"object": {"sha": ANNOTATION, "type": "tag"}}))
            elif route == "immutable-releases":
                print(json.dumps({"enabled": fault != "mutable"}))
            elif route.startswith("releases?"):
                print(json.dumps([[release] if release else []]))
            elif route.startswith("releases/assets/"):
                sys.stdout.buffer.write((state_path.parent / route.rsplit("/", 1)[1]).read_bytes())
            elif route == "releases":
                assert release is None
                state["release"] = dict(id=100, draft=True, immutable=False, prerelease=False, assets=[], tag_name=TAG,
                                        author={"login": "github-actions[bot]"}, target_commitish=HEAD,
                                        name=f"Marreq {TAG} (run {os.environ['GITHUB_RUN_ID']})")
                print(json.dumps(state["release"]))
            elif route == "releases/100":
                if "PATCH" in args:
                    assert release["draft"] and len(release["assets"]) == 9
                    release.update(draft=False, immutable=True)
                print(json.dumps(release))
            elif route == "git/refs":
                name = next(arg.removeprefix("ref=refs/tags/") for arg in args if arg.startswith("ref=refs/tags/"))
                source = args[-1].split("=")[1]
                assert name not in tags
                tags[name] = source
            else:
                raise AssertionError(args)
    elif tool == "git":
        if args[0] == "ls-remote":
            ref = args[2]
            name = ref.removeprefix("refs/tags/")
            if name in tags:
                print(f"{tags[name]}\t{ref}")
        elif args[0] == "archive":
            assert args[1] == HEAD
            with tarfile.open(fileobj=sys.stdout.buffer, mode="w|"):
                pass
        else:
            raise AssertionError(args)
    elif tool == "curl":
        url = args[-1]
        if url.startswith("https://ghcr.io/token"):
            print('{"token":"offline-token"}')
        else:
            repository, ref = url.split("/v2/")[1].split("/manifests/")
            repository = "ghcr.io/" + repository
            digest = ref if ref.startswith("sha256:") else aliases.get(f"{repository}:{ref}")
            body = blobs.get(f"{repository}@{digest}")
            if body is None:
                Path(option("--output")).write_text('{"errors":[{"code":"MANIFEST_UNKNOWN"}]}')
                print(404)
            else:
                Path(option("--output")).write_text(body)
                Path(option("--dump-header")).write_text(f"Docker-Content-Digest: {digest}\n")
                print(200)
    elif tool == "docker":
        if args[:2] == ["buildx", "build"]:
            stage = option("--tag")
            repository = stage.rsplit(":", 1)[0]
            plan = json.loads((work / "plan.json").read_text())
            component, row = next((key, row) for key, row in plan["selection"].items()
                                  if row["image"] == repository)
            dockerfile = {
                "server": "docker/Dockerfile", "cloud": "docker/Dockerfile",
                "frontend": "docker/frontend/Dockerfile", "mcp": "mcp-server/Dockerfile",
            }[component]
            assert option("--file") == str(work / "context" / dockerfile)
            assert f"MARREQ_VERSION={row['version']}" in args and f"MARREQ_GIT_SHA={HEAD}" in args
            if component in ("server", "cloud"):
                assert f"MARREQ_BIN=marreq-{component}" in args
            state["builds"].append(repository)
            write_json(state_path, state)
            if fault == "build":
                sys.exit(1)
            body = json.dumps({"repository": repository, "stage": stage})
            digest = checksum(body.encode())
            aliases[stage] = digest
            blobs[f"{repository}@{digest}"] = body
            metadata = {} if fault == "missing-digest" else {"containerimage.digest": digest}
            write_json(option("--metadata-file"), metadata)
        elif args[:3] == ["buildx", "imagetools", "create"]:
            alias, ref = option("--tag"), args[-1]
            assert alias not in aliases, "A retry attempted to overwrite a canonical tag"
            if fault == "promotion" and len(state["promotions"]) == 1:
                sys.exit(1)
            assert ref in blobs
            state["promotions"].append(alias)
            aliases[alias] = ref.split("@")[1]
        elif args[:2] == ["image", "inspect"]:
            repository = args[-1].split("@")[0]
            selected = json.loads((work / "plan.json").read_text())["selection"]
            component, row = next((key, row) for key, row in selected.items() if row["image"] == repository)
            labels = {"org.opencontainers.image.title": repository.rsplit("/", 1)[1],
                      "org.opencontainers.image.source": "https://github.com/MarreqSW/Marreq",
                      "org.opencontainers.image.version": row["version"],
                      "org.opencontainers.image.revision": OLD if fault == "wrong-sha" else row["source_sha"],
                      "org.marreq.contract.sha256": checksum((work / "assets" / row["contract_file"]).read_bytes())[7:]}
            assert component in COMPONENTS
            print(json.dumps([{"Architecture": "amd64", "Os": "linux", "Config": {"Labels": labels}}]))
        elif args[0] != "pull" and args[:2] != ["buildx", "prune"]:
            raise AssertionError(args)
    else:
        raise AssertionError(tool)
    write_json(state_path, state)


class Publication(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.work = self.root / "work"
        self.assets = self.work / "assets"
        self.assets.mkdir(parents=True)
        (self.work / "notes.md").write_text("Test notes\n")
        self.state_file = self.root / "state.json"
        self.mocks = self.root / "mocks"
        self.mocks.mkdir()
        for tool in ("gh", "git", "curl", "docker"):
            executable = self.mocks / tool
            executable.write_text(f'#!/bin/sh\nexec "{sys.executable}" "{Path(__file__).resolve()}" --fixture {tool} "$@"\n')
            executable.chmod(0o755)
        scripts = self.root / "scripts"
        scripts.mkdir()
        (scripts / "test-migration-image.sh").write_text("#!/bin/sh\nexit 0\n")
        (scripts / "test-migration-image.sh").chmod(0o755)
        (scripts / "test-artifact-image.py").write_text("pass\n")
        self.env = {**os.environ, "PATH": f"{self.mocks}:{os.environ['PATH']}",
                    "RELEASE_CLI": str(REPO / "target/debug/marreq-release"),
                    "GITHUB_REF_NAME": TAG, "RELEASE_HEAD": HEAD, "TAG_OBJECT": ANNOTATION,
                    "GITHUB_REPOSITORY": "MarreqSW/Marreq", "RELEASE_WORK": str(self.work),
                    "GITHUB_RUN_ID": "123", "GITHUB_ACTOR": "fixture",
                    "MARREQ_RELEASE_POLICY_TOKEN": "offline", "GH_TOKEN": "offline",
                    "MARREQ_RELEASE_ROOT": str(self.root), "FIXTURE_STATE": str(self.state_file)}

    def configure(self, builds, bootstrap=False):
        plan = {"tag": TAG, "head": HEAD, "previous": OLD, "bootstrap": bootstrap,
                "versions": {"marreq": TAG[1:]}, "selection": {}}
        state = {"aliases": {}, "blobs": {}, "tags": {}, "release": None, "builds": [], "promotions": []}
        for component in COMPONENTS:
            repo = image(component)
            build = component in builds
            source, version = (HEAD, "0.1.2") if build else (OLD, "0.1.1")
            tag = f"marreq-{component}-v{version}"
            file = f"{component}-contract.json"
            backend = component in ("cloud", "server")
            contract = {"format": "marreq.contract.v2", "component": "backend" if backend else component,
                        "version": version, "source_sha": source, "contract": {}}
            if backend:
                contract["variant"] = component
            write_json(self.assets / file, contract)
            body = json.dumps({"repository": repo, "version": version})
            digest = checksum(body.encode())
            if not build:
                state["tags"][tag] = source
                state["aliases"][f"{repo}:{version}"] = digest
                state["blobs"][f"{repo}@{digest}"] = body
            plan["versions"][component] = version
            plan["selection"][component] = {"version": version, "image": repo, "action": "build" if build else "reuse",
                                            "source_sha": source, "source_tag": tag, "contract_file": file,
                                            "retained_digest": None if build else digest,
                                            "retained_contract_sha256": None if build else checksum((self.assets / file).read_bytes())[7:]}
        write_json(self.work / "plan.json", plan)
        write_json(self.state_file, state)

    def state(self):
        return json.loads(self.state_file.read_text())

    def publish(self, expected=None, attempt=1, fault="", run_id="123"):
        result = subprocess.run(
            [REPO / "scripts/coordinated-release.sh", "publish"], cwd=self.root, capture_output=True, text=True,
            env={**self.env, "GITHUB_RUN_ATTEMPT": str(attempt), "GITHUB_RUN_ID": run_id, "FIXTURE_FAULT": fault},
            timeout=60,
        )
        output = result.stdout + result.stderr
        if expected is None:
            self.assertEqual(result.returncode, 0, output)
        else:
            self.assertNotEqual(result.returncode, 0, output)
            self.assertIn(expected, output)
            self.assertTrue(self.state()["release"] is None or self.state()["release"]["draft"])
        return self.state()

    def test_mcp_only_and_documentation_only(self):
        for builds in (("mcp",), ()):
            with self.subTest(builds=builds):
                self.configure(builds)
                state = self.publish()
                self.assertEqual(state["builds"], [image(key) for key in builds])
                data = json.loads((self.assets / "release-manifest.json").read_text())
                self.assertEqual(set(data["components"]), set(COMPONENTS))
                self.assertEqual(data["components"]["cloud"]["action"], "reuse")

    def test_shared_core_partial_promotion_resumes_without_overwrite(self):
        self.configure(("server", "cloud"))
        state = self.publish("", fault="promotion")
        self.assertEqual(len(state["promotions"]), 1)
        self.assertTrue(state["release"]["draft"])
        self.publish("unexpected GitHub Release", attempt=2, run_id="999")
        state = self.publish(attempt=3)
        self.assertCountEqual(state["builds"], [image("server"), image("cloud")])
        self.assertEqual(len(state["promotions"]), 2)
        self.assertFalse(state["release"]["draft"])
        self.assertEqual(self.publish(attempt=4), state)

    def test_bootstrap_recovers_from_asset_failure(self):
        self.configure(COMPONENTS, bootstrap=True)
        state = self.publish("", fault="asset")
        self.assertEqual(state["promotions"], [])
        self.assertTrue(state["release"]["draft"])
        state = self.publish(attempt=2)
        self.assertCountEqual(state["builds"], map(image, COMPONENTS))
        self.assertEqual(len(state["promotions"]), 4)

    def test_no_release_on_failed_build_digest_or_origin(self):
        for fault, expected in (("build", ""), ("missing-digest", ""), ("wrong-sha", "OCI identity"),
                                ("unprotected", "main protection"), ("stale", "no longer the tip"),
                                ("mutable", "enable immutable releases")):
            with self.subTest(fault=fault):
                self.configure(("mcp",))
                state = self.publish(expected, fault=fault)
                self.assertEqual(state["promotions"], [])
                self.assertIsNone(state["release"])

    def test_conflicts_and_missing_reused_image_fail_before_build(self):
        for conflict, expected in (("alias", "already occupied"), ("source", "already occupied"),
                                   ("missing", "is unavailable")):
            with self.subTest(conflict=conflict):
                self.configure(("mcp",))
                state = self.state()
                if conflict == "alias":
                    other = f"{image('cloud')}:0.1.1"
                    digest = state["aliases"][other]
                    state["aliases"][f"{image('mcp')}:0.1.2"] = digest
                    state["blobs"][f"{image('mcp')}@{digest}"] = state["blobs"][f"{image('cloud')}@{digest}"]
                elif conflict == "source":
                    state["tags"]["marreq-mcp-v0.1.2"] = OLD
                else:
                    digest = state["aliases"][f"{image('frontend')}:0.1.1"]
                    del state["blobs"][f"{image('frontend')}@{digest}"]
                write_json(self.state_file, state)
                self.assertEqual(self.publish(expected)["builds"], [])


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--fixture":
        fixture_command(sys.argv[2], sys.argv[3:])
    else:
        subprocess.run(["cargo", "build", "--locked", "-p", "marreq-release"], cwd=REPO, check=True)
        unittest.main()
