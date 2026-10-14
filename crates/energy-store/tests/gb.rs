// SPDX-License-Identifier: MIT

//! Green Button persistence integration tests.
//!
//! # Purpose
//!
//! Verifies the public `energy-store` APIs for the Green Button parent-resource
//! chain, numeric metadata, lookup behavior, and typed constraint errors.
//!
//! # Architecture
//!
//! These tests exercise the MariaDB boundary defined by ADR-0016 and ADR-0022
//! using synthetic disposable records as required by ADR-0020.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use energy_store::{
    ReadingTypeValues, StoreError, create_acquisition_source,
    create_data_custodian, create_interval_block, create_meter_reading,
    create_reading_type_with_values, create_usage_point, get_interval_block,
    get_interval_block_by_external_id, get_meter_reading,
    get_meter_reading_by_external_id, get_reading_type,
    get_reading_type_by_external_id, get_usage_point,
    get_usage_point_by_external_id,
};
use sqlx::{MySql, Pool, types::chrono::NaiveDateTime};

/// Verifies creation and retrieval of the initial Green Button entity chain.
///
/// # Test Goal
///
/// Confirm that the public `energy-store` API can persist and retrieve a Usage
/// Point, Reading Type, Meter Reading, and Interval Block using database types
/// that match the Green Button schema.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - Synthetic numeric Green Button code values used only to verify storage.
///
/// # Expected Outcomes
///
/// - each entity is created successfully;
/// - stored parent/child relationships match returned identifiers;
/// - numeric Green Button fields round-trip as integers;
/// - the Usage Point can be found by source external ID; and
/// - test-created records are removed.
#[tokio::test]
async fn green_button_repository_round_trip() {
    let database_url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL must be set for database integration tests");

    let pool = Pool::<MySql>::connect(&database_url)
        .await
        .expect("test database connection should succeed");

    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let suffix = unique_suffix();

    let custodian_name = format!("green-button-test-custodian-{suffix}");

    let custodian_id = create_data_custodian(&pool, &custodian_name)
        .await
        .expect("custodian creation should succeed");

    let source_id = create_acquisition_source(
        &pool,
        custodian_id,
        &format!("green-button-test-source-{suffix}"),
    )
    .await
    .expect("source creation should succeed");

    let usage_external_id = format!("usage-point-{suffix}");

    // Synthetic numeric value used to verify database/API type consistency.
    // This test does not assert that the value maps to a particular
    // Green Button service-category meaning.
    let service_category_kind = 1_u32;

    let usage_point_id = create_usage_point(
        &pool,
        source_id,
        &usage_external_id,
        Some(service_category_kind),
    )
    .await
    .expect("usage point creation should succeed");

    let usage_point = get_usage_point(&pool, usage_point_id)
        .await
        .expect("usage point lookup should succeed")
        .expect("usage point should exist");

    assert_eq!(usage_point.id, usage_point_id.0);
    assert_eq!(usage_point.acquisition_source_id, source_id.0);
    assert_eq!(usage_point.external_id, usage_external_id);
    assert_eq!(usage_point.service_category_kind, Some(service_category_kind));

    let usage_point_by_external_id =
        get_usage_point_by_external_id(&pool, source_id, &usage_external_id)
            .await
            .expect("usage point external-ID lookup should succeed")
            .expect("usage point should exist");

    assert_eq!(usage_point_by_external_id.id, usage_point_id.0);

    let reading_type_external_id = format!("reading-type-{suffix}");
    let reading_type_values = ReadingTypeValues {
        accumulation_behaviour: Some(4),
        commodity: Some(1),
        data_qualifier: Some(12),
        flow_direction: Some(1),
        kind: Some(12),
        phase: Some(0),
        power_of_ten_multiplier: Some(0),
        unit_of_measure: Some(72),
    };

    let reading_type_id = create_reading_type_with_values(
        &pool,
        source_id,
        &reading_type_external_id,
        reading_type_values,
    )
    .await
    .expect("reading type creation should succeed");

    let reading_type = get_reading_type(&pool, reading_type_id)
        .await
        .expect("reading type lookup should succeed")
        .expect("reading type should exist");

    assert_eq!(reading_type.id, reading_type_id.0);
    assert_eq!(reading_type.acquisition_source_id, source_id.0);
    assert_eq!(reading_type.external_id, reading_type_external_id);
    assert_eq!(reading_type.accumulation_behaviour, Some(4));
    assert_eq!(reading_type.commodity, Some(1));
    assert_eq!(reading_type.data_qualifier, Some(12));
    assert_eq!(reading_type.flow_direction, Some(1));
    assert_eq!(reading_type.kind, Some(12));
    assert_eq!(reading_type.phase, Some(0));
    assert_eq!(reading_type.power_of_ten_multiplier, Some(0));
    assert_eq!(reading_type.unit_of_measure, Some(72));

    let reading_type_by_external_id = get_reading_type_by_external_id(
        &pool,
        source_id,
        &reading_type_external_id,
    )
    .await
    .expect("reading type external-ID lookup should succeed")
    .expect("reading type should exist");

    assert_eq!(reading_type_by_external_id.id, reading_type_id.0);

    let meter_reading_external_id = format!("meter-reading-{suffix}");

    let meter_reading_id = create_meter_reading(
        &pool,
        usage_point_id,
        reading_type_id,
        &meter_reading_external_id,
    )
    .await
    .expect("meter reading creation should succeed");

    let meter_reading = get_meter_reading(&pool, meter_reading_id)
        .await
        .expect("meter reading lookup should succeed")
        .expect("meter reading should exist");

    assert_eq!(meter_reading.id, meter_reading_id.0);
    assert_eq!(meter_reading.usage_point_id, usage_point_id.0);
    assert_eq!(meter_reading.reading_type_id, reading_type_id.0);

    let meter_reading_by_external_id = get_meter_reading_by_external_id(
        &pool,
        usage_point_id,
        &meter_reading_external_id,
    )
    .await
    .expect("meter reading external-ID lookup should succeed")
    .expect("meter reading should exist");

    assert_eq!(meter_reading_by_external_id.id, meter_reading_id.0);

    let interval_start = NaiveDateTime::parse_from_str(
        "2026-01-01 00:00:00",
        "%Y-%m-%d %H:%M:%S",
    )
    .expect("static test timestamp should parse");

    let interval_duration_seconds = 3600_u64;

    let interval_block_external_id = format!("interval-block-{suffix}");

    let interval_block_id = create_interval_block(
        &pool,
        meter_reading_id,
        &interval_block_external_id,
        interval_start,
        interval_duration_seconds,
    )
    .await
    .expect("interval block creation should succeed");

    let interval_block = get_interval_block(&pool, interval_block_id)
        .await
        .expect("interval block lookup should succeed")
        .expect("interval block should exist");

    assert_eq!(interval_block.id, interval_block_id.0);
    assert_eq!(interval_block.meter_reading_id, meter_reading_id.0);
    assert_eq!(interval_block.interval_start, interval_start);
    assert_eq!(
        interval_block.interval_duration_seconds,
        interval_duration_seconds
    );

    let interval_block_by_external_id = get_interval_block_by_external_id(
        &pool,
        meter_reading_id,
        &interval_block_external_id,
    )
    .await
    .expect("interval block external-ID lookup should succeed")
    .expect("interval block should exist");

    assert_eq!(interval_block_by_external_id.id, interval_block_id.0);

    cleanup(
        &pool,
        interval_block_id.0,
        meter_reading_id.0,
        reading_type_id.0,
        usage_point_id.0,
        source_id.0,
        custodian_id.0,
    )
    .await
    .expect("integration-test cleanup should succeed");
}

/// Verifies typed errors for Green Button uniqueness conflicts.
///
/// # Test Goal
///
/// Confirm that duplicate source-scoped Green Button identifiers are
/// translated into [`StoreError::Conflict`].
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
///
/// # Expected Outcomes
///
/// - the first Usage Point insert succeeds;
/// - a duplicate source/external-ID insert fails with
///   [`StoreError::Conflict`]; and
/// - test-created records are removed.
#[tokio::test]
async fn green_button_repository_returns_typed_constraint_errors() {
    let database_url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL must be set for database integration tests");

    let pool = Pool::<MySql>::connect(&database_url)
        .await
        .expect("test database connection should succeed");

    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let suffix = unique_suffix();

    let custodian_id = create_data_custodian(
        &pool,
        &format!("green-button-constraint-custodian-{suffix}"),
    )
    .await
    .expect("custodian creation should succeed");

    let source_id = create_acquisition_source(
        &pool,
        custodian_id,
        &format!("green-button-constraint-source-{suffix}"),
    )
    .await
    .expect("source creation should succeed");

    let external_id = format!("duplicate-usage-point-{suffix}");

    let usage_point_id =
        create_usage_point(&pool, source_id, &external_id, None)
            .await
            .expect("initial usage point creation should succeed");

    let duplicate = create_usage_point(&pool, source_id, &external_id, None)
        .await
        .expect_err("duplicate usage point should fail");

    assert!(
        matches!(duplicate, StoreError::Conflict),
        "duplicate usage point should return StoreError::Conflict"
    );

    cleanup(&pool, 0, 0, 0, usage_point_id.0, source_id.0, custodian_id.0)
        .await
        .expect("integration-test cleanup should succeed");
}

/// Removes Green Button records created by an integration test.
///
/// # Inputs
///
/// IDs may be zero for entity types that were not created by a test.
///
/// # Expected Outcomes
///
/// Existing test records are removed in foreign-key-safe order.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if a cleanup query fails.
#[allow(clippy::too_many_arguments)]
async fn cleanup(
    pool: &Pool<MySql>,
    interval_block_id: u64,
    meter_reading_id: u64,
    reading_type_id: u64,
    usage_point_id: u64,
    source_id: u64,
    custodian_id: u64,
) -> Result<(), sqlx::Error> {
    if interval_block_id != 0 {
        sqlx::query("DELETE FROM interval_block WHERE id = ?")
            .bind(interval_block_id)
            .execute(pool)
            .await?;
    }

    if meter_reading_id != 0 {
        sqlx::query("DELETE FROM meter_reading WHERE id = ?")
            .bind(meter_reading_id)
            .execute(pool)
            .await?;
    }

    if reading_type_id != 0 {
        sqlx::query("DELETE FROM reading_type WHERE id = ?")
            .bind(reading_type_id)
            .execute(pool)
            .await?;
    }

    if usage_point_id != 0 {
        sqlx::query("DELETE FROM usage_point WHERE id = ?")
            .bind(usage_point_id)
            .execute(pool)
            .await?;
    }

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

/// Creates a process-local suffix for integration-test record names.
///
/// # Returns
///
/// Nanoseconds elapsed since the Unix epoch.
///
/// # Expected Outcomes
///
/// Repeated test runs are unlikely to generate duplicate source identifiers.
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos()
}
