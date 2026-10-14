// SPDX-License-Identifier: MIT

//! Integration tests for retrieval coverage and gap persistence.
//!
//! # Architecture
//!
//! These tests provide implementation evidence for ADR-0004, ADR-0010,
//! ADR-0016, ADR-0020, and ADR-0022.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use energy_store::{
    StoreError, create_acquisition_source, create_data_custodian,
    create_ingestion_run, create_retrieval_coverage_revision,
    create_retrieval_gap, create_usage_point, get_or_create_retrieval_coverage,
    get_retrieval_coverage, get_retrieval_coverage_revision,
    list_retrieval_gaps,
};
use sqlx::{MySql, Pool, types::chrono::NaiveDateTime};

/// Verifies idempotent source-wide coverage and gap history.
///
/// # Test Goal
///
/// Confirm ADR-0010 and ADR-0022 behavior for repeated source-wide coverage,
/// observed coverage state, and explicit unavailable subranges.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - Synthetic source, ingestion-run, coverage, and gap records.
///
/// # Expected Outcomes
///
/// - repeated source-wide coverage resolves to one row;
/// - source-wide scope remains `NULL`;
/// - a partial coverage revision is stored and retrieved;
/// - a known gap is stored and listed; and
/// - test records are removed.
#[tokio::test]
async fn retrieval_coverage_repository_round_trip() {
    let pool = test_pool().await;
    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let suffix = unique_suffix();

    let custodian_id =
        create_data_custodian(&pool, &format!("coverage-custodian-{suffix}"))
            .await
            .expect("custodian creation should succeed");

    let source_id = create_acquisition_source(
        &pool,
        custodian_id,
        &format!("coverage-source-{suffix}"),
    )
    .await
    .expect("source creation should succeed");

    let run_id = create_ingestion_run(&pool, source_id)
        .await
        .expect("ingestion run creation should succeed");

    let range_start = timestamp("2026-01-01 00:00:00");
    let range_end = timestamp("2026-01-02 00:00:00");

    let coverage_id = get_or_create_retrieval_coverage(
        &pool,
        source_id,
        None,
        range_start,
        range_end,
    )
    .await
    .expect("coverage creation should succeed");

    let repeated_id = get_or_create_retrieval_coverage(
        &pool,
        source_id,
        None,
        range_start,
        range_end,
    )
    .await
    .expect("repeated coverage creation should succeed");

    assert_eq!(coverage_id, repeated_id);

    let coverage = get_retrieval_coverage(&pool, coverage_id)
        .await
        .expect("coverage lookup should succeed")
        .expect("coverage should exist");

    assert_eq!(coverage.acquisition_source_id, source_id.0);
    assert_eq!(coverage.usage_point_id, None);
    assert_eq!(coverage.range_start, range_start);
    assert_eq!(coverage.range_end, range_end);

    let observed_at = timestamp("2026-01-02 00:05:00");

    let revision_id = create_retrieval_coverage_revision(
        &pool,
        coverage_id,
        run_id,
        "partial",
        observed_at,
    )
    .await
    .expect("coverage revision creation should succeed");

    let revision = get_retrieval_coverage_revision(&pool, revision_id)
        .await
        .expect("coverage revision lookup should succeed")
        .expect("coverage revision should exist");

    assert_eq!(revision.retrieval_coverage_id, coverage_id.0);
    assert_eq!(revision.ingestion_run_id, run_id.0);
    assert_eq!(revision.status, "partial");
    assert_eq!(revision.observed_at, observed_at);

    let gap_start = timestamp("2026-01-01 04:00:00");
    let gap_end = timestamp("2026-01-01 05:00:00");

    let gap_id = create_retrieval_gap(
        &pool,
        revision_id,
        gap_start,
        gap_end,
        Some("synthetic unavailable range"),
    )
    .await
    .expect("gap creation should succeed");

    let gaps = list_retrieval_gaps(&pool, revision_id)
        .await
        .expect("gap lookup should succeed");

    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].id, gap_id.0);
    assert_eq!(gaps[0].gap_start, gap_start);
    assert_eq!(gaps[0].gap_end, gap_end);

    cleanup(&pool, source_id.0, custodian_id.0)
        .await
        .expect("integration-test cleanup should succeed");
}

/// Verifies scoped coverage and database-enforced constraints.
///
/// # Test Goal
///
/// Confirm source-wide and Usage Point-scoped ranges remain distinct, and that
/// schema constraints reject invalid coverage status and invalid gap ranges.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - Synthetic source, Usage Point, ingestion-run, and coverage records.
///
/// # Expected Outcomes
///
/// - source-wide and Usage Point-scoped ranges can coexist;
/// - invalid status is rejected as StoreError::Conflict;
/// - an invalid gap range is rejected as StoreError::Conflict; and
/// - test records are removed.
#[tokio::test]
async fn retrieval_coverage_repository_enforces_scope_and_constraints() {
    let pool = test_pool().await;
    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let suffix = unique_suffix();

    let custodian_id = create_data_custodian(
        &pool,
        &format!("coverage-constraint-custodian-{suffix}"),
    )
    .await
    .expect("custodian creation should succeed");

    let source_id = create_acquisition_source(
        &pool,
        custodian_id,
        &format!("coverage-constraint-source-{suffix}"),
    )
    .await
    .expect("source creation should succeed");

    let usage_point_id = create_usage_point(
        &pool,
        source_id,
        &format!("coverage-usage-point-{suffix}"),
        Some(0),
    )
    .await
    .expect("usage point creation should succeed");

    let run_id = create_ingestion_run(&pool, source_id)
        .await
        .expect("ingestion run creation should succeed");

    let range_start = timestamp("2026-02-01 00:00:00");
    let range_end = timestamp("2026-02-02 00:00:00");

    let source_coverage = get_or_create_retrieval_coverage(
        &pool,
        source_id,
        None,
        range_start,
        range_end,
    )
    .await
    .expect("source-wide coverage creation should succeed");

    let usage_coverage = get_or_create_retrieval_coverage(
        &pool,
        source_id,
        Some(usage_point_id),
        range_start,
        range_end,
    )
    .await
    .expect("Usage Point coverage creation should succeed");

    assert_ne!(source_coverage, usage_coverage);

    let observed_at = timestamp("2026-02-02 00:05:00");

    let invalid_status = create_retrieval_coverage_revision(
        &pool,
        usage_coverage,
        run_id,
        "unknown",
        observed_at,
    )
    .await
    .expect_err("invalid coverage status should fail");

    assert!(
        matches!(invalid_status, StoreError::Conflict),
        "invalid status should return StoreError::Conflict"
    );

    let revision_id = create_retrieval_coverage_revision(
        &pool,
        usage_coverage,
        run_id,
        "partial",
        observed_at,
    )
    .await
    .expect("valid coverage revision creation should succeed");

    let invalid_gap = create_retrieval_gap(
        &pool,
        revision_id,
        range_end,
        range_start,
        Some("invalid"),
    )
    .await
    .expect_err("reversed gap range should fail");

    assert!(
        matches!(invalid_gap, StoreError::Conflict),
        "invalid gap range should return StoreError::Conflict"
    );

    cleanup(&pool, source_id.0, custodian_id.0)
        .await
        .expect("integration-test cleanup should succeed");
}

/// Opens the MariaDB integration-test connection.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
///
/// # Returns
///
/// Returns a connected MariaDB pool.
///
/// # Expected Outcomes
///
/// Fails immediately when the test URL is missing or the database is
/// unavailable.
async fn test_pool() -> Pool<MySql> {
    let database_url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL must be set for database integration tests");

    Pool::<MySql>::connect(&database_url)
        .await
        .expect("test database connection should succeed")
}

/// Parses a fixed UTC test timestamp.
///
/// # Inputs
///
/// - `value`: Timestamp in `%Y-%m-%d %H:%M:%S` format.
///
/// # Returns
///
/// Returns the parsed [`NaiveDateTime`].
///
/// # Expected Outcomes
///
/// Static timestamps used by these tests parse successfully.
fn timestamp(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .expect("static test timestamp should parse")
}

/// Removes synthetic records created by a retrieval-coverage test.
///
/// # Inputs
///
/// - `pool`: Active MariaDB test connection pool.
/// - `source_id`: Acquisition Source created by the test.
/// - `custodian_id`: Data Custodian created by the test.
///
/// # Expected Outcomes
///
/// Child records are removed in foreign-key-safe order.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if cleanup fails.
async fn cleanup(
    pool: &Pool<MySql>,
    source_id: u64,
    custodian_id: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE rg
        FROM retrieval_gap rg
        JOIN retrieval_coverage_revision rcr
          ON rcr.id = rg.retrieval_coverage_revision_id
        JOIN retrieval_coverage rc
          ON rc.id = rcr.retrieval_coverage_id
        WHERE rc.acquisition_source_id = ?
        "#,
    )
    .bind(source_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        DELETE rcr
        FROM retrieval_coverage_revision rcr
        JOIN retrieval_coverage rc
          ON rc.id = rcr.retrieval_coverage_id
        WHERE rc.acquisition_source_id = ?
        "#,
    )
    .bind(source_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "DELETE FROM retrieval_coverage WHERE acquisition_source_id = ?",
    )
    .bind(source_id)
    .execute(pool)
    .await?;

    sqlx::query("DELETE FROM ingestion_run WHERE acquisition_source_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM usage_point WHERE acquisition_source_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM acquisition_source WHERE id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM data_custodian WHERE id = ?")
        .bind(custodian_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Creates a process-local suffix for synthetic integration-test names.
///
/// # Returns
///
/// Nanoseconds elapsed since the Unix epoch.
///
/// # Expected Outcomes
///
/// Repeated test runs are unlikely to collide on unique source identifiers.
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos()
}
