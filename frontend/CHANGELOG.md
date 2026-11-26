# Frontend changelog

## [Unreleased]

## [0.2.4] - 2026-11-26

### Changed
- Rebuild the Frontend image for the updated coordinated release workflow, which now deploys published images by digest (#450, #451).

## [0.2.3] - 2026-11-24

### Added
- Added a cache administration page for inspecting cache statistics and evicting cached entries (#250).
- Added build identity at `/build-info.json` and retained the Frontend HTTP contract for compatibility checks against Server and Cloud (#423).

### Changed
- The SPA can start when optional MCP is down, with MCP routing available when the upstream recovers (#436).
- Aligned API client types and compatibility checks with the shared HTTP contract (#424).
- Updated locked npm dependencies.

## [0.2.2] - Bootstrap

- Adopt the version floor from the existing `marreq-frontend-v0.2.2` tag and GHCR image.
