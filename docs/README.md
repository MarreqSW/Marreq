# Documentation

This repo’s documentation is organized by audience:

- **Users**: end-user workflows and UI instructions.
- **Developers**: setup, local dev, integrations, testing, and operational notes.
- **Architects**: system design, data model, and key technical decisions.

## Users

- [User manual](user-manual/user-manual.md)
- [Typical workflow](user-manual/workflow.md)
- [Migrating from IBM DOORS](user-manual/doors-to-marreq-migration.md)

## Developers

- [Backend layout & API-only mode](developer/backend-layout.md)
- [HTTP API contract (SPA / interchangeable clients)](../doc/API.md) — auth, CSRF, cookies; partial [OpenAPI](../doc/openapi.yaml)
- [Database setup](developer/database-setup.md)
- [MCP setup](developer/mcp-setup.md)
- [Semantic search](developer/semantic-search.md)
- [Ollama setup](developer/ollama-setup.md)
- [CSS style guide](developer/css-style-guide.md)
- [Approval workflow testing](developer/testing/approval-workflow-testing.md)
- [API test coverage analysis](developer/testing/api-test-coverage-analysis.md)
- [Test coverage recommendations](developer/testing/test-coverage-recommendations.md)

## Architects

- [Database schema (ER diagram)](architecture/database-schema.md)
- [Database models](architecture/database-models.md)
- [Status enums](architecture/status-enums.md)
- [Baselines UI suggestion](architecture/ui/baselines-ui-suggestion.md)

## Conventions

- Filenames are `lowercase-kebab-case.md`.
- Generated artifacts (e.g. HTML exports of the user manual) live under `docs/user-manual/generated/`.
