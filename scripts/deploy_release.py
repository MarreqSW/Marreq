#!/usr/bin/env python3
"""Selective production deployment for a coordinated Marreq product release.

Designed for the existing single-VPS Docker Compose host. Preserves PostgreSQL
and attachment volumes, never builds images on the host, and fails closed when
pending migrations would require coherent recovery that cannot be automated
safely on the current infrastructure.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

PRODUCT_TAG = re.compile(r"^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
DIGEST = re.compile(r"^sha256:(?!0{64}$)[0-9a-f]{64}$")
GIT_SHA = re.compile(r"^(?!0{40}$)[0-9a-f]{40}$")

COMPONENT_SERVICES = {
    "server": "marreq-server",
    "frontend": "frontend",
    "mcp": "mcp-server",
}
CANONICAL_IMAGES = {
    "server": "ghcr.io/marreqsw/marreq-server",
    "frontend": "ghcr.io/marreqsw/marreq-frontend",
    "mcp": "ghcr.io/marreqsw/marreq-mcp-server",
}
EVIDENCE_FILES = (
    "release-manifest.json",
    "server-provenance.json",
    "server-compatibility.json",
    "server-contract.json",
    "frontend-contract.json",
    "mcp-contract.json",
)


class DeployError(Exception):
    """Operator-visible deployment failure."""


class DeployContext:
    def __init__(self, args: argparse.Namespace):
        self.args = args
        self.repo = Path(os.environ.get("MARREQ_REPO_ROOT", "/home/marreq/Marreq")).resolve()
        self.project = os.environ.get("MARREQ_COMPOSE_PROJECT", "docker")
        self.lock_path = Path(
            os.environ.get("MARREQ_OPERATIONS_LOCK", "/home/marreq/.marreq-operations.lock")
        )
        self.expected_cluster = os.environ.get(
            "MARREQ_EXPECTED_PG_CLUSTER_ID", "7690530505020129316"
        )
        self.public_url = os.environ.get("MARREQ_PUBLIC_URL", "https://app.marreq.com").rstrip("/")
        self.min_disk_mb = int(os.environ.get("MARREQ_MIN_DISK_MB", "2048"))
        self.compose_files = [
            self.repo / "docker" / "docker-compose.light.github.yml",
            self.repo / "docker" / "docker-compose.persistence.yml",
            self.repo / "docker" / "docker-compose.release.yml",
        ]
        self.releases = self.repo / "releases"
        self.current_dir = self.releases / "current"
        self.previous_dir = self.releases / "previous"
        self.variant = "server"
        self.lock_fd = None
        self.dry_run = args.dry_run
        self.rollback = args.rollback
        self.tag = args.tag


def run(cmd: list[str], *, check: bool = True, capture: bool = True, env: dict | None = None,
        input_text: str | None = None, timeout: int | None = 120) -> subprocess.CompletedProcess:
    result = subprocess.run(
        cmd,
        check=False,
        capture_output=capture,
        text=True,
        env=env or os.environ.copy(),
        input=input_text,
        timeout=timeout,
    )
    if check and result.returncode != 0:
        stderr = (result.stderr or "").strip()
        stdout = (result.stdout or "").strip()
        detail = stderr or stdout or f"exit {result.returncode}"
        raise DeployError(f"command failed ({' '.join(cmd[:4])}…): {detail}")
    return result


def log(message: str) -> None:
    print(message, flush=True)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise DeployError(message)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag", help="Coordinated product release tag (vX.Y.Z)")
    parser.add_argument("--dry-run", action="store_true", help="Validate only; do not mutate")
    parser.add_argument(
        "--rollback",
        action="store_true",
        help="Restore the previously retained release override when schema-compatible",
    )
    return parser.parse_args(argv)


def acquire_lock(ctx: DeployContext) -> None:
    ctx.lock_path.parent.mkdir(parents=True, exist_ok=True)
    ctx.lock_fd = open(ctx.lock_path, "a+", encoding="utf-8")
    try:
        import fcntl

        fcntl.flock(ctx.lock_fd.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as exc:
        raise DeployError(
            f"another deploy or backup holds {ctx.lock_path}; retry later"
        ) from exc
    ctx.lock_fd.seek(0)
    ctx.lock_fd.truncate()
    ctx.lock_fd.write(f"deploy {ctx.tag} pid={os.getpid()} at={int(time.time())}\n")
    ctx.lock_fd.flush()


def release_lock(ctx: DeployContext) -> None:
    if ctx.lock_fd is None:
        return
    try:
        import fcntl

        fcntl.flock(ctx.lock_fd.fileno(), fcntl.LOCK_UN)
    finally:
        ctx.lock_fd.close()
        ctx.lock_fd = None


def compose_cmd(ctx: DeployContext, *args: str) -> list[str]:
    cmd = ["docker", "compose", "-p", ctx.project]
    for path in ctx.compose_files:
        cmd.extend(["-f", str(path)])
    cmd.extend(args)
    return cmd


def validate_tag(tag: str) -> None:
    require(bool(PRODUCT_TAG.fullmatch(tag)), f"invalid product release tag: {tag}")


def load_json(path: Path) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise DeployError(f"invalid JSON at {path}: {exc}") from exc
    require(isinstance(data, dict), f"expected object in {path}")
    return data


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def ensure_compose_layout(ctx: DeployContext) -> None:
    missing = [str(path) for path in ctx.compose_files[:2] if not path.is_file()]
    require(not missing, f"missing production Compose files: {', '.join(missing)}")
    release_override = ctx.compose_files[2]
    if not release_override.is_file():
        release_override.parent.mkdir(parents=True, exist_ok=True)
        release_override.write_text(
            "# Generated by scripts/deploy-release.sh — digest-pinned app images only.\n"
            "services: {}\n",
            encoding="utf-8",
        )


def verify_disk_space(ctx: DeployContext) -> None:
    usage = shutil.disk_usage(ctx.repo)
    free_mb = usage.free // (1024 * 1024)
    require(
        free_mb >= ctx.min_disk_mb,
        f"insufficient disk space: {free_mb} MiB free, need >= {ctx.min_disk_mb} MiB",
    )


def installed_state_path(ctx: DeployContext) -> Path:
    return ctx.current_dir / "installed-state.json"


def read_installed_state(ctx: DeployContext) -> dict | None:
    path = installed_state_path(ctx)
    if not path.is_file():
        return None
    return load_json(path)


def parse_version_tuple(tag: str) -> tuple[int, int, int]:
    require(bool(PRODUCT_TAG.fullmatch(tag)), f"invalid tag for ordering: {tag}")
    major, minor, patch = tag[1:].split(".")
    return int(major), int(minor), int(patch)


def reject_stale_request(ctx: DeployContext) -> None:
    if ctx.rollback:
        return
    state = read_installed_state(ctx)
    if not state:
        return
    current = state.get("product_tag")
    if not isinstance(current, str) or not PRODUCT_TAG.fullmatch(current):
        return
    require(
        parse_version_tuple(ctx.tag) >= parse_version_tuple(current),
        f"refusing stale/out-of-order deploy: requested {ctx.tag} < installed {current}",
    )


def fetch_release_evidence(ctx: DeployContext) -> Path:
    target = ctx.releases / ctx.tag
    target.mkdir(parents=True, mode=0o700, exist_ok=True)
    os.chmod(target, 0o700)
    manifest = target / "release-manifest.json"
    missing = [name for name in EVIDENCE_FILES if not (target / name).is_file()]
    if missing:
        log(f"downloading release evidence for {ctx.tag} (missing: {', '.join(missing)})")
        if shutil.which("gh"):
            run(
                [
                    "gh",
                    "release",
                    "download",
                    ctx.tag,
                    "--repo",
                    "MarreqSW/Marreq",
                    "--dir",
                    str(target),
                    "--clobber",
                ],
                capture=True,
            )
        else:
            release = github_api_json(f"repos/MarreqSW/Marreq/releases/tags/{ctx.tag}")
            assets = {asset["name"]: asset["browser_download_url"] for asset in release.get("assets", [])}
            for name in missing:
                url = assets.get(name)
                require(url, f"release asset {name} unavailable for download")
                run(["curl", "-fsSL", "-o", str(target / name), url])
    for name in EVIDENCE_FILES:
        path = target / name
        require(path.is_file(), f"missing release asset {name} under {target}")
    return target


def validate_manifest(evidence: Path) -> dict:
    manifest = load_json(evidence / "release-manifest.json")
    require(manifest.get("schema_version") == 1, "unsupported release-manifest schema")
    product = manifest.get("product")
    require(isinstance(product, dict), "manifest missing product")
    require(product.get("tag") == evidence.name, "manifest product.tag mismatch")
    require(GIT_SHA.fullmatch(str(product.get("source_sha", ""))), "invalid product source_sha")
    components = manifest.get("components")
    require(isinstance(components, dict), "manifest missing components")
    for name in COMPONENT_SERVICES:
        entry = components.get(name)
        require(isinstance(entry, dict), f"manifest missing component {name}")
        require(entry.get("image") == CANONICAL_IMAGES[name], f"untrusted image repo for {name}")
        require(DIGEST.fullmatch(str(entry.get("digest", ""))), f"invalid digest for {name}")
        expected_ref = f"{entry['image']}@{entry['digest']}"
        require(entry.get("reference") == expected_ref, f"reference mismatch for {name}")
        require(GIT_SHA.fullmatch(str(entry.get("source_sha", ""))), f"invalid source_sha for {name}")
    return manifest


def github_api_json(path: str) -> dict:
    """Fetch a GitHub API JSON object using gh when available, otherwise curl."""
    if shutil.which("gh"):
        result = run(["gh", "api", path])
        return json.loads(result.stdout)
    url = f"https://api.github.com/{path}"
    headers = ["-H", "Accept: application/vnd.github+json", "-H", "User-Agent: marreq-deploy"]
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if token:
        headers.extend(["-H", f"Authorization: Bearer {token}"])
    result = run(["curl", "-fsS", *headers, url])
    return json.loads(result.stdout)


def verify_github_release(ctx: DeployContext) -> None:
    release = github_api_json(f"repos/MarreqSW/Marreq/releases/tags/{ctx.tag}")
    require(release.get("draft") is False, "release is still a draft")
    require(release.get("prerelease") is False, "refusing prerelease deployment")
    # Immutable Releases API may be unavailable on older accounts; treat explicit false as fatal.
    immutable = release.get("immutable")
    if immutable is False:
        raise DeployError("release is not immutable")


def verify_evidence_tools(evidence: Path) -> None:
    scripts = Path(__file__).resolve().parent
    provenance = evidence / "server-provenance.json"
    compatibility = evidence / "server-compatibility.json"
    run([str(scripts / "check-deployment-provenance"), "--manifest", str(provenance), "--mode", "validate"])
    # Compatibility verifier needs the Rust binary in CI/dev; on VPS prefer pre-validated assets.
    if shutil.which("marreq-contracts") or (scripts.parent / "target/debug/marreq-contracts").is_file():
        verify = scripts / "check-api-compatibility.sh"
        run(["bash", str(verify), "--manifest", str(compatibility)])
    else:
        log("marreq-contracts unavailable on host; relying on retained release-gate compatibility assets")


def postgres_cluster_id(ctx: DeployContext) -> str:
    result = run(
        [
            "docker",
            "exec",
            f"{ctx.project}-db-1",
            "psql",
            "-U",
            "rust",
            "-d",
            "postgres",
            "-Atc",
            "SELECT system_identifier FROM pg_control_system();",
        ]
    )
    value = result.stdout.strip()
    require(value.isdigit(), f"unexpected PostgreSQL cluster identity: {value!r}")
    return value


def verify_persistence(ctx: DeployContext) -> None:
    cluster = postgres_cluster_id(ctx)
    require(
        cluster == ctx.expected_cluster,
        f"PostgreSQL cluster identity mismatch: observed {cluster}, expected {ctx.expected_cluster}",
    )
    inspect = run(
        [
            "docker",
            "inspect",
            "--format",
            "{{json .Mounts}}",
            f"{ctx.project}-db-1",
        ]
    )
    mounts = json.loads(inspect.stdout)
    require(isinstance(mounts, list), "invalid db mount evidence")
    names = {mount.get("Name") for mount in mounts if isinstance(mount, dict)}
    # Accept either the intentional long anonymous volume name or the documented external alias.
    require(
        any(isinstance(name, str) and len(name) >= 12 for name in names),
        "db persistent volume mount missing",
    )
    att = run(
        [
            "docker",
            "inspect",
            "--format",
            "{{json .Mounts}}",
            f"{ctx.project}-marreq-server-1",
        ]
    )
    server_mounts = json.loads(att.stdout)
    attachment_mounts = [
        mount
        for mount in server_mounts
        if isinstance(mount, dict) and mount.get("Destination") == "/var/lib/marreq/attachments"
    ]
    require(attachment_mounts, "attachment volume is not mounted on marreq-server")


def service_image_ref(ctx: DeployContext, service: str) -> str | None:
    result = run(
        [
            "docker",
            "ps",
            "-a",
            "--no-trunc",
            "--filter",
            f"label=com.docker.compose.project={ctx.project}",
            "--filter",
            f"label=com.docker.compose.service={service}",
            "--format",
            "{{.ID}}",
        ]
    )
    container_ids = [line.strip() for line in result.stdout.splitlines() if line.strip()]
    if not container_ids:
        return None
    require(len(container_ids) == 1, f"ambiguous containers for service {service}")
    inspected = run(["docker", "inspect", "--", container_ids[0]])
    data = json.loads(inspected.stdout)[0]
    configured = data["Config"]["Image"]
    image_id = data["Image"]
    image = json.loads(run(["docker", "image", "inspect", "--", image_id]).stdout)[0]
    refs = image.get("RepoDigests") or []
    if configured in refs:
        return configured
    if DIGEST.fullmatch(configured or ""):
        return None
    # Prefer exact digest-pinned configured reference when present.
    if "@sha256:" in (configured or ""):
        return configured
    for ref in refs:
        if "@sha256:" in ref:
            return ref
    return configured


def desired_references(manifest: dict) -> dict[str, str]:
    return {
        component: manifest["components"][component]["reference"]
        for component in COMPONENT_SERVICES
    }


def changed_services(ctx: DeployContext, manifest: dict) -> list[str]:
    desired = desired_references(manifest)
    changed = []
    for component, service in COMPONENT_SERVICES.items():
        current = service_image_ref(ctx, service)
        if current != desired[component]:
            changed.append(service)
    return changed


def load_existing_release_override(path: Path) -> dict:
    if not path.is_file():
        return {"services": {}}
    text = path.read_text(encoding="utf-8").strip()
    if not text:
        return {"services": {}}
    if text.startswith("{"):
        data = json.loads(text)
        require(isinstance(data, dict), "invalid release override JSON")
        data.setdefault("services", {})
        return data
    # Minimal YAML subset reader for existing production overrides: prefer PyYAML if present.
    try:
        import yaml  # type: ignore
    except ImportError as exc:
        raise DeployError(
            "PyYAML required to preserve an existing YAML release override; "
            "install python3-yaml on the host"
        ) from exc
    data = yaml.safe_load(text) or {}
    require(isinstance(data, dict), "invalid release override YAML")
    data.setdefault("services", {})
    require(isinstance(data["services"], dict), "invalid release override services")
    return data


def render_release_override(manifest: dict, existing: dict | None = None) -> str:
    """Update only image pins; preserve host-local service customizations."""
    document = json.loads(json.dumps(existing or {"services": {}}))
    services = document.setdefault("services", {})
    require(isinstance(services, dict), "invalid release override services")
    for component, service in COMPONENT_SERVICES.items():
        entry = services.get(service)
        if not isinstance(entry, dict):
            entry = {}
            services[service] = entry
        entry["image"] = manifest["components"][component]["reference"]
        entry["pull_policy"] = "never"
        # Compose merge: clear any inherited build from base files when possible.
        entry["build"] = None
    # Drop legacy invalid top-level keys from earlier generator versions.
    document.pop("_comment", None)
    # Compose only accepts unknown top-level keys under the x- extension namespace.
    document["x-marreq-release-comment"] = (
        "Generated by scripts/deploy-release.sh; image digests are managed automatically. "
        "Preserve non-image service keys."
    )
    # Emit JSON so we do not depend on PyYAML for writes and keep stable formatting.
    return json.dumps(document, indent=2, sort_keys=True) + "\n"


def write_release_override(ctx: DeployContext, manifest: dict) -> None:
    path = ctx.compose_files[2]
    existing = load_existing_release_override(path)
    content = render_release_override(manifest, existing)
    if ctx.dry_run:
        log(f"dry-run: would write {path}")
        return
    tmp = path.with_suffix(".yml.tmp")
    tmp.write_text(content, encoding="utf-8")
    os.chmod(tmp, 0o644)
    tmp.replace(path)


def retain_previous(ctx: DeployContext) -> None:
    if ctx.dry_run:
        return
    ctx.previous_dir.mkdir(parents=True, mode=0o700, exist_ok=True)
    os.chmod(ctx.previous_dir, 0o700)
    current_override = ctx.compose_files[2]
    if current_override.is_file():
        shutil.copy2(current_override, ctx.previous_dir / "docker-compose.release.yml")
    state = installed_state_path(ctx)
    if state.is_file():
        shutil.copy2(state, ctx.previous_dir / "installed-state.json")
    current_evidence = ctx.current_dir / "release-manifest.json"
    if current_evidence.is_file():
        shutil.copy2(current_evidence, ctx.previous_dir / "release-manifest.json")


def record_installed(ctx: DeployContext, manifest: dict, changed: list[str]) -> None:
    if ctx.dry_run:
        return
    ctx.current_dir.mkdir(parents=True, mode=0o700, exist_ok=True)
    os.chmod(ctx.current_dir, 0o700)
    evidence = ctx.releases / ctx.tag
    for name in EVIDENCE_FILES:
        src = evidence / name
        if src.is_file():
            shutil.copy2(src, ctx.current_dir / name)
    state = {
        "product_tag": ctx.tag,
        "product_version": manifest["product"]["version"],
        "source_sha": manifest["product"]["source_sha"],
        "recorded_at_unix": int(time.time()),
        "changed_services": changed,
        "components": {
            component: {
                "service": service,
                "version": manifest["components"][component]["version"],
                "source_sha": manifest["components"][component]["source_sha"],
                "reference": manifest["components"][component]["reference"],
            }
            for component, service in COMPONENT_SERVICES.items()
        },
    }
    path = installed_state_path(ctx)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.chmod(tmp, 0o600)
    tmp.replace(path)


def migration_pending(ctx: DeployContext, manifest: dict) -> bool:
    """Return True when candidate server image reports pending migrations.

    Uses `marreq-migrations check` inside the candidate image against the live
    database URL available to the running server container. A non-zero exit with
    explicit pending signal fails closed; successful check means deploy may proceed.
    """
    reference = manifest["components"]["server"]["reference"]
    # Discover DATABASE_URL from the running server without printing it.
    probe = run(
        [
            "docker",
            "inspect",
            "--format",
            '{{range .Config.Env}}{{println .}}{{end}}',
            f"{ctx.project}-marreq-server-1",
        ]
    )
    database_url = None
    for line in probe.stdout.splitlines():
        if line.startswith("DATABASE_URL="):
            database_url = line.split("=", 1)[1]
            break
    require(database_url, "DATABASE_URL unavailable on running marreq-server")
    network = f"{ctx.project}_default"
    result = run(
        [
            "docker",
            "run",
            "--rm",
            "--network",
            network,
            "-e",
            "DATABASE_URL",
            "--entrypoint",
            "marreq-migrations",
            reference,
            "check",
        ],
        check=False,
        env={**os.environ, "DATABASE_URL": database_url},
        timeout=180,
    )
    if result.returncode == 0:
        return False
    combined = f"{result.stdout}\n{result.stderr}".lower()
    if "pending" in combined or "not applied" in combined or "migration" in combined:
        return True
    raise DeployError(
        "marreq-migrations check failed without a clear pending/compatible signal; "
        "refusing to mutate production"
    )


def pull_images(ctx: DeployContext, references: list[str]) -> None:
    for reference in references:
        if ctx.dry_run:
            log(f"dry-run: would pull {reference}")
            continue
        run(["docker", "pull", reference], timeout=600)


def recreate_service(ctx: DeployContext, service: str) -> None:
    if ctx.dry_run:
        log(f"dry-run: would recreate {service}")
        return
    run(
        compose_cmd(
            ctx,
            "up",
            "-d",
            "--no-deps",
            "--no-build",
            "--pull",
            "never",
            "--wait",
            "--force-recreate",
            service,
        ),
        timeout=300,
    )


def wait_healthy(ctx: DeployContext, service: str) -> None:
    if ctx.dry_run:
        return
    container = f"{ctx.project}-{service}-1"
    deadline = time.time() + 180
    while time.time() < deadline:
        result = run(
            ["docker", "inspect", "--format", "{{.State.Status}} {{if .State.Health}}{{.State.Health.Status}}{{end}}", container],
            check=False,
        )
        text = result.stdout.strip()
        if text.startswith("running") and ("healthy" in text or text == "running"):
            # Services without a healthcheck report only "running".
            if "health" not in text.lower() or "healthy" in text:
                return
        time.sleep(2)
    raise DeployError(f"service {service} did not become healthy in time")


def verify_runtime(ctx: DeployContext, manifest: dict, services: list[str]) -> None:
    if ctx.dry_run:
        log("dry-run: skipping runtime identity checks")
        return
    if "marreq-server" in services or not services:
        build = run(
            [
                "docker",
                "exec",
                f"{ctx.project}-marreq-server-1",
                "curl",
                "-fsS",
                "--noproxy",
                "*",
                "--max-time",
                "5",
                "http://127.0.0.1:8000/api/meta/build",
            ]
        )
        meta = json.loads(build.stdout)
        server = manifest["components"]["server"]
        require(meta.get("backend_version") == server["version"], "backend runtime version mismatch")
        require(meta.get("backend_git_sha") == server["source_sha"], "backend source SHA mismatch")
        require(meta.get("deployment_mode") == "server", "unexpected deployment mode")
    if "frontend" in services or not services:
        info = run(
            [
                "docker",
                "exec",
                f"{ctx.project}-frontend-1",
                "wget",
                "-Y",
                "off",
                "-qO-",
                "-T",
                "5",
                "http://127.0.0.1/build-info.json",
            ]
        )
        frontend = json.loads(info.stdout)
        expected = manifest["components"]["frontend"]
        require(frontend.get("version") == expected["version"], "frontend runtime version mismatch")
        require(frontend.get("source_sha") == expected["source_sha"], "frontend source SHA mismatch")
    if "mcp-server" in services or not services:
        mcp = run(
            [
                "docker",
                "exec",
                f"{ctx.project}-mcp-server-1",
                "node",
                "-e",
                "fetch('http://127.0.0.1:3000/build-info',{signal:AbortSignal.timeout(5000),redirect:'error'})"
                ".then(r=>{if(!r.ok)throw Error();return r.text()}).then(t=>process.stdout.write(t))"
                ".catch(()=>process.exit(1))",
            ]
        )
        payload = json.loads(mcp.stdout)
        expected = manifest["components"]["mcp"]
        require(payload.get("version") == expected["version"], "MCP runtime version mismatch")
        require(payload.get("source_sha") == expected["source_sha"], "MCP source SHA mismatch")
    public = run(["curl", "-fsS", "-o", "/dev/null", "-w", "%{http_code}", f"{ctx.public_url}/login"])
    require(public.stdout.strip() == "200", f"public login endpoint unhealthy: HTTP {public.stdout.strip()}")
    # Provenance inspect when the checker and contracts are present locally.
    scripts = Path(__file__).resolve().parent
    provenance = ctx.releases / ctx.tag / "server-provenance.json"
    if provenance.is_file():
        result = run(
            [
                str(scripts / "check-deployment-provenance"),
                "--manifest",
                str(provenance),
                "--mode",
                "inspect",
                "--compose-project",
                ctx.project,
            ],
            check=False,
        )
        if result.returncode == 1:
            raise DeployError("deployment provenance drift detected after update")
        if result.returncode not in (0, 2):
            raise DeployError("deployment provenance inspect failed unexpectedly")
        if result.returncode == 2:
            log("warning: provenance inspect unverified; runtime checks already passed")


def deploy_from_manifest(ctx: DeployContext, manifest: dict) -> int:
    changed = changed_services(ctx, manifest)
    if not changed:
        log(f"installed state already matches {ctx.tag}; no-op")
        record_installed(ctx, manifest, [])
        verify_runtime(ctx, manifest, [])
        return 0

    backend_changing = "marreq-server" in changed
    if backend_changing and migration_pending(ctx, manifest):
        raise DeployError(
            "pending database migrations detected. Automatic apply is disabled on this "
            "host because coherent recovery (quiesce → marreq-backup → isolated restore "
            "rehearsal → migrate) requires operator supervision. Stop writers, create a "
            "coherent recovery bundle, rehearse restore, apply migrations with "
            "docker-compose.migrate.yml, then re-run this deploy."
        )

    retain_previous(ctx)
    write_release_override(ctx, manifest)
    refs = [manifest["components"][component]["reference"]
            for component, service in COMPONENT_SERVICES.items()
            if service in changed]
    pull_images(ctx, refs)

    # Recreate one service at a time (limited RAM). Prefer backend before dependents.
    order = [s for s in ("marreq-server", "mcp-server", "frontend") if s in changed]
    completed = []
    try:
        for service in order:
            recreate_service(ctx, service)
            wait_healthy(ctx, service)
            completed.append(service)
    except DeployError:
        log(f"partial deployment failure after updating: {', '.join(completed) or '(none)'}")
        raise

    verify_runtime(ctx, manifest, changed)
    verify_persistence(ctx)
    record_installed(ctx, manifest, changed)
    log(f"deployed {ctx.tag}; updated services: {', '.join(changed)}")
    return 0


def rollback(ctx: DeployContext) -> int:
    previous_override = ctx.previous_dir / "docker-compose.release.yml"
    previous_state = ctx.previous_dir / "installed-state.json"
    require(previous_override.is_file(), "no retained previous release override")
    require(previous_state.is_file(), "no retained previous installed-state")
    state = load_json(previous_state)
    previous_tag = state.get("product_tag")
    require(isinstance(previous_tag, str), "previous installed-state missing product_tag")
    evidence = ctx.releases / previous_tag
    if (evidence / "release-manifest.json").is_file():
        manifest = validate_manifest(evidence)
    else:
        # Reconstruct a minimal manifest from retained state.
        components = {}
        for component, service in COMPONENT_SERVICES.items():
            entry = state["components"][component]
            digest = entry["reference"].split("@", 1)[1]
            components[component] = {
                "image": CANONICAL_IMAGES[component],
                "digest": digest,
                "reference": entry["reference"],
                "version": entry["version"],
                "source_sha": entry["source_sha"],
            }
        manifest = {
            "schema_version": 1,
            "product": {
                "tag": previous_tag,
                "version": previous_tag[1:],
                "source_sha": state.get("source_sha", entry["source_sha"]),
            },
            "components": components,
        }
    ctx.tag = previous_tag
    # Schema-aware gate for both dry-run and real rollback. Never reverse migrations.
    # Using the retained previous server image, refuse rollback when that image would
    # still see pending/incompatible history against the live database.
    log("validating schema compatibility before application rollback")
    if migration_pending(ctx, manifest):
        raise DeployError(
            "refusing application rollback: retained previous server image is not "
            "compatible with the current database schema (pending or failed "
            "marreq-migrations check). Prefer forward recovery; never restore "
            "PostgreSQL or run down migrations automatically."
        )
    verify_persistence(ctx)
    if ctx.dry_run:
        changed = changed_services(ctx, manifest)
        log(
            f"dry-run: would roll back to {previous_tag}; "
            f"services to update: {', '.join(changed) or '(none)'}"
        )
        return 0
    shutil.copy2(previous_override, ctx.compose_files[2])
    return deploy_from_manifest(ctx, manifest)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    ctx = DeployContext(args)
    validate_tag(ctx.tag)
    try:
        acquire_lock(ctx)
        ensure_compose_layout(ctx)
        verify_disk_space(ctx)
        reject_stale_request(ctx)
        if ctx.rollback:
            return rollback(ctx)
        verify_github_release(ctx)
        evidence = fetch_release_evidence(ctx)
        manifest = validate_manifest(evidence)
        verify_evidence_tools(evidence)
        verify_persistence(ctx)
        return deploy_from_manifest(ctx, manifest)
    except DeployError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    finally:
        release_lock(ctx)


if __name__ == "__main__":
    sys.exit(main())
