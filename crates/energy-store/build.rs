// SPDX-License-Identifier: MIT

//! Cargo build configuration for `energy-store`.
//!
//! # Purpose
//!
//! Rebuild the crate when embedded SQL migration files change.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

/// Configures Cargo to watch the SQL migration directory.
///
/// # Purpose
///
/// Ensures the migration set embedded by `sqlx::migrate!()` stays synchronized
/// with files under `migrations/`.
///
/// # Inputs
///
/// Cargo supplies the build-script environment. This function has no explicit
/// parameters.
///
/// # Expected Outcomes
///
/// Cargo reruns this build script and rebuilds `energy-store` whenever a file
/// in the migration directory changes.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
