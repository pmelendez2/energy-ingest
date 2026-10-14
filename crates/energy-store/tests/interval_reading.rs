// SPDX-License-Identifier: MIT

//! Integration tests for canonical interval-reading revision storage.
//!
//! # Architecture
//!
//! These tests provide implementation evidence for ADR-0005, ADR-0016,
//! ADR-0020, and ADR-0022.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use energy_store::{
    StoreError, create_acquisition_source, create_data_custodian,
    create_ingestion_run, create_interval_block,
    create_interval_reading_revision, create_meter_reading,
    create_reading_type, create_usage_point, get_interval_reading,
    get_interval_reading_by_identity, get_interval_reading_revision,
    get_or_create_interval_reading, list_interval_reading_revisions,
    set_accepted_interval_reading_revision,
};
use sqlx::{MySql, Pool, types::chrono::NaiveDateTime};

/// Verifies canonical interval identity and revision-history behavior.
///
/// # Test Goal
///
/// Confirm the ADR-0005 and ADR-0022 behavior that repeated imports resolve to
/// one canonical interval identity while separate observed values are retained
/// as revisions.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - Synthetic source, Green Button, interval, and revision records.
///
/// # Expected Outcomes
///
/// - repeated canonical creation returns the same interval-reading identifier;
/// - the canonical interval can be found by its schema-defined identity;
/// - multiple revisions can be stored across ingestion runs;
/// - revision history is returned without overwriting earlier observations;
/// - one revision can be selected as accepted; and
/// - test-created records are removed in foreign-key-safe order.
#[tokio::test]
async fn interval_reading_repository_preserves_revision_history() {
    let pool = test_pool().await;
    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let fixture = create_fixture(&pool, "revision-history")
        .await
        .expect("test fixture creation should succeed");

    let interval_start = NaiveDateTime::parse_from_str(
        "2026-01-01 00:00:00",
        "%Y-%m-%d %H:%M:%S",
    )
    .expect("static test timestamp should parse");

    let interval_id = get_or_create_interval_reading(
        &pool,
        fixture.meter_reading_id,
        interval_start,
        3600,
    )
    .await
    .expect("canonical interval creation should succeed");

    let repeated_interval_id = get_or_create_interval_reading(
        &pool,
        fixture.meter_reading_id,
        interval_start,
        3600,
    )
    .await
    .expect("repeated canonical interval creation should succeed");

    assert_eq!(
        interval_id, repeated_interval_id,
        "repeated imports should resolve to one canonical interval identity"
    );

    let found = get_interval_reading_by_identity(
        &pool,
        fixture.meter_reading_id,
        interval_start,
        3600,
    )
    .await
    .expect("canonical interval lookup should succeed")
    .expect("canonical interval should exist");

    assert_eq!(found.id, interval_id.0);
    assert_eq!(found.meter_reading_id, fixture.meter_reading_id.0);
    assert_eq!(found.interval_start, interval_start);
    assert_eq!(found.interval_duration_seconds, 3600);
    assert_eq!(found.accepted_revision_id, None);

    let first_revision_id = create_interval_reading_revision(
        &pool,
        interval_id,
        fixture.first_ingestion_run_id,
        Some(fixture.interval_block_id),
        Some("revision-1"),
        None,
        None,
        1000,
        Some("reported"),
    )
    .await
    .expect("first revision creation should succeed");

    let second_revision_id = create_interval_reading_revision(
        &pool,
        interval_id,
        fixture.second_ingestion_run_id,
        Some(fixture.interval_block_id),
        Some("revision-2"),
        None,
        None,
        1100,
        Some("corrected"),
    )
    .await
    .expect("second revision creation should succeed");

    let first_revision =
        get_interval_reading_revision(&pool, first_revision_id)
            .await
            .expect("revision lookup should succeed")
            .expect("first revision should exist");

    assert_eq!(first_revision.interval_reading_id, interval_id.0);
    assert_eq!(
        first_revision.ingestion_run_id,
        fixture.first_ingestion_run_id.0
    );
    assert_eq!(first_revision.reading_value, 1000);

    let revisions = list_interval_reading_revisions(&pool, interval_id)
        .await
        .expect("revision history lookup should succeed");

    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].id, first_revision_id.0);
    assert_eq!(revisions[0].reading_value, 1000);
    assert_eq!(revisions[1].id, second_revision_id.0);
    assert_eq!(revisions[1].reading_value, 1100);

    set_accepted_interval_reading_revision(
        &pool,
        interval_id,
        second_revision_id,
    )
    .await
    .expect("accepted revision update should succeed");

    let accepted = get_interval_reading(&pool, interval_id)
        .await
        .expect("canonical interval lookup should succeed")
        .expect("canonical interval should exist");

    assert_eq!(accepted.accepted_revision_id, Some(second_revision_id.0));

    cleanup_fixture(&pool, &[interval_id.0], &fixture)
        .await
        .expect("integration-test cleanup should succeed");
}

/// Verifies that an interval cannot accept a revision owned by another interval.
///
/// # Test Goal
///
/// Prove the ADR-0022 database invariant that `accepted_revision_id` must refer
/// to a revision belonging to the same canonical interval reading.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - Two synthetic canonical intervals with separate revisions.
///
/// # Expected Outcomes
///
/// - each interval and revision is created successfully;
/// - assigning the second interval's revision to the first interval fails;
/// - the failure crosses the persistence boundary as [`StoreError::Conflict`];
/// - the previously accepted revision remains unchanged; and
/// - all test-created records are removed.
#[tokio::test]
async fn interval_reading_repository_rejects_foreign_accepted_revision() {
    let pool = test_pool().await;
    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let fixture = create_fixture(&pool, "foreign-revision")
        .await
        .expect("test fixture creation should succeed");

    let first_start = NaiveDateTime::parse_from_str(
        "2026-01-01 00:00:00",
        "%Y-%m-%d %H:%M:%S",
    )
    .expect("static test timestamp should parse");
    let second_start = NaiveDateTime::parse_from_str(
        "2026-01-01 01:00:00",
        "%Y-%m-%d %H:%M:%S",
    )
    .expect("static test timestamp should parse");

    let first_interval_id = get_or_create_interval_reading(
        &pool,
        fixture.meter_reading_id,
        first_start,
        3600,
    )
    .await
    .expect("first canonical interval creation should succeed");

    let second_interval_id = get_or_create_interval_reading(
        &pool,
        fixture.meter_reading_id,
        second_start,
        3600,
    )
    .await
    .expect("second canonical interval creation should succeed");

    let first_revision_id = create_interval_reading_revision(
        &pool,
        first_interval_id,
        fixture.first_ingestion_run_id,
        Some(fixture.interval_block_id),
        Some("first-interval-revision"),
        None,
        None,
        1000,
        None,
    )
    .await
    .expect("first revision creation should succeed");

    let second_revision_id = create_interval_reading_revision(
        &pool,
        second_interval_id,
        fixture.first_ingestion_run_id,
        Some(fixture.interval_block_id),
        Some("second-interval-revision"),
        None,
        None,
        2000,
        None,
    )
    .await
    .expect("second revision creation should succeed");

    set_accepted_interval_reading_revision(
        &pool,
        first_interval_id,
        first_revision_id,
    )
    .await
    .expect("valid accepted revision update should succeed");

    let invalid = set_accepted_interval_reading_revision(
        &pool,
        first_interval_id,
        second_revision_id,
    )
    .await
    .expect_err("foreign revision assignment should fail");

    assert!(
        matches!(invalid, StoreError::Conflict),
        "foreign accepted revision should return StoreError::Conflict"
    );

    let first_interval = get_interval_reading(&pool, first_interval_id)
        .await
        .expect("canonical interval lookup should succeed")
        .expect("first canonical interval should exist");

    assert_eq!(
        first_interval.accepted_revision_id,
        Some(first_revision_id.0),
        "failed update must not replace the previously accepted revision"
    );

    cleanup_fixture(
        &pool,
        &[first_interval_id.0, second_interval_id.0],
        &fixture,
    )
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
/// The function fails immediately when the test URL is missing or the database
/// is unavailable.
async fn test_pool() -> Pool<MySql> {
    let database_url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL must be set for database integration tests");

    Pool::<MySql>::connect(&database_url)
        .await
        .expect("test database connection should succeed")
}

/// Identifiers for records shared by an interval-reading integration test.
///
/// The fixture contains only synthetic, disposable test data as required by
/// ADR-0020.
struct TestFixture {
    custodian_id: u64,
    source_id: u64,
    usage_point_id: u64,
    reading_type_id: u64,
    meter_reading_id: energy_store::MeterReadingId,
    interval_block_id: energy_store::IntervalBlockId,
    first_ingestion_run_id: energy_store::IngestionRunId,
    second_ingestion_run_id: energy_store::IngestionRunId,
}

/// Creates the synthetic parent records required by interval-reading tests.
///
/// # Inputs
///
/// - `pool`: Active MariaDB test connection pool.
/// - `label`: Short test-specific label included in generated names.
///
/// # Returns
///
/// Returns identifiers for the complete parent-record fixture.
///
/// # Errors
///
/// Returns [`StoreError`] if any `energy-store` operation fails.
///
/// # Notes
///
/// ADR-0020 requires routine integration tests to use synthetic or sanitized
/// data rather than live provider data.
async fn create_fixture(
    pool: &Pool<MySql>,
    label: &str,
) -> Result<TestFixture, StoreError> {
    let suffix = unique_suffix();

    let custodian_id =
        create_data_custodian(pool, &format!("{label}-custodian-{suffix}"))
            .await?;

    let source_id = create_acquisition_source(
        pool,
        custodian_id,
        &format!("{label}-source-{suffix}"),
    )
    .await?;

    let usage_point_id = create_usage_point(
        pool,
        source_id,
        &format!("{label}-usage-point-{suffix}"),
        Some(0),
    )
    .await?;

    let reading_type_id = create_reading_type(
        pool,
        source_id,
        &format!("{label}-reading-type-{suffix}"),
    )
    .await?;

    let meter_reading_id = create_meter_reading(
        pool,
        usage_point_id,
        reading_type_id,
        &format!("{label}-meter-reading-{suffix}"),
    )
    .await?;

    let block_start = NaiveDateTime::parse_from_str(
        "2026-01-01 00:00:00",
        "%Y-%m-%d %H:%M:%S",
    )
    .expect("static test timestamp should parse");

    let interval_block_id = create_interval_block(
        pool,
        meter_reading_id,
        &format!("{label}-interval-block-{suffix}"),
        block_start,
        7200,
    )
    .await?;

    let first_ingestion_run_id = create_ingestion_run(pool, source_id).await?;
    let second_ingestion_run_id = create_ingestion_run(pool, source_id).await?;

    Ok(TestFixture {
        custodian_id: custodian_id.0,
        source_id: source_id.0,
        usage_point_id: usage_point_id.0,
        reading_type_id: reading_type_id.0,
        meter_reading_id,
        interval_block_id,
        first_ingestion_run_id,
        second_ingestion_run_id,
    })
}

/// Removes all records created by an interval-reading integration test.
///
/// # Inputs
///
/// - `pool`: Active MariaDB test connection pool.
/// - `interval_reading_ids`: Canonical interval identifiers created by the test.
/// - `fixture`: Parent-record identifiers created by [`create_fixture`].
///
/// # Expected Outcomes
///
/// Records are deleted in foreign-key-safe order, including clearing accepted
/// revision references before revision rows are removed.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if any cleanup query fails.
async fn cleanup_fixture(
    pool: &Pool<MySql>,
    interval_reading_ids: &[u64],
    fixture: &TestFixture,
) -> Result<(), sqlx::Error> {
    for interval_reading_id in interval_reading_ids {
        sqlx::query("UPDATE interval_reading SET accepted_revision_id = NULL WHERE id = ?")
            .bind(interval_reading_id)
            .execute(pool)
            .await?;
    }

    for interval_reading_id in interval_reading_ids {
        sqlx::query("DELETE FROM interval_reading_revision WHERE interval_reading_id = ?")
            .bind(interval_reading_id)
            .execute(pool)
            .await?;
    }

    for interval_reading_id in interval_reading_ids {
        sqlx::query("DELETE FROM interval_reading WHERE id = ?")
            .bind(interval_reading_id)
            .execute(pool)
            .await?;
    }

    sqlx::query("DELETE FROM ingestion_run WHERE id IN (?, ?)")
        .bind(fixture.first_ingestion_run_id.0)
        .bind(fixture.second_ingestion_run_id.0)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM interval_block WHERE id = ?")
        .bind(fixture.interval_block_id.0)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM meter_reading WHERE id = ?")
        .bind(fixture.meter_reading_id.0)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM reading_type WHERE id = ?")
        .bind(fixture.reading_type_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM usage_point WHERE id = ?")
        .bind(fixture.usage_point_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM acquisition_source WHERE id = ?")
        .bind(fixture.source_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM data_custodian WHERE id = ?")
        .bind(fixture.custodian_id)
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
