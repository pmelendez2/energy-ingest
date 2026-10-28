// SPDX-License-Identifier: MIT

//! Database installation and bootstrap support.
//!
//! # Purpose
//!
//! Provides the administrative database bootstrap workflow used to
//! initialize or update an `energy_ingest` MariaDB database.
//!
//! # Architecture
//!
//! Bootstrap uses the same embedded SQLx migration path as the
//! application. Migration SQL remains the authoritative schema
//! definition.
//!
//! Administrative credentials are supplied by the caller and are not
//! stored by this module.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, Pool};

/// Application tables expected after current migrations complete.
const EXPECTED_TABLES: &[&str] = &[
    "data_custodian",
    "acquisition_source",
    "ingestion_run",
    "import_document",
    "usage_point",
    "reading_type",
    "meter_reading",
    "interval_block",
    "interval_reading",
    "interval_reading_revision",
    "retrieval_coverage",
    "retrieval_coverage_revision",
    "retrieval_gap",
    "oauth_authorization",
    "physical_meter",
    "physical_meter_resource",
    "api_client",
];

/// Result of a successful database bootstrap operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabaseBootstrapResult {
    /// Number of required application tables verified after migration.
    pub verified_tables: usize,
}

/// Errors produced by database bootstrap.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseBootstrapError {
    /// MariaDB could not be reached with the supplied administrative
    /// connection configuration.
    ///
    /// The underlying error is retained as a source but omitted from
    /// the normal display message so connection configuration
    /// containing secrets is not accidentally printed.
    #[error("failed to connect to MariaDB")]
    Connect(#[source] sqlx::Error),

    /// SQLx could not apply or validate the embedded migration history.
    #[error("database migration failed: {0}")]
    Migration(#[source] sqlx::migrate::MigrateError),

    /// Schema verification could not query MariaDB metadata.
    #[error("failed to verify database schema: {0}")]
    Verification(#[source] sqlx::Error),

    /// A required application table is absent after migration.
    #[error("database bootstrap completed but required table `{0}` is missing")]
    MissingTable(&'static str),
}

/// Initializes or updates an `energy_ingest` database.
///
/// # Purpose
///
/// Connects to MariaDB, runs the application's embedded SQLx
/// migrations, and verifies that the expected application tables exist.
///
/// # Inputs
///
/// - `database_url`: Administrative MariaDB connection URL supplied by
///   the deployment or administrative environment.
///
/// # Returns
///
/// Returns a [`DatabaseBootstrapResult`] after migrations and
/// verification complete successfully.
///
/// # Errors
///
/// Returns [`DatabaseBootstrapError`] when:
///
/// - MariaDB cannot be reached;
/// - SQLx migration history is incompatible;
/// - a migration fails;
/// - schema verification fails; or
/// - an expected application table is missing.
///
/// # Notes
///
/// The operation is safe to repeat. SQLx applies only pending
/// migrations and validates previously applied migration history.
pub async fn bootstrap_database(
    database_url: &str,
) -> Result<DatabaseBootstrapResult, DatabaseBootstrapError> {
    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
        .map_err(DatabaseBootstrapError::Connect)?;

    crate::migrate(&pool).await.map_err(DatabaseBootstrapError::Migration)?;

    let verified_tables = verify_schema(&pool).await?;

    pool.close().await;

    Ok(DatabaseBootstrapResult { verified_tables })
}

/// Verifies the application tables expected after current migrations.
async fn verify_schema(
    pool: &Pool<MySql>,
) -> Result<usize, DatabaseBootstrapError> {
    for table_name in EXPECTED_TABLES {
        let table_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM information_schema.TABLES
            WHERE TABLE_SCHEMA = DATABASE()
              AND TABLE_NAME = ?
            "#,
        )
        .bind(table_name)
        .fetch_one(pool)
        .await
        .map_err(DatabaseBootstrapError::Verification)?;

        if table_count != 1 {
            return Err(DatabaseBootstrapError::MissingTable(table_name));
        }
    }

    Ok(EXPECTED_TABLES.len())
}
