# marreq-server

Self-hosted Marreq deployment binary for organizations that manage their own user accounts (admin-created users, no public registration). It composes shared business logic from `marreq-core` and adds server-only routes and the `Server` deployment-mode implementation on top.

See [../docs/developer/setup.md](../docs/developer/setup.md) for setup with and without Docker, and [../docs/developer/workspace-layout.md](../docs/developer/workspace-layout.md) for the deployment-mode differences.

## Running

```sh
cargo run -p marreq-server
```
