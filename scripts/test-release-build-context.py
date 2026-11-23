#!/usr/bin/env python3
"""Reproduce release context extraction under umask 077 and check permissions.

The coordinated publisher keeps umask 077 for credentials and journals. GNU tar
applies that umask by default for non-root extractors, which previously left
docker/docker-entrypoint.sh mode 0600. After Docker COPY and chmod +x the file
became 0711, which the non-root marreq user cannot read, so Server and Cloud
images failed at startup. This test runs a real git archive extraction.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parent.parent
ENTRYPOINT = Path("docker/docker-entrypoint.sh")
PUBLISHER = REPO / "scripts" / "coordinated-release.sh"


def mode(path: Path) -> int:
    return stat.S_IMODE(path.stat().st_mode)


def extract(destination: Path, *, preserve: bool) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    archive = subprocess.Popen(
        ["git", "archive", "HEAD"],
        cwd=REPO,
        stdout=subprocess.PIPE,
    )
    tar_cmd = ["tar", "-x", "-C", str(destination)]
    if preserve:
        tar_cmd.insert(1, "--same-permissions")
    extracted = subprocess.run(tar_cmd, stdin=archive.stdout, check=False)
    assert archive.stdout is not None
    archive.stdout.close()
    if archive.wait() != 0 or extracted.returncode != 0:
        raise RuntimeError("git archive | tar extraction failed")


def assert_readable_entrypoint(context: Path) -> None:
    path = context / ENTRYPOINT
    current = mode(path)
    # Git stores the script as a regular non-executable file (100644). The
    # Dockerfile adds +x after COPY. The release bug was losing group/other
    # read bits under umask 077 so the non-root runtime user could not read it.
    if current & stat.S_IRUSR == 0:
        raise AssertionError(f"{path} must be owner-readable, got {current:04o}")
    if current & (stat.S_IRGRP | stat.S_IROTH) == 0:
        raise AssertionError(
            f"{path} mode {current:04o} is not group/other-readable; "
            "chmod +x alone leaves non-root runtime users unable to read the script"
        )


class ReleaseBuildContext(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        previous = os.umask(0o077)
        self.addCleanup(os.umask, previous)
        self.work = self.root / "work"
        self.work.mkdir()
        self.secret = self.work / "release.token"
        self.secret.write_text("offline-secret\n")
        self.assets = self.work / "assets"
        self.assets.mkdir()
        self.context = self.work / "context"

    def test_publisher_uses_permission_preserving_extraction(self) -> None:
        text = PUBLISHER.read_text()
        self.assertIn("umask 077", text)
        self.assertRegex(
            text,
            r"git archive \"\$head\" \| tar --same-permissions -x -C \"\$work/context\"",
        )
        self.assertNotIn("docker buildx prune --all --force", text)

    def test_default_tar_under_umask_strips_entrypoint_reads(self) -> None:
        extract(self.context, preserve=False)
        path = self.context / ENTRYPOINT
        self.assertTrue(path.is_file())
        self.assertEqual(mode(path) & (stat.S_IRGRP | stat.S_IROTH), 0)
        with self.assertRaises(AssertionError):
            assert_readable_entrypoint(self.context)

    def test_same_permissions_keeps_entrypoint_readable(self) -> None:
        extract(self.context, preserve=True)
        assert_readable_entrypoint(self.context)
        self.assertEqual(mode(self.secret), 0o600)
        self.assertEqual(mode(self.assets), 0o700)
        self.assertEqual(mode(self.work), 0o700)
        dockerfile = self.context / "docker" / "Dockerfile"
        self.assertTrue(dockerfile.is_file())
        self.assertNotEqual(mode(dockerfile) & stat.S_IRUSR, 0)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "context",
        nargs="?",
        help="Existing extracted build context to validate (CI path)",
    )
    parser.add_argument(
        "--workspace",
        help="Release workspace that must retain restrictive permissions",
    )
    args = parser.parse_args(argv)
    if args.context:
        context = Path(args.context)
        assert_readable_entrypoint(context)
        if args.workspace:
            workspace = Path(args.workspace)
            for path in workspace.iterdir():
                if path.resolve() == context.resolve():
                    continue
                current = mode(path)
                if path.is_dir():
                    if current & (stat.S_IRWXG | stat.S_IRWXO):
                        raise SystemExit(f"{path} should stay private under umask 077, got {current:04o}")
                elif current & (stat.S_IRWXG | stat.S_IRWXO):
                    raise SystemExit(f"{path} should stay private under umask 077, got {current:04o}")
        print(f"Release build context permissions ok: {context / ENTRYPOINT}")
        return 0
    result = unittest.main(argv=[sys.argv[0]], exit=False)
    return 0 if result.result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
