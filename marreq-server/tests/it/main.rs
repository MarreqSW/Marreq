//! All integration tests in one binary: each former `tests/*.rs` file is a
//! module here, so the crate and its dependencies are linked once instead of
//! once per file. Filter with `cargo test --test it <module>`.

mod admin_users_integration_test;
mod server_mode_integration_test;
