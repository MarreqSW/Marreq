# MCP changelog

## [Unreleased]

## [0.2.1] - 2026-11-24

### Added
- The MCP handshake advertises the packaged release version, and HTTP mode reports build identity at `/build-info` (#423).
- Added a retained MCP HTTP contract and backend compatibility range for checks against Server and Cloud (#424).

### Changed
- Aligned the MCP client's API types and HTTP behavior with the verified backend contract (#424).
- Updated locked npm dependencies, including security fixes.

## [0.2.0] - Bootstrap

- Adopt the version floor from the existing `marreq-mcp-v0.2.0` tag and GHCR image.
  The image lacks source revision metadata.
