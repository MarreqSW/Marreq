#!/usr/bin/env python3
"""Check the real Compose graph and exercise production nginx with HTTP fixtures.

No database, developer .env, fixed host ports, or production resources are used.
Requires Docker Compose >= 2.35 (--no-env-resolution and !override),
Python 3, and the built SPA image.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent.parent
HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}))
# Allow CLI/Docker credentials, but never application settings from the caller.
ENV = {k: v for k, v in os.environ.items()
       if not k.startswith(('MARREQ_', 'COMPOSE_', 'ROCKET_', 'CSRF_'))}
ENV.update({
    'ROCKET_SECRET_KEY': 'ci-placeholder-rocket-secret-key-32charsxx',
    'MARREQ_PUBLIC_BASE_URL': 'https://example.test',
    'CSRF_ALLOWED_ORIGINS': 'https://example.test',
    'MARREQ_MCP_PUBLIC_URL': 'https://example.test/mcp',
    'MARREQ_MCP_ALLOWED_HOSTS': 'example.test',
    'MARREQ_MCP_AUDIT_SECRET': 'ci-placeholder-mcp-audit-secret-32chars',
    'MARREQ_CLOUD_PUBLIC_BASE_URL': 'https://cloud.example.test',
    'MARREQ_CLOUD_MCP_PUBLIC_URL': 'https://cloud.example.test/mcp',
    'MARREQ_CLOUD_MCP_ALLOWED_HOSTS': 'cloud.example.test',
    'MARREQ_CLOUD_CSRF_ALLOWED_ORIGINS': 'https://cloud.example.test',
    'MARREQ_MIGRATION_IMAGE': 'marreq-server:test',
    'MARREQ_MIGRATION_DATABASE_URL': 'postgres://test:test@db/test',
    'MARREQ_RECOVERY_ROOT': '/tmp/marreq-compose-test-recovery',
})


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, env=ENV, text=True).strip()


def compose(project, base, overlay=None, profile=False):
    cmd = ['docker', 'compose', '--env-file', '/dev/null', '-p', project,
           '-f', str(ROOT / 'docker' / base)]
    if overlay:
        cmd += ['-f', str(overlay)]
    if profile:
        cmd += ['--profile', 'cloud']
    return cmd


def validate_configs():
    for base in ('docker-compose.yml', 'docker-compose.light.yml'):
        for overlay in (None, 'docker-compose.dev.yml', 'docker-compose.ci.yml',
                        'docker-compose.prod.yml', 'docker-compose.migrate.yml'):
            for cloud in (False, True):
                cmd = compose('marreq-compose-check', base,
                              ROOT / 'docker' / overlay if overlay else None, cloud)
                # Do not read or print developer env_file contents.
                model = json.loads(run(*cmd, 'config', '--no-env-resolution', '--format', 'json'))
                services = model['services']
                for frontend, backend, mcp in (
                    ('frontend', 'marreq-server', 'mcp-server'),
                    ('frontend-cloud', 'marreq-cloud', 'mcp-cloud'),
                ):
                    if frontend not in services:
                        continue
                    deps = services[frontend].get('depends_on', {})
                    assert set(deps) == {backend}, (base, overlay, frontend, deps)
                    assert deps[backend]['condition'] == 'service_healthy'
                    assert not deps[backend].get('restart', False)
                    assert services[mcp]['depends_on'][backend]['condition'] == 'service_healthy'
                    assert not services[mcp].get('ports'), 'MCP must remain internal-only'
                print(f'CONFIG OK {base} + {overlay or "base"} cloud={cloud}', flush=True)


def poll(label, check, timeout=45):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            result = check()
            if result:
                return result
        except (OSError, ValueError, subprocess.CalledProcessError) as exc:
            last = exc
        time.sleep(0.25)  # Bounded polling of an observable condition, not a startup delay.
    raise AssertionError(f'Timed out: {label}; last error: {last}')


def inspect(cid):
    return json.loads(run('docker', 'inspect', cid))[0]


def snapshot(cid):
    info = inspect(cid)
    assert info['State']['Running'], (cid, info['State'])
    return (info['Id'], info['State']['StartedAt'], info['RestartCount'])


def request(origin, path, method='GET', body=None, headers=None):
    req = urllib.request.Request(origin + path, data=body, method=method, headers=headers or {})
    try:
        with HTTP.open(req, timeout=3) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as exc:
        return exc.code, exc.read()


def exercise(base, production, cloud, args, work):
    project = 'marreq-proxy-' + uuid.uuid4().hex[:12]
    frontend, backend, mcp, port = (
        ('frontend-cloud', 'marreq-cloud', 'mcp-cloud', 8001) if cloud else
        ('frontend', 'marreq-server', 'mcp-server', 8000))
    cmd = compose(project, base, ROOT / 'docker/docker-compose.prod.yml' if production else None, cloud)
    override = work / f'{project}.yml'
    fixture = ROOT / 'tests/docker/proxy-upstream.cjs'
    # Replace only expensive application processes; retain frontend config,
    # environment, dependency graph and Docker DNS from the real Compose files.
    override.write_text(f'''services:
  {frontend}:
    build: !reset null
    image: {json.dumps(args.frontend_image)}
    restart: "no"
    ports: !override ["127.0.0.1::80"]
  {backend}:
    build: !reset null
    image: {json.dumps(args.fixture_image)}
    env_file: !reset []
    depends_on: !reset {{}}
    restart: "no"
    entrypoint: [node, /fixture.cjs, "{port}"]
    command: !reset []
    ports: !reset []
    volumes: !override [{json.dumps(str(fixture) + ':/fixture.cjs:ro')}]
    healthcheck:
      test: [CMD, node, -e, "fetch('http://127.0.0.1:{port}/healthz').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"]
      interval: 1s
      timeout: 2s
      start_period: 0s
      retries: 1
  {mcp}:
    build: !reset null
    image: {json.dumps(args.fixture_image)}
    restart: "no"
    entrypoint: [node, /fixture.cjs, "3000"]
    command: !reset []
    volumes: [{json.dumps(str(fixture) + ':/fixture.cjs:ro')}]
    healthcheck:
      test: [CMD, node, -e, "fetch('http://127.0.0.1:3000/healthz').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"]
      interval: 1s
      timeout: 2s
      start_period: 0s
      retries: 1
''')
    cmd += ['-f', str(override)]
    holder = project + '-old-ip'

    def dc(*arguments):
        return run(*cmd, *arguments)

    def cid(service):
        return dc('ps', '-aq', service)

    def assert_unchanged(service, before):
        assert snapshot(cid(service)) == before, f'{service} recreated or restarted'

    try:
        # Deliberately do not use --no-deps here: prove frontend startup doesn't
        # transitively create MCP, and the required backend still becomes healthy.
        dc('up', '-d', '--no-build', frontend)
        assert not cid(mcp), 'Starting frontend unexpectedly created MCP'
        front_id = cid(frontend)
        run('docker', 'exec', front_id, 'nginx', '-t')
        binding = dc('port', frontend, '80')
        origin = 'http://' + binding
        front_before, backend_before = snapshot(front_id), snapshot(cid(backend))

        def available():
            for path in ('/', '/projects/fixture'):
                status, body = request(origin, path)
                if status != 200 or b'<html' not in body.lower():
                    return False
            for path in ('/api/auth/csrf?fixture=1', '/api/admin/backup',
                         '/oauth/authorize?fixture=1', '/.well-known/oauth-protected-resource/mcp'):
                status, body = request(origin, path, headers={'X-Forwarded-Proto': 'https'})
                if status != 200:
                    return False
                data = json.loads(body)
                assert data['path'] == path
                assert data['headers']['x-forwarded-proto'] == 'https'
            return True

        poll('SPA/API/OAuth with absent MCP', available)
        poll('absent MCP returns upstream error', lambda: request(origin, '/mcp')[0] == 502)
        print(f'{project}: absent MCP, nginx -t and SPA/API/OAuth OK; frontend={front_id[:12]}', flush=True)

        def routed(expected):
            status, body = request(origin, '/mcp?fixture=1', 'POST', b'{"fixture":true}', {
                'Authorization': 'Bearer disposable-fixture', 'Mcp-Session-Id': 'test-session',
                'Last-Event-ID': 'event-1', 'X-Forwarded-Proto': 'https',
                'Host': 'public.example.test', 'Content-Type': 'application/json',
            })
            if status != 200:
                return False
            data = json.loads(body)
            assert data['instance'] == expected, data
            assert data['path'] == '/mcp?fixture=1' and data['method'] == 'POST'
            assert data['body'] == '{"fixture":true}'
            for key, value in {'authorization': 'Bearer disposable-fixture',
                               'mcp-session-id': 'test-session', 'last-event-id': 'event-1',
                               'x-forwarded-proto': 'https', 'host': 'public.example.test',
                               'x-forwarded-host': 'public.example.test'}.items():
                assert data['headers'][key] == value, (key, data)
            assert data['headers']['x-real-ip'] and data['headers']['x-forwarded-for']
            return True

        dc('up', '-d', '--no-build', mcp)
        mcp_id = cid(mcp)
        poll('MCP starts after frontend', lambda: routed(inspect(mcp_id)['Config']['Hostname']))
        run('docker', 'exec', mcp_id, 'touch', '/tmp/unhealthy')
        poll('MCP unhealthy', lambda: inspect(mcp_id)['State']['Health']['Status'] == 'unhealthy')
        assert available() and request(origin, '/mcp')[0] == 503
        assert_unchanged(frontend, front_before)
        # Create a fresh frontend while MCP is unhealthy, without --no-deps:
        # Compose must still wait for the backend, but must not wait for MCP.
        mcp_before = snapshot(mcp_id)
        dc('rm', '-sf', frontend)
        dc('up', '-d', '--no-build', frontend)
        origin = 'http://' + dc('port', frontend, '80')
        poll('frontend created with unhealthy MCP', available)
        assert_unchanged(mcp, mcp_before)
        front_before = snapshot(cid(frontend))
        run('docker', 'exec', mcp_id, 'rm', '/tmp/unhealthy')
        dc('stop', mcp)
        assert available()
        poll('stopped MCP returns upstream error', lambda: request(origin, '/mcp')[0] == 502)
        dc('start', mcp)
        poll('MCP recovers after stop', lambda: routed(inspect(mcp_id)['Config']['Hostname']))
        dc('restart', mcp)
        poll('MCP recovers after restart', lambda: routed(inspect(mcp_id)['Config']['Hostname']))
        assert_unchanged(frontend, front_before)
        assert_unchanged(backend, backend_before)

        # Prime DNS, remove the container and reserve its old address so Docker
        # cannot accidentally reuse it and make a stale-DNS test falsely pass.
        old = inspect(mcp_id)
        network, endpoint = next(iter(old['NetworkSettings']['Networks'].items()))
        dc('rm', '-sf', mcp)
        run('docker', 'run', '-d', '--name', holder, '--network', network,
            '--ip', endpoint['IPAddress'], '--entrypoint', 'node', args.fixture_image,
            '-e', 'setInterval(()=>{}, 1000)')
        dc('up', '-d', '--no-build', mcp)
        new_id = cid(mcp)
        new = inspect(new_id)
        assert new_id != mcp_id
        assert new['NetworkSettings']['Networks'][network]['IPAddress'] != endpoint['IPAddress']
        poll('nginx re-resolves changed MCP IP', lambda: routed(new['Config']['Hostname']))
        assert available()
        assert_unchanged(frontend, front_before)
        assert_unchanged(backend, backend_before)
        print(f'{project}: MCP replacement {mcp_id[:12]} -> {new_id[:12]}, '
              f'IP {endpoint["IPAddress"]} -> {new["NetworkSettings"]["Networks"][network]["IPAddress"]}; '
              'frontend/backend unchanged', flush=True)

        dc('up', '-d', '--no-build', '--no-deps', '--force-recreate', mcp)
        updated_id = cid(mcp)
        assert updated_id != new_id
        poll('selective MCP update', lambda: routed(inspect(updated_id)['Config']['Hostname']))
        assert_unchanged(frontend, front_before)
        assert_unchanged(backend, backend_before)
        mcp_before = snapshot(updated_id)
        dc('restart', frontend)
        origin = 'http://' + dc('port', frontend, '80')
        poll('frontend restart', available)
        poll('MCP route after frontend restart', lambda: routed(inspect(updated_id)['Config']['Hostname']))
        assert_unchanged(mcp, mcp_before)
        assert_unchanged(backend, backend_before)
        print(f'RUNTIME OK {base} prod={production} cloud={cloud}: '
              'absent/unhealthy/stop/start/restart/changed-IP/update/reverse-restart', flush=True)
    except BaseException:
        subprocess.run([*cmd, 'logs', '--no-color', '--tail', '60'], cwd=ROOT, env=ENV)
        raise
    finally:
        subprocess.run(['docker', 'rm', '-f', holder], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run([*cmd, 'down', '--volumes', '--remove-orphans'], cwd=ROOT, env=ENV, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config-only', action='store_true')
    parser.add_argument('--frontend-image', default='marreq-frontend:ci')
    parser.add_argument('--fixture-image', default='node:22-alpine')
    args = parser.parse_args()
    validate_configs()
    if not args.config_only:
        with tempfile.TemporaryDirectory(prefix='marreq-proxy-') as temp:
            for base in ('docker-compose.yml', 'docker-compose.light.yml'):
                for production in (False, True):
                    for cloud in (False, True):
                        exercise(base, production, cloud, args, Path(temp))


if __name__ == '__main__':
    main()
