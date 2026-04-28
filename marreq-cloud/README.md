# marreq-cloud

`marreq-cloud` is the hosted SaaS binary for Marreq. It extends the shared `marreq-core` library with self-registration, email verification, a single environment-bootstrapped site admin, and personal per-user workspaces. Cloud-specific routes (public auth: register, verify-email, forgot-password, reset-password) and fairings (cloud_admin_bootstrap) are composed on top of the shared Rocket application via `marreq_core::app::build_with`.

```sh
cargo run -p marreq-cloud
```
