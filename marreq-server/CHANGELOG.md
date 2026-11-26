# Server changelog

## [Unreleased]

## [0.2.3] - 2026-11-26

### Changed
- Update the bundled PostgreSQL migration runner to `diesel_migrations` 2.4.0 and Diesel 2.4.0 (#446).

## [0.2.2] - 2026-11-24

### Added
- Added exact version, source revision, and API contract identity to coordinated Server images. Build metadata identifies the self-hosted deployment variant (#423).
- Added a serialized migration command and a one-shot Compose migration job, with backup and restore-rehearsal gates for production migrations (#435).

### Changed
- Server startup validates the prepared database schema. Operators apply migrations explicitly, and container startup defaults to no demo seeding (#435).
- Isolated Server and Cloud image build inputs so the self-hosted build excludes Cloud sources (#440).
- Updated locked Rust dependencies, including security fixes.

### Fixed
- Cache statistics now reflect entries actually present in the shared cache.

## [0.2.1] - Bootstrap

- Adopt the version floor from the existing `marreq-server-v0.2.1` tag and GHCR image.
