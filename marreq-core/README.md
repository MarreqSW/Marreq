# marreq-core

`marreq-core` is the shared Rust library crate for the [Marreq](https://github.com/MarreqSW/Marreq)
requirements-management platform. It contains all domain models, Diesel persistence layer,
authentication primitives, shared Rocket routes, fairings, and services that are consumed by both
`marreq-server` (self-hosted deployment) and `marreq-cloud` (SaaS deployment). The overall
multi-crate architecture is described in the [workspace plan](../docs/developer/workspace-layout.md).

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
marreq-core = { path = "../marreq-core" }
```

For integration tests that need the mock repository, enable the `test-helpers` feature:

```toml
[dev-dependencies]
marreq-core = { path = "../marreq-core", features = ["test-helpers"] }
```
