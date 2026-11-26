# Cloud changelog

## [Unreleased]

## [0.1.2] - 2026-11-26

### Changed
- Update the bundled PostgreSQL migration runner to `diesel_migrations` 2.4.0 and Diesel 2.4.0 (#446).

## [0.1.1] - 2026-11-24

### Added
- First coordinated image for the hosted backend, which supports self-registration, email verification, and an initial site administrator.
- Added Cloud-specific build metadata and retained HTTP contract evidence for the Frontend and MCP deployment checks (#423).
- Added explicit, serialized database migrations with a production backup and isolated restore rehearsal before schema changes (#435).

### Changed
- Cloud startup validates the prepared schema. Operators apply migrations explicitly, and container startup defaults to no demo seeding (#435).
- Isolated the Cloud image build from Server sources and updated locked Rust dependencies (#440).

## [0.1.0] - Bootstrap

- Register the existing Cargo package version as a development baseline.
  Cloud publication could not be verified.
