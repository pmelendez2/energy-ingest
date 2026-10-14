// SPDX-License-Identifier: MIT

//! Data source and ingestion-run storage.
//!
//! # Purpose
//!
//! Stores where imported data came from and records each ingestion operation
//! using plain application-controlled identifiers.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use crate::StoreError;
use sqlx::{FromRow, MySql, Pool, types::chrono::NaiveDateTime};

/// Application-controlled identifier for a Data Custodian.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataCustodianId(pub u64);

/// Application-controlled identifier for an Acquisition Source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AcquisitionSourceId(pub u64);

/// Application-controlled identifier for an Ingestion Run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IngestionRunId(pub u64);

/// Stored organization responsible for an acquisition source.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct DataCustodian {
    /// Application-controlled database identifier.
    pub id: u64,
    /// Human-readable custodian name.
    pub name: String,
}

/// Stored source from which energy data is acquired.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct AcquisitionSource {
    /// Application-controlled database identifier.
    pub id: u64,
    /// Data Custodian that owns or operates this source.
    pub data_custodian_id: u64,
    /// Human-readable source name.
    pub name: String,
}

/// Stored record of one ingestion operation.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct IngestionRun {
    /// Application-controlled database identifier.
    pub id: u64,
    /// Source used by this ingestion run.
    pub acquisition_source_id: u64,
    /// UTC time at which the ingestion run started.
    pub started_at: NaiveDateTime,
}

/// Creates a Data Custodian record.
///
/// # Purpose
///
/// Stores the organization responsible for one or more acquisition sources.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `name`: Human-readable Data Custodian name.
///
/// # Returns
///
/// Returns the new [`DataCustodianId`].
///
/// # Errors
///
/// - [`StoreError::Conflict`] if the name already exists.
/// - [`StoreError::Database`] for other database failures.
pub async fn create_data_custodian(
    pool: &Pool<MySql>,
    name: &str,
) -> Result<DataCustodianId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO data_custodian (name)
        VALUES (?)
        "#,
    )
    .bind(name)
    .execute(pool)
    .await?;

    Ok(DataCustodianId(result.last_insert_id()))
}

/// Returns a Data Custodian by application identifier.
///
/// # Purpose
///
/// Retrieves a previously stored Data Custodian.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Data Custodian identifier.
///
/// # Returns
///
/// Returns `Some(DataCustodian)` when found, otherwise `None`.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_data_custodian(
    pool: &Pool<MySql>,
    id: DataCustodianId,
) -> Result<Option<DataCustodian>, StoreError> {
    Ok(sqlx::query_as::<_, DataCustodian>(
        r#"
        SELECT id, name
        FROM data_custodian
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Creates an Acquisition Source for a Data Custodian.
///
/// # Purpose
///
/// Stores a named source from which energy data is acquired.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `data_custodian_id`: Owning Data Custodian.
/// - `name`: Human-readable source name.
///
/// # Returns
///
/// Returns the new [`AcquisitionSourceId`].
///
/// # Errors
///
/// - [`StoreError::Conflict`] for uniqueness conflicts.
/// - [`StoreError::Database`] for invalid references or other database errors.
pub async fn create_acquisition_source(
    pool: &Pool<MySql>,
    data_custodian_id: DataCustodianId,
    name: &str,
) -> Result<AcquisitionSourceId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO acquisition_source (
            data_custodian_id,
            name
        )
        VALUES (?, ?)
        "#,
    )
    .bind(data_custodian_id.0)
    .bind(name)
    .execute(pool)
    .await?;

    Ok(AcquisitionSourceId(result.last_insert_id()))
}

/// Returns an Acquisition Source by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Acquisition Source identifier.
///
/// # Returns
///
/// Returns `Some(AcquisitionSource)` when found, otherwise `None`.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_acquisition_source(
    pool: &Pool<MySql>,
    id: AcquisitionSourceId,
) -> Result<Option<AcquisitionSource>, StoreError> {
    Ok(sqlx::query_as::<_, AcquisitionSource>(
        r#"
        SELECT id, data_custodian_id, name
        FROM acquisition_source
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Finds an Acquisition Source by Data Custodian and source name.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `data_custodian_id`: Owning Data Custodian.
/// - `name`: Source name to locate.
///
/// # Returns
///
/// Returns the matching source when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_source_by_name(
    pool: &Pool<MySql>,
    data_custodian_id: DataCustodianId,
    name: &str,
) -> Result<Option<AcquisitionSource>, StoreError> {
    Ok(sqlx::query_as::<_, AcquisitionSource>(
        r#"
        SELECT id, data_custodian_id, name
        FROM acquisition_source
        WHERE data_custodian_id = ?
          AND name = ?
        "#,
    )
    .bind(data_custodian_id.0)
    .bind(name)
    .fetch_optional(pool)
    .await?)
}

/// Creates an Ingestion Run using the database UTC clock.
///
/// # Purpose
///
/// Records the start of one ingestion operation for an Acquisition Source.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `acquisition_source_id`: Source being ingested.
///
/// # Returns
///
/// Returns the new [`IngestionRunId`].
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the insert fails.
pub async fn create_ingestion_run(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
) -> Result<IngestionRunId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO ingestion_run (
            acquisition_source_id,
            started_at
        )
        VALUES (?, UTC_TIMESTAMP(6))
        "#,
    )
    .bind(acquisition_source_id.0)
    .execute(pool)
    .await?;

    Ok(IngestionRunId(result.last_insert_id()))
}

/// Returns an Ingestion Run by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Ingestion Run identifier.
///
/// # Returns
///
/// Returns the matching run when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_ingestion_run(
    pool: &Pool<MySql>,
    id: IngestionRunId,
) -> Result<Option<IngestionRun>, StoreError> {
    Ok(sqlx::query_as::<_, IngestionRun>(
        r#"
        SELECT id, acquisition_source_id, started_at
        FROM ingestion_run
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}
