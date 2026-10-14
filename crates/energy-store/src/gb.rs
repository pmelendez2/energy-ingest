// SPDX-License-Identifier: MIT

//! Green Button persistence primitives.
//!
//! # Purpose
//!
//! Provides typed MariaDB access for the Green Button parent-resource chain
//! used by canonical energy ingestion.
//!
//! # Architecture
//!
//! This module implements the persistence boundary defined by ADR-0016 and
//! ADR-0022. XML parsing remains in the `gb` crate; this module stores
//! provider-independent identifiers and numeric Green Button metadata.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use crate::{AcquisitionSourceId, StoreError};
use sqlx::{FromRow, MySql, Pool, types::chrono::NaiveDateTime};

/// Application-controlled identifier for a Usage Point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UsagePointId(pub u64);

/// Application-controlled identifier for a Reading Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReadingTypeId(pub u64);

/// Numeric Green Button fields stored for a Reading Type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReadingTypeValues {
    /// Green Button numeric accumulation-behaviour code.
    pub accumulation_behaviour: Option<u32>,

    /// Green Button numeric commodity code.
    pub commodity: Option<u32>,

    /// Green Button numeric data-qualifier code.
    pub data_qualifier: Option<u32>,

    /// Green Button numeric flow-direction code.
    pub flow_direction: Option<u32>,

    /// Green Button numeric reading-kind code.
    pub kind: Option<u32>,

    /// Green Button numeric phase code.
    pub phase: Option<u32>,

    /// Base-10 exponent applied to reading values.
    pub power_of_ten_multiplier: Option<i16>,

    /// Green Button numeric unit-of-measure code.
    pub unit_of_measure: Option<u32>,
}

/// Application-controlled identifier for a Meter Reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeterReadingId(pub u64);

/// Application-controlled identifier for an Interval Block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalBlockId(pub u64);

/// Stored Green Button Usage Point.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct UsagePoint {
    /// Application-controlled identifier.
    pub id: u64,

    /// Acquisition Source that supplied this Usage Point.
    pub acquisition_source_id: u64,

    /// Identifier supplied by the acquisition source.
    pub external_id: String,

    /// Green Button numeric service-category code.
    pub service_category_kind: Option<u32>,

    /// UTC timestamp when this record was created.
    pub created_at: NaiveDateTime,
}

/// Stored Green Button Reading Type.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ReadingType {
    /// Application-controlled identifier.
    pub id: u64,

    /// Acquisition Source that supplied this Reading Type.
    pub acquisition_source_id: u64,

    /// Identifier supplied by the acquisition source.
    pub external_id: String,

    /// Green Button numeric accumulation-behaviour code.
    pub accumulation_behaviour: Option<u32>,

    /// Green Button numeric commodity code.
    pub commodity: Option<u32>,

    /// Green Button numeric data-qualifier code.
    pub data_qualifier: Option<u32>,

    /// Green Button numeric flow-direction code.
    pub flow_direction: Option<u32>,

    /// Green Button numeric reading-kind code.
    pub kind: Option<u32>,

    /// Green Button numeric phase code.
    pub phase: Option<u32>,

    /// Base-10 exponent applied to reading values.
    pub power_of_ten_multiplier: Option<i16>,

    /// Green Button numeric unit-of-measure code.
    pub unit_of_measure: Option<u32>,

    /// UTC timestamp when this record was created.
    pub created_at: NaiveDateTime,
}

/// Stored Green Button Meter Reading.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct MeterReading {
    /// Application-controlled identifier.
    pub id: u64,

    /// Usage Point to which this Meter Reading belongs.
    pub usage_point_id: u64,

    /// Reading Type that describes this Meter Reading.
    pub reading_type_id: u64,

    /// Identifier supplied by the acquisition source.
    pub external_id: String,

    /// UTC timestamp when this record was created.
    pub created_at: NaiveDateTime,
}

/// Stored Green Button Interval Block.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct IntervalBlock {
    /// Application-controlled identifier.
    pub id: u64,

    /// Meter Reading that owns this Interval Block.
    pub meter_reading_id: u64,

    /// Identifier supplied by the acquisition source.
    pub external_id: String,

    /// UTC start time of the block.
    pub interval_start: NaiveDateTime,

    /// Length of the block in seconds.
    pub interval_duration_seconds: u64,

    /// UTC timestamp when this record was created.
    pub created_at: NaiveDateTime,
}

/// Creates a Usage Point.
///
/// # Purpose
///
/// Stores a source-scoped Green Button Usage Point.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `acquisition_source_id`: Source that supplied the Usage Point.
/// - `external_id`: Identifier supplied by that source.
/// - `service_category_kind`: Optional Green Button numeric service-category
///   code.
///
/// # Returns
///
/// Returns the new [`UsagePointId`].
///
/// # Errors
///
/// - [`StoreError::Conflict`] when the source/external-ID combination already
///   exists.
/// - [`StoreError::Database`] for invalid references or other database
///   failures.
pub async fn create_usage_point(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
    external_id: &str,
    service_category_kind: Option<u32>,
) -> Result<UsagePointId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO usage_point (
            acquisition_source_id,
            external_id,
            service_category_kind
        )
        VALUES (?, ?, ?)
        "#,
    )
    .bind(acquisition_source_id.0)
    .bind(external_id)
    .bind(service_category_kind)
    .execute(pool)
    .await?;

    Ok(UsagePointId(result.last_insert_id()))
}

/// Returns a Usage Point by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Usage Point identifier.
///
/// # Returns
///
/// Returns the matching [`UsagePoint`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_usage_point(
    pool: &Pool<MySql>,
    id: UsagePointId,
) -> Result<Option<UsagePoint>, StoreError> {
    Ok(sqlx::query_as::<_, UsagePoint>(
        r#"
        SELECT
            id,
            acquisition_source_id,
            external_id,
            service_category_kind,
            created_at
        FROM usage_point
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Finds a Usage Point by acquisition source and source identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `acquisition_source_id`: Source to search.
/// - `external_id`: Source-provided Usage Point identifier.
///
/// # Returns
///
/// Returns the matching [`UsagePoint`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_usage_point_by_external_id(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
    external_id: &str,
) -> Result<Option<UsagePoint>, StoreError> {
    Ok(sqlx::query_as::<_, UsagePoint>(
        r#"
        SELECT
            id,
            acquisition_source_id,
            external_id,
            service_category_kind,
            created_at
        FROM usage_point
        WHERE acquisition_source_id = ?
          AND external_id = ?
        "#,
    )
    .bind(acquisition_source_id.0)
    .bind(external_id)
    .fetch_optional(pool)
    .await?)
}

/// Creates a Reading Type using only its source-scoped identity.
///
/// This compatibility helper stores all optional numeric fields as `NULL`.
pub async fn create_reading_type(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
    external_id: &str,
) -> Result<ReadingTypeId, StoreError> {
    create_reading_type_with_values(
        pool,
        acquisition_source_id,
        external_id,
        ReadingTypeValues::default(),
    )
    .await
}

/// Creates a Reading Type with Green Button numeric metadata.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `acquisition_source_id`: Source that supplied the Reading Type.
/// - `external_id`: Identifier supplied by that source.
/// - `values`: Numeric Green Button fields retained by the canonical schema.
///
/// # Returns
///
/// Returns the new [`ReadingTypeId`].
///
/// # Errors
///
/// - [`StoreError::Conflict`] when the source/external-ID combination already
///   exists.
/// - [`StoreError::Database`] for invalid references or other database
///   failures.
pub async fn create_reading_type_with_values(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
    external_id: &str,
    values: ReadingTypeValues,
) -> Result<ReadingTypeId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO reading_type (
            acquisition_source_id,
            external_id,
            accumulation_behaviour,
            commodity,
            data_qualifier,
            flow_direction,
            kind,
            phase,
            power_of_ten_multiplier,
            unit_of_measure
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(acquisition_source_id.0)
    .bind(external_id)
    .bind(values.accumulation_behaviour)
    .bind(values.commodity)
    .bind(values.data_qualifier)
    .bind(values.flow_direction)
    .bind(values.kind)
    .bind(values.phase)
    .bind(values.power_of_ten_multiplier)
    .bind(values.unit_of_measure)
    .execute(pool)
    .await?;

    Ok(ReadingTypeId(result.last_insert_id()))
}

/// Returns a Reading Type by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Reading Type identifier.
///
/// # Returns
///
/// Returns the matching [`ReadingType`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_reading_type(
    pool: &Pool<MySql>,
    id: ReadingTypeId,
) -> Result<Option<ReadingType>, StoreError> {
    Ok(sqlx::query_as::<_, ReadingType>(
        r#"
        SELECT
            id,
            acquisition_source_id,
            external_id,
            accumulation_behaviour,
            commodity,
            data_qualifier,
            flow_direction,
            kind,
            phase,
            power_of_ten_multiplier,
            unit_of_measure,
            created_at
        FROM reading_type
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Finds a Reading Type by acquisition source and source identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `acquisition_source_id`: Source that supplied the Reading Type.
/// - `external_id`: Source-provided Reading Type identifier.
///
/// # Returns
///
/// Returns the matching [`ReadingType`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_reading_type_by_external_id(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
    external_id: &str,
) -> Result<Option<ReadingType>, StoreError> {
    Ok(sqlx::query_as::<_, ReadingType>(
        r#"
        SELECT
            id,
            acquisition_source_id,
            external_id,
            accumulation_behaviour,
            commodity,
            data_qualifier,
            flow_direction,
            kind,
            phase,
            power_of_ten_multiplier,
            unit_of_measure,
            created_at
        FROM reading_type
        WHERE acquisition_source_id = ?
          AND external_id = ?
        "#,
    )
    .bind(acquisition_source_id.0)
    .bind(external_id)
    .fetch_optional(pool)
    .await?)
}

/// Creates a Meter Reading.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `usage_point_id`: Parent Usage Point.
/// - `reading_type_id`: Reading Type describing the values.
/// - `external_id`: Identifier supplied by the acquisition source.
///
/// # Returns
///
/// Returns the new [`MeterReadingId`].
///
/// # Errors
///
/// Returns a typed [`StoreError`] for conflicts, invalid references, or other
/// database failures.
pub async fn create_meter_reading(
    pool: &Pool<MySql>,
    usage_point_id: UsagePointId,
    reading_type_id: ReadingTypeId,
    external_id: &str,
) -> Result<MeterReadingId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO meter_reading (
            usage_point_id,
            reading_type_id,
            external_id
        )
        VALUES (?, ?, ?)
        "#,
    )
    .bind(usage_point_id.0)
    .bind(reading_type_id.0)
    .bind(external_id)
    .execute(pool)
    .await?;

    Ok(MeterReadingId(result.last_insert_id()))
}

/// Returns a Meter Reading by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Meter Reading identifier.
///
/// # Returns
///
/// Returns the matching [`MeterReading`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_meter_reading(
    pool: &Pool<MySql>,
    id: MeterReadingId,
) -> Result<Option<MeterReading>, StoreError> {
    Ok(sqlx::query_as::<_, MeterReading>(
        r#"
        SELECT
            id,
            usage_point_id,
            reading_type_id,
            external_id,
            created_at
        FROM meter_reading
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Finds a Meter Reading by parent Usage Point and source identifier.
pub async fn get_meter_reading_by_external_id(
    pool: &Pool<MySql>,
    usage_point_id: UsagePointId,
    external_id: &str,
) -> Result<Option<MeterReading>, StoreError> {
    Ok(sqlx::query_as::<_, MeterReading>(
        r#"
        SELECT
            id,
            usage_point_id,
            reading_type_id,
            external_id,
            created_at
        FROM meter_reading
        WHERE usage_point_id = ?
          AND external_id = ?
        "#,
    )
    .bind(usage_point_id.0)
    .bind(external_id)
    .fetch_optional(pool)
    .await?)
}

/// Creates an Interval Block.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `meter_reading_id`: Parent Meter Reading.
/// - `external_id`: Identifier supplied by the acquisition source.
/// - `interval_start`: UTC start of the block.
/// - `interval_duration_seconds`: Positive block duration in seconds.
///
/// # Returns
///
/// Returns the new [`IntervalBlockId`].
///
/// # Errors
///
/// Returns a typed [`StoreError`] for conflicts, invalid references, invalid
/// duration, or other database failures.
pub async fn create_interval_block(
    pool: &Pool<MySql>,
    meter_reading_id: MeterReadingId,
    external_id: &str,
    interval_start: NaiveDateTime,
    interval_duration_seconds: u64,
) -> Result<IntervalBlockId, StoreError> {
    let result = sqlx::query(
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
    .bind(meter_reading_id.0)
    .bind(external_id)
    .bind(interval_start)
    .bind(interval_duration_seconds)
    .execute(pool)
    .await?;

    Ok(IntervalBlockId(result.last_insert_id()))
}

/// Returns an Interval Block by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Interval Block identifier.
///
/// # Returns
///
/// Returns the matching [`IntervalBlock`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_interval_block(
    pool: &Pool<MySql>,
    id: IntervalBlockId,
) -> Result<Option<IntervalBlock>, StoreError> {
    Ok(sqlx::query_as::<_, IntervalBlock>(
        r#"
        SELECT
            id,
            meter_reading_id,
            external_id,
            interval_start,
            interval_duration_seconds,
            created_at
        FROM interval_block
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}
/// Finds an Interval Block by parent Meter Reading and source identifier.
pub async fn get_interval_block_by_external_id(
    pool: &Pool<MySql>,
    meter_reading_id: MeterReadingId,
    external_id: &str,
) -> Result<Option<IntervalBlock>, StoreError> {
    Ok(sqlx::query_as::<_, IntervalBlock>(
        r#"
        SELECT
            id,
            meter_reading_id,
            external_id,
            interval_start,
            interval_duration_seconds,
            created_at
        FROM interval_block
        WHERE meter_reading_id = ?
          AND external_id = ?
        "#,
    )
    .bind(meter_reading_id.0)
    .bind(external_id)
    .fetch_optional(pool)
    .await?)
}
