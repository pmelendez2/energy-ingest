// SPDX-License-Identifier: MIT

//! Integration tests for database migrations and schema constraints.
//!
//! # Purpose
//!
//! Verifies that the complete migration set creates the expected tables and
//! enforces the key relationships and constraints defined by the schema.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use sqlx::{MySql, Pool};

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
    "api_client",
];

/// Verifies schema creation and key database constraints.
///
/// # Test Goal
///
/// Confirm that the current SQL migration set creates the expected database
/// model and rejects invalid or duplicate relationships.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - SQL migrations embedded by `energy-store`.
///
/// # Expected Outcomes
///
/// - every table in `EXPECTED_TABLES` exists;
/// - valid source, Green Button, reading, coverage, and OAuth records succeed;
/// - duplicate identities and invalid relationships are rejected; and
/// - all test changes are rolled back.
#[tokio::test]
async fn migration_creates_and_enforces_canonical_schema() {
    let database_url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL must be set for database integration tests");

    let pool = Pool::<MySql>::connect(&database_url)
        .await
        .expect("test database connection should succeed");

    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    verify_expected_tables(&pool).await;

    let mut transaction =
        pool.begin().await.expect("test transaction should begin");

    /*
     * Ingestion source tracking.
     */

    let custodian = sqlx::query(
        r#"
        INSERT INTO data_custodian (name)
        VALUES (?)
        "#,
    )
    .bind("integration-test-custodian")
    .execute(&mut *transaction)
    .await
    .expect("data custodian insert should succeed");

    let custodian_id = custodian.last_insert_id();

    let duplicate_custodian = sqlx::query(
        r#"
        INSERT INTO data_custodian (name)
        VALUES (?)
        "#,
    )
    .bind("integration-test-custodian")
    .execute(&mut *transaction)
    .await;

    assert!(
        duplicate_custodian.is_err(),
        "duplicate data custodian should be rejected"
    );

    let source = sqlx::query(
        r#"
        INSERT INTO acquisition_source (
            data_custodian_id,
            name
        )
        VALUES (?, ?)
        "#,
    )
    .bind(custodian_id)
    .bind("integration-test-source")
    .execute(&mut *transaction)
    .await
    .expect("acquisition source insert should succeed");

    let source_id = source.last_insert_id();

    let duplicate_source = sqlx::query(
        r#"
        INSERT INTO acquisition_source (
            data_custodian_id,
            name
        )
        VALUES (?, ?)
        "#,
    )
    .bind(custodian_id)
    .bind("integration-test-source")
    .execute(&mut *transaction)
    .await;

    assert!(
        duplicate_source.is_err(),
        "duplicate acquisition source should be rejected"
    );

    let invalid_source = sqlx::query(
        r#"
        INSERT INTO acquisition_source (
            data_custodian_id,
            name
        )
        VALUES (?, ?)
        "#,
    )
    .bind(u64::MAX)
    .bind("invalid-source")
    .execute(&mut *transaction)
    .await;

    assert!(
        invalid_source.is_err(),
        "acquisition source with invalid custodian should be rejected"
    );

    let ingestion_run = sqlx::query(
        r#"
        INSERT INTO ingestion_run (
            acquisition_source_id,
            started_at
        )
        VALUES (?, UTC_TIMESTAMP(6))
        "#,
    )
    .bind(source_id)
    .execute(&mut *transaction)
    .await
    .expect("ingestion run insert should succeed");

    let ingestion_run_id = ingestion_run.last_insert_id();

    /*
     * Canonical Green Button entities.
     */

    let usage_point = sqlx::query(
        r#"
        INSERT INTO usage_point (
            acquisition_source_id,
            external_id
        )
        VALUES (?, ?)
        "#,
    )
    .bind(source_id)
    .bind("integration-test-usage-point")
    .execute(&mut *transaction)
    .await
    .expect("usage point insert should succeed");

    let usage_point_id = usage_point.last_insert_id();

    let duplicate_usage_point = sqlx::query(
        r#"
        INSERT INTO usage_point (
            acquisition_source_id,
            external_id
        )
        VALUES (?, ?)
        "#,
    )
    .bind(source_id)
    .bind("integration-test-usage-point")
    .execute(&mut *transaction)
    .await;

    assert!(
        duplicate_usage_point.is_err(),
        "duplicate usage point should be rejected"
    );

    let reading_type = sqlx::query(
        r#"
        INSERT INTO reading_type (
            acquisition_source_id,
            external_id
        )
        VALUES (?, ?)
        "#,
    )
    .bind(source_id)
    .bind("integration-test-reading-type")
    .execute(&mut *transaction)
    .await
    .expect("reading type insert should succeed");

    let reading_type_id = reading_type.last_insert_id();

    let meter_reading = sqlx::query(
        r#"
        INSERT INTO meter_reading (
            usage_point_id,
            reading_type_id,
            external_id
        )
        VALUES (?, ?, ?)
        "#,
    )
    .bind(usage_point_id)
    .bind(reading_type_id)
    .bind("integration-test-meter-reading")
    .execute(&mut *transaction)
    .await
    .expect("meter reading insert should succeed");

    let meter_reading_id = meter_reading.last_insert_id();

    let interval_block = sqlx::query(
        r#"
        INSERT INTO interval_block (
            meter_reading_id,
            external_id,
            interval_start,
            interval_duration_seconds
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(meter_reading_id)
    .bind("integration-test-interval-block")
    .bind("2026-01-01 00:00:00.000000")
    .bind(3600_u32)
    .execute(&mut *transaction)
    .await
    .expect("interval block insert should succeed");

    let interval_block_id = interval_block.last_insert_id();

    /*
     * Canonical interval readings and revision history.
     */

    let interval_reading = sqlx::query(
        r#"
        INSERT INTO interval_reading (
            meter_reading_id,
            interval_start,
            interval_duration_seconds
        )
        VALUES (?, ?, ?)
        "#,
    )
    .bind(meter_reading_id)
    .bind("2026-01-01 00:00:00.000000")
    .bind(900_u32)
    .execute(&mut *transaction)
    .await
    .expect("interval reading insert should succeed");

    let interval_reading_id = interval_reading.last_insert_id();

    let duplicate_interval_reading = sqlx::query(
        r#"
        INSERT INTO interval_reading (
            meter_reading_id,
            interval_start,
            interval_duration_seconds
        )
        VALUES (?, ?, ?)
        "#,
    )
    .bind(meter_reading_id)
    .bind("2026-01-01 00:00:00.000000")
    .bind(900_u32)
    .execute(&mut *transaction)
    .await;

    assert!(
        duplicate_interval_reading.is_err(),
        "duplicate canonical interval reading should be rejected"
    );

    let first_revision = sqlx::query(
        r#"
        INSERT INTO interval_reading_revision (
            interval_reading_id,
            ingestion_run_id,
            interval_block_id,
            source_external_id,
            reading_value
        )
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(interval_reading_id)
    .bind(ingestion_run_id)
    .bind(interval_block_id)
    .bind("integration-test-revision-1")
    .bind(1250_i64)
    .execute(&mut *transaction)
    .await
    .expect("first interval revision should succeed");

    let first_revision_id = first_revision.last_insert_id();

    // One ingestion run may preserve multiple observed revisions.
    let second_revision = sqlx::query(
        r#"
        INSERT INTO interval_reading_revision (
            interval_reading_id,
            ingestion_run_id,
            interval_block_id,
            source_external_id,
            reading_value
        )
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(interval_reading_id)
    .bind(ingestion_run_id)
    .bind(interval_block_id)
    .bind("integration-test-revision-2")
    .bind(1300_i64)
    .execute(&mut *transaction)
    .await
    .expect("second revision from the same ingestion run should succeed");

    assert!(
        second_revision.last_insert_id() != first_revision_id,
        "distinct revisions should receive distinct identities"
    );

    sqlx::query(
        r#"
        UPDATE interval_reading
        SET accepted_revision_id = ?
        WHERE id = ?
        "#,
    )
    .bind(first_revision_id)
    .bind(interval_reading_id)
    .execute(&mut *transaction)
    .await
    .expect("reading should accept one of its own revisions");

    let other_reading = sqlx::query(
        r#"
        INSERT INTO interval_reading (
            meter_reading_id,
            interval_start,
            interval_duration_seconds
        )
        VALUES (?, ?, ?)
        "#,
    )
    .bind(meter_reading_id)
    .bind("2026-01-01 00:15:00.000000")
    .bind(900_u32)
    .execute(&mut *transaction)
    .await
    .expect("second canonical interval reading should succeed");

    let other_reading_id = other_reading.last_insert_id();

    let cross_reading_revision = sqlx::query(
        r#"
        UPDATE interval_reading
        SET accepted_revision_id = ?
        WHERE id = ?
        "#,
    )
    .bind(first_revision_id)
    .bind(other_reading_id)
    .execute(&mut *transaction)
    .await;

    assert!(
        cross_reading_revision.is_err(),
        "a reading must not accept another reading's revision"
    );

    /*
     * Retrieval coverage.
     */

    let usage_point_coverage = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage (
            acquisition_source_id,
            usage_point_id,
            range_start,
            range_end
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(source_id)
    .bind(usage_point_id)
    .bind("2026-01-01 00:00:00.000000")
    .bind("2026-01-02 00:00:00.000000")
    .execute(&mut *transaction)
    .await
    .expect("usage-point retrieval coverage should succeed");

    let coverage_id = usage_point_coverage.last_insert_id();

    let duplicate_usage_point_coverage = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage (
            acquisition_source_id,
            usage_point_id,
            range_start,
            range_end
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(source_id)
    .bind(usage_point_id)
    .bind("2026-01-01 00:00:00.000000")
    .bind("2026-01-02 00:00:00.000000")
    .execute(&mut *transaction)
    .await;

    assert!(
        duplicate_usage_point_coverage.is_err(),
        "duplicate usage-point coverage should be rejected"
    );

    let source_coverage = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage (
            acquisition_source_id,
            usage_point_id,
            range_start,
            range_end
        )
        VALUES (?, NULL, ?, ?)
        "#,
    )
    .bind(source_id)
    .bind("2026-02-01 00:00:00.000000")
    .bind("2026-02-02 00:00:00.000000")
    .execute(&mut *transaction)
    .await
    .expect("source-wide retrieval coverage should succeed");

    assert!(
        source_coverage.last_insert_id() > 0,
        "source-wide coverage should receive an identity"
    );

    // Verify NULL usage_point_id cannot bypass uniqueness.
    let duplicate_source_coverage = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage (
            acquisition_source_id,
            usage_point_id,
            range_start,
            range_end
        )
        VALUES (?, NULL, ?, ?)
        "#,
    )
    .bind(source_id)
    .bind("2026-02-01 00:00:00.000000")
    .bind("2026-02-02 00:00:00.000000")
    .execute(&mut *transaction)
    .await;

    assert!(
        duplicate_source_coverage.is_err(),
        "duplicate source-wide coverage should be rejected"
    );

    let invalid_coverage_range = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage (
            acquisition_source_id,
            usage_point_id,
            range_start,
            range_end
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(source_id)
    .bind(usage_point_id)
    .bind("2026-03-02 00:00:00.000000")
    .bind("2026-03-01 00:00:00.000000")
    .execute(&mut *transaction)
    .await;

    assert!(
        invalid_coverage_range.is_err(),
        "coverage end must be after its start"
    );

    let coverage_revision = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage_revision (
            retrieval_coverage_id,
            ingestion_run_id,
            status,
            observed_at
        )
        VALUES (?, ?, ?, UTC_TIMESTAMP(6))
        "#,
    )
    .bind(coverage_id)
    .bind(ingestion_run_id)
    .bind("partial")
    .execute(&mut *transaction)
    .await
    .expect("retrieval coverage revision should succeed");

    let coverage_revision_id = coverage_revision.last_insert_id();

    let invalid_status = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage_revision (
            retrieval_coverage_id,
            ingestion_run_id,
            status,
            observed_at
        )
        VALUES (?, ?, ?, UTC_TIMESTAMP(6))
        "#,
    )
    .bind(coverage_id)
    .bind(ingestion_run_id)
    .bind("invalid-status")
    .execute(&mut *transaction)
    .await;

    assert!(
        invalid_status.is_err(),
        "invalid coverage status should be rejected"
    );

    sqlx::query(
        r#"
        INSERT INTO retrieval_gap (
            retrieval_coverage_revision_id,
            gap_start,
            gap_end,
            reason
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(coverage_revision_id)
    .bind("2026-01-01 03:00:00.000000")
    .bind("2026-01-01 04:00:00.000000")
    .bind("integration-test-gap")
    .execute(&mut *transaction)
    .await
    .expect("retrieval gap should succeed");

    let invalid_gap = sqlx::query(
        r#"
        INSERT INTO retrieval_gap (
            retrieval_coverage_revision_id,
            gap_start,
            gap_end,
            reason
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(coverage_revision_id)
    .bind("2026-01-01 05:00:00.000000")
    .bind("2026-01-01 04:00:00.000000")
    .bind("invalid-gap")
    .execute(&mut *transaction)
    .await;

    assert!(invalid_gap.is_err(), "retrieval gap end must be after its start");

    /*
     * OAuth authorization relationship.
     *
     * ADR-0021 / ADR-0022: the initial residential model permits
     * zero or one OAuth authorization per Acquisition Source. One
     * authorization may expose multiple Usage Points and commodities.
     */

    let first_authorization = sqlx::query(
        r#"
        INSERT INTO oauth_authorization (
            acquisition_source_id,
            external_authorization_id,
            resource_owner_external_id,
            status
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(source_id)
    .bind("provider-auth-1")
    .bind("resource-owner-1")
    .bind("active")
    .execute(&mut *transaction)
    .await
    .expect("first OAuth authorization should succeed");

    assert!(
        first_authorization.last_insert_id() > 0,
        "OAuth authorization should receive an identity"
    );

    // ADR-0021 / ADR-0022: a second authorization row for the same
    // Acquisition Source must be rejected.
    let duplicate_authorization = sqlx::query(
        r#"
        INSERT INTO oauth_authorization (
            acquisition_source_id,
            external_authorization_id,
            resource_owner_external_id,
            status
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(source_id)
    .bind("provider-auth-2")
    .bind("resource-owner-2")
    .bind("active")
    .execute(&mut *transaction)
    .await;

    assert!(
        duplicate_authorization.is_err(),
        "second OAuth authorization for the same acquisition source should be rejected"
    );

    // OAuth authorization rows must reference an existing Acquisition Source.
    let invalid_oauth_source = sqlx::query(
        r#"
        INSERT INTO oauth_authorization (
            acquisition_source_id,
            status
        )
        VALUES (?, ?)
        "#,
    )
    .bind(u64::MAX)
    .bind("active")
    .execute(&mut *transaction)
    .await;

    assert!(
        invalid_oauth_source.is_err(),
        "OAuth authorization with invalid source should be rejected"
    );

    transaction.rollback().await.expect("test transaction should roll back");
}

/// Verifies that all expected application tables exist.
///
/// # Inputs
///
/// - `pool`: Active MariaDB integration-test connection pool.
///
/// # Expected Outcomes
///
/// Every table listed in `EXPECTED_TABLES` exists in the selected database.
///
/// # Panics
///
/// Panics if a metadata query fails or an expected table is missing.
async fn verify_expected_tables(pool: &Pool<MySql>) {
    for table_name in EXPECTED_TABLES {
        let exists: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT TABLE_NAME
            FROM information_schema.TABLES
            WHERE TABLE_SCHEMA = DATABASE()
              AND TABLE_NAME = ?
            "#,
        )
        .bind(table_name)
        .fetch_optional(pool)
        .await
        .expect("table lookup should succeed");

        assert!(exists.is_some(), "expected table {table_name} to exist");
    }
}
