// SPDX-License-Identifier: MIT

//! Administrative bootstrap CLI tests.
//!
//! # Purpose
//!
//! Verifies basic command-line behavior and ensures missing database
//! credentials fail without exposing secret values.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use std::process::Command;

/// Verifies the administrative CLI exposes bootstrap usage information.
///
/// # Test Goal
///
/// Confirm that `--help` documents the database bootstrap command and
/// required environment variable.
///
/// # Inputs
///
/// None.
///
/// # Expected Outcomes
///
/// The command exits successfully and describes both the bootstrap
/// command and `ENGY_DB_ADMIN_URL`.
#[test]
fn admin_cli_help_describes_database_bootstrap() {
    let output = Command::new(env!("CARGO_BIN_EXE_energy-store-admin"))
        .arg("--help")
        .output()
        .expect("administrative binary should execute");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("database bootstrap"));
    assert!(stdout.contains("ENGY_DB_ADMIN_URL"));
}

/// Verifies bootstrap refuses to run without administrative database
/// configuration.
///
/// # Test Goal
///
/// Confirm the command fails clearly when the deployment environment
/// has not provided the database administration URL.
///
/// # Inputs
///
/// `ENGY_DB_ADMIN_URL` is deliberately removed.
///
/// # Expected Outcomes
///
/// - the command exits unsuccessfully;
/// - stderr identifies the missing configuration variable.
#[test]
fn database_bootstrap_requires_admin_database_url() {
    let output = Command::new(env!("CARGO_BIN_EXE_energy-store-admin"))
        .args(["database", "bootstrap"])
        .env_remove("ENGY_DB_ADMIN_URL")
        .output()
        .expect("administrative binary should execute");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("ENGY_DB_ADMIN_URL"));
}
