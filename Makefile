# Marreq dev convenience targets. The Rust code lives in marreq-{core,server,cloud}
# crates within this Cargo workspace; this Makefile wires the most common commands.

.PHONY: help server cloud build test fmt lint clean docker-server docker-cloud compose-server compose-cloud frontend frontend-test frontend-test-coverage

help:           ## Show this help.
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  %-18s %s\n", $$1, $$2}'

server:         ## Run the self-hosted marreq-server binary.
	cargo run -p marreq-server

cloud:          ## Run the hosted marreq-cloud binary.
	cargo run -p marreq-cloud

build:          ## cargo build --workspace --release.
	cargo build --workspace --release

test:           ## Run the full Rust workspace test suite.
	cargo test --workspace

fmt:            ## Format the workspace.
	cargo fmt --all

lint:           ## Clippy the workspace.
	cargo clippy --workspace --all-targets -- -D warnings

clean:          ## cargo clean.
	cargo clean

docker-server:  ## Build the Docker image for marreq-server.
	docker build -f docker/Dockerfile --build-arg MARREQ_BIN=marreq-server -t marreq:server .

docker-cloud:   ## Build the Docker image for marreq-cloud.
	docker build -f docker/Dockerfile --build-arg MARREQ_BIN=marreq-cloud -t marreq:cloud .

compose-server: ## Bring up the self-hosted Docker stack (db, ollama, marreq-server, frontend, adminer).
	docker compose -f docker/docker-compose.yml up -d

compose-cloud:  ## Bring up the cloud stack (adds marreq-cloud via the `cloud` profile).
	docker compose -f docker/docker-compose.yml --profile cloud up -d

frontend:       ## Run the Vite dev server.
	cd frontend && npm run dev

frontend-test:  ## Run frontend SPA tests (vitest + RTL).
	cd frontend && npm test

frontend-test-coverage: ## Run frontend SPA tests with coverage.
	cd frontend && npm run test:coverage
