// SPDX-License-Identifier: MIT

//! Database bootstrap integration tests.
//!
//! # Purpose
//!
//! Verifies that the administrative bootstrap workflow initializes the
//! current schema and remains safe when executed repeatedly.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

/// Verifies database bootstrap can be executed repeatedly.
///
/// # Test Goal
///
/// Confirm that bootstrap applies the current SQLx migrations, verifies
/// the resulting schema, and succeeds when repeated against an
/// already-current database.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
///
/// # Expected Outcomes
///
/// - the first bootstrap succeeds;
/// - all expected application tables are verified;
/// - a second bootstrap succeeds without duplicating schema objects; and
/// - both executions report the same verified table count.
#[tokio::test]
async fn database_bootstrap_is_repeatable() {
    let database_url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL must be set for integration tests");

    let first = energy_store::bootstrap_database(&database_url)
        .await
        .expect("initial database bootstrap should succeed");

    assert_eq!(
        first.verified_tables, 16,
        "bootstrap should verify all expected application tables"
    );

    let second = energy_store::bootstrap_database(&database_url)
        .await
        .expect("repeated database bootstrap should succeed");

    assert_eq!(
        second.verified_tables, first.verified_tables,
        "repeated bootstrap should verify the same schema"
    );
}
