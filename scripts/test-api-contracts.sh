#!/usr/bin/env bash
# Creates a disposable DB; ignores DATABASE_URL and .env. No production access.
set -euo pipefail
cd "$(dirname "$0")/.."
work=$(mktemp -d)
container="marreq-contract-$RANDOM-$$"
backend_pid=
cleanup() { if [[ -n "$backend_pid" ]]; then kill "$backend_pid" 2>/dev/null || true; wait "$backend_pid" 2>/dev/null || true; fi; docker rm -f "$container" >/dev/null 2>&1 || true; rm -rf "$work"; }
trap cleanup EXIT
# No bind mounts, persistent volumes or remotely supplied database URLs.
docker run --detach --rm --name "$container" \
  --tmpfs /var/lib/postgresql/data \
  -e POSTGRES_DB=marreq_contract_test -e POSTGRES_USER=contract_test \
  -e POSTGRES_PASSWORD=disposable-contract-test \
  -p 127.0.0.1::5432 pgvector/pgvector:pg17@sha256:ac08538c6f8b9904c33c8224c5e5706dbe760aca29db1d096972b4052c22a75d >/dev/null
ready=false
for attempt in {1..60}; do
  if docker exec "$container" pg_isready -U contract_test -d marreq_contract_test >/dev/null 2>&1; then ready=true; break; fi
  sleep 1
done
if [[ "$ready" != true ]]; then echo "isolated contract DB did not become ready" >&2; exit 1; fi
for migration in marreq-core/migrations/*/up.sql; do
  docker exec -i "$container" psql -v ON_ERROR_STOP=1 -U contract_test -d marreq_contract_test < "$migration" > "$work/migration.log" 2>&1 || { cat "$work/migration.log"; exit 1; }
done
port=$(docker port "$container" 5432/tcp)
port=${port##*:}
export MARREQ_TEST_DATABASE_URL="postgres://contract_test:disposable-contract-test@127.0.0.1:$port/marreq_contract_test"
export DATABASE_URL="$MARREQ_TEST_DATABASE_URL"
export MARREQ_CONTRACT_TRANSCRIPT="$work/exchanges.jsonl"
cargo test --locked -p marreq-core --no-default-features --test delegated_oauth_mcp_tools_postgres_integration --no-run --message-format=json > "$work/test-build.jsonl"
test_executable=$(node -e 'const fs=require("node:fs"); console.log(fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map(JSON.parse).find(r=>r.executable&&r.target.name==="delegated_oauth_mcp_tools_postgres_integration").executable)' "$work/test-build.jsonl")
# A clean working directory and environment prevent dotenv/SMTP/semantic-search
# configuration inherited from a developer machine from entering test processes.
(cd "$work" && env -i PATH="$PATH" LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}" \
  MARREQ_TEST_DATABASE_URL="$MARREQ_TEST_DATABASE_URL" \
  MARREQ_CONTRACT_TRANSCRIPT="$MARREQ_CONTRACT_TRANSCRIPT" \
  MARREQ_UI_MODE=api_only "$test_executable" --nocapture)
# Run the actual frontend transport and MCP HTTP client against a live socket.
# All database and service configuration is pinned to this ephemeral test stack.
npm run build --prefix mcp-server
cargo build --locked -p marreq-server -p marreq-cloud --message-format=json > "$work/server-build.jsonl"
frontend/node_modules/.bin/rolldown scripts/smoke-api-consumers.mjs --platform node --format esm --file "$work/smoke.mjs"
export ROCKET_DATABASES="{my_db={url=\"$DATABASE_URL\",pool_size=2}}"
export MARREQ_PUBLIC_BASE_URL=http://localhost:8080 MARREQ_MCP_PUBLIC_URL=http://localhost:8080/mcp
export MARREQ_MCP_AUDIT_SECRET=integration-test-audit-secret-32-bytes
for mode in server cloud; do
  executable=$(node -e 'const fs=require("node:fs"); console.log(fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map(JSON.parse).find(r=>r.executable&&r.target.name===process.argv[2]).executable)' "$work/server-build.jsonl" "marreq-$mode")
  backend_port=$(node -e 'const s=require("node:net").createServer();s.listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close()})')
  export MARREQ_CONTRACT_BACKEND_URL="http://127.0.0.1:$backend_port" MARREQ_CONTRACT_VARIANT="$mode"
  (cd "$work" && exec env -i PATH="$PATH" LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}" \
    DATABASE_URL="$DATABASE_URL" ROCKET_ADDRESS=127.0.0.1 ROCKET_PORT="$backend_port" \
    ROCKET_DATABASES="$ROCKET_DATABASES" MARREQ_PUBLIC_BASE_URL="$MARREQ_PUBLIC_BASE_URL" \
    MARREQ_MCP_PUBLIC_URL="$MARREQ_MCP_PUBLIC_URL" MARREQ_MCP_AUDIT_SECRET="$MARREQ_MCP_AUDIT_SECRET" \
    CSRF_ALLOWED_ORIGINS="$MARREQ_CONTRACT_BACKEND_URL" \
    MARREQ_AUTO_SEED=false MARREQ_UI_MODE=api_only "$executable") > "$work/backend.log" 2>&1 &
  backend_pid=$!
  ready=false
  for attempt in {1..60}; do
    if ! kill -0 "$backend_pid" 2>/dev/null; then cat "$work/backend.log"; exit 1; fi
    if curl --silent --fail "$MARREQ_CONTRACT_BACKEND_URL/api/meta/health" > /dev/null; then ready=true; break; fi
    sleep 1
  done
  if [[ "$ready" != true ]]; then cat "$work/backend.log"; exit 1; fi
  node "$work/smoke.mjs"
  kill "$backend_pid"
  wait "$backend_pid" || true
  backend_pid=
done
# Missing/skipped critical exchanges fail, not just malformed response bodies.
cargo run --quiet --locked -p marreq-contracts -- verify-traffic "$MARREQ_CONTRACT_TRANSCRIPT"
