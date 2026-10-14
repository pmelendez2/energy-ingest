// SPDX-License-Identifier: MIT

//! Canonical interval-reading and revision persistence.
//!
//! # Purpose
//!
//! Provides the provider-independent persistence API for canonical interval
//! identities and their observed revisions.
//!
//! # Architecture
//!
//! - ADR-0004 defines UTC timestamp handling.
//! - ADR-0005 requires revision-aware imports so changed source values are
//!   retained rather than silently overwritten.
//! - ADR-0016 defines MariaDB as the persistence boundary and keeps database
//!   access inside `energy-store`.
//! - ADR-0022 defines the canonical interval-reading and revision model.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use crate::{IngestionRunId, IntervalBlockId, MeterReadingId, StoreError};
use sqlx::{FromRow, MySql, Pool, types::chrono::NaiveDateTime};

/// Application-controlled identifier for a canonical interval reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalReadingId(pub u64);

/// Application-controlled identifier for an observed interval-reading revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalReadingRevisionId(pub u64);

/// Canonical identity of one meter interval.
///
/// The canonical row identifies an interval independently of any particular
/// value observed for it. Revisions are stored separately so later imports can
/// retain source history instead of overwriting earlier observations.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct IntervalReading {
    /// Application-controlled database identifier.
    pub id: u64,

    /// Meter Reading to which this interval belongs.
    pub meter_reading_id: u64,

    /// UTC start timestamp of the interval.
    pub interval_start: NaiveDateTime,

    /// Length of the interval in seconds.
    pub interval_duration_seconds: u32,

    /// Revision currently accepted as the value for this interval.
    pub accepted_revision_id: Option<u64>,

    /// UTC timestamp when this canonical interval was created.
    pub created_at: NaiveDateTime,
}

/// One observed version of an interval-reading value.
///
/// ADR-0005 requires revisions to be retained so imports can represent source
/// corrections or changed values without losing prior observations.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct IntervalReadingRevision {
    /// Application-controlled database identifier.
    pub id: u64,

    /// Canonical interval reading represented by this revision.
    pub interval_reading_id: u64,

    /// Ingestion run that observed this revision.
    pub ingestion_run_id: u64,

    /// Source Interval Block containing the revision, when available.
    pub interval_block_id: Option<u64>,

    /// Source-provided identifier for the revision, when available.
    pub source_external_id: Option<String>,

    /// UTC source publication timestamp, when supplied.
    pub source_published_at: Option<NaiveDateTime>,

    /// UTC source update timestamp, when supplied.
    pub source_updated_at: Option<NaiveDateTime>,

    /// Raw reading value before applying the Reading Type multiplier.
    pub reading_value: i64,

    /// Source-provided reading quality information, when available.
    pub reading_quality: Option<String>,

    /// UTC timestamp when this revision was stored.
    pub created_at: NaiveDateTime,
}

/// Gets or creates the canonical identity for one meter interval.
///
/// # Purpose
///
/// Makes repeated imports idempotent at the canonical interval-identity level.
/// The schema-defined identity is the Meter Reading, UTC interval start, and
/// interval duration.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `meter_reading_id`: Meter Reading that owns the interval.
/// - `interval_start`: UTC start timestamp of the interval.
/// - `interval_duration_seconds`: Positive interval length in seconds.
///
/// # Returns
///
/// Returns the existing or newly created [`IntervalReadingId`].
///
/// # Errors
///
/// Returns [`StoreError::Database`] for invalid foreign keys, invalid duration,
/// or other database failures.
///
/// # Notes
///
/// ADR-0005 requires repeated imports to preserve a stable canonical interval
/// identity while storing changed observations as revisions.
///
/// `LAST_INSERT_ID(id)` is intentionally used on the duplicate-key path so the
/// same statement returns the existing canonical identifier without requiring
/// a race-prone select-then-insert sequence.
pub async fn get_or_create_interval_reading(
    pool: &Pool<MySql>,
    meter_reading_id: MeterReadingId,
    interval_start: NaiveDateTime,
    interval_duration_seconds: u32,
) -> Result<IntervalReadingId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO interval_reading (
            meter_reading_id,
            interval_start,
            interval_duration_seconds
        )
        VALUES (?, ?, ?)
        ON DUPLICATE KEY UPDATE
            id = LAST_INSERT_ID(id)
        "#,
    )
    .bind(meter_reading_id.0)
    .bind(interval_start)
    .bind(interval_duration_seconds)
    .execute(pool)
    .await?;

    Ok(IntervalReadingId(result.last_insert_id()))
}

/// Returns a canonical interval reading by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Canonical interval-reading identifier.
///
/// # Returns
///
/// Returns the matching [`IntervalReading`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_interval_reading(
    pool: &Pool<MySql>,
    id: IntervalReadingId,
) -> Result<Option<IntervalReading>, StoreError> {
    Ok(sqlx::query_as::<_, IntervalReading>(
        r#"
        SELECT
            id,
            meter_reading_id,
            interval_start,
            interval_duration_seconds,
            accepted_revision_id,
            created_at
        FROM interval_reading
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Finds a canonical interval reading by its schema-defined identity.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `meter_reading_id`: Meter Reading that owns the interval.
/// - `interval_start`: UTC start timestamp of the interval.
/// - `interval_duration_seconds`: Interval length in seconds.
///
/// # Returns
///
/// Returns the matching [`IntervalReading`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_interval_reading_by_identity(
    pool: &Pool<MySql>,
    meter_reading_id: MeterReadingId,
    interval_start: NaiveDateTime,
    interval_duration_seconds: u32,
) -> Result<Option<IntervalReading>, StoreError> {
    Ok(sqlx::query_as::<_, IntervalReading>(
        r#"
        SELECT
            id,
            meter_reading_id,
            interval_start,
            interval_duration_seconds,
            accepted_revision_id,
            created_at
        FROM interval_reading
        WHERE meter_reading_id = ?
          AND interval_start = ?
          AND interval_duration_seconds = ?
        "#,
    )
    .bind(meter_reading_id.0)
    .bind(interval_start)
    .bind(interval_duration_seconds)
    .fetch_optional(pool)
    .await?)
}

/// Creates an observed revision for a canonical interval reading.
///
/// # Purpose
///
/// Stores an observed value without overwriting earlier observations.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `interval_reading_id`: Canonical interval represented by this revision.
/// - `ingestion_run_id`: Ingestion run that observed the revision.
/// - `interval_block_id`: Source Interval Block, when available.
/// - `source_external_id`: Source-provided revision identifier, when available.
/// - `source_published_at`: UTC source publication timestamp, when supplied.
/// - `source_updated_at`: UTC source update timestamp, when supplied.
/// - `reading_value`: Raw source reading before multiplier application.
/// - `reading_quality`: Source-provided quality information, when available.
///
/// # Returns
///
/// Returns the new [`IntervalReadingRevisionId`].
///
/// # Errors
///
/// Returns [`StoreError::Database`] for invalid references or other database
/// failures.
///
/// # Notes
///
/// ADR-0005 permits multiple observed revisions during one ingestion run.
/// The database therefore does not impose a one-revision-per-run restriction.
#[allow(clippy::too_many_arguments)]
pub async fn create_interval_reading_revision(
    pool: &Pool<MySql>,
    interval_reading_id: IntervalReadingId,
    ingestion_run_id: IngestionRunId,
    interval_block_id: Option<IntervalBlockId>,
    source_external_id: Option<&str>,
    source_published_at: Option<NaiveDateTime>,
    source_updated_at: Option<NaiveDateTime>,
    reading_value: i64,
    reading_quality: Option<&str>,
) -> Result<IntervalReadingRevisionId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO interval_reading_revision (
            interval_reading_id,
            ingestion_run_id,
            interval_block_id,
            source_external_id,
            source_published_at,
            source_updated_at,
            reading_value,
            reading_quality
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(interval_reading_id.0)
    .bind(ingestion_run_id.0)
    .bind(interval_block_id.map(|id| id.0))
    .bind(source_external_id)
    .bind(source_published_at)
    .bind(source_updated_at)
    .bind(reading_value)
    .bind(reading_quality)
    .execute(pool)
    .await?;

    Ok(IntervalReadingRevisionId(result.last_insert_id()))
}

/// Returns one interval-reading revision by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Revision identifier.
///
/// # Returns
///
/// Returns the matching [`IntervalReadingRevision`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_interval_reading_revision(
    pool: &Pool<MySql>,
    id: IntervalReadingRevisionId,
) -> Result<Option<IntervalReadingRevision>, StoreError> {
    Ok(sqlx::query_as::<_, IntervalReadingRevision>(
        r#"
        SELECT
            id,
            interval_reading_id,
            ingestion_run_id,
            interval_block_id,
            source_external_id,
            source_published_at,
            source_updated_at,
            reading_value,
            reading_quality,
            created_at
        FROM interval_reading_revision
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Returns all stored revisions for one canonical interval reading.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `interval_reading_id`: Canonical interval whose history should be read.
///
/// # Returns
///
/// Returns revisions ordered by application identifier, which also preserves
/// insertion order for the current auto-increment schema.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn list_interval_reading_revisions(
    pool: &Pool<MySql>,
    interval_reading_id: IntervalReadingId,
) -> Result<Vec<IntervalReadingRevision>, StoreError> {
    Ok(sqlx::query_as::<_, IntervalReadingRevision>(
        r#"
        SELECT
            id,
            interval_reading_id,
            ingestion_run_id,
            interval_block_id,
            source_external_id,
            source_published_at,
            source_updated_at,
            reading_value,
            reading_quality,
            created_at
        FROM interval_reading_revision
        WHERE interval_reading_id = ?
        ORDER BY id
        "#,
    )
    .bind(interval_reading_id.0)
    .fetch_all(pool)
    .await?)
}

/// Selects the accepted revision for a canonical interval reading.
///
/// # Purpose
///
/// Records which observed revision the application currently accepts as the
/// canonical value while retaining all other revisions as history.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `interval_reading_id`: Canonical interval to update.
/// - `revision_id`: Revision to accept.
///
/// # Returns
///
/// Returns `Ok(())` when the canonical interval is updated.
///
/// # Errors
///
/// - [`StoreError::NotFound`] when the canonical interval does not exist.
/// - [`StoreError::Database`] when the revision does not belong to the same
///   canonical interval or another database constraint fails.
///
/// # Notes
///
/// ADR-0022 requires the database to prevent accepting a revision belonging to
/// another canonical interval. The composite foreign key in migration 0003 is
/// the authoritative enforcement mechanism.
pub async fn set_accepted_interval_reading_revision(
    pool: &Pool<MySql>,
    interval_reading_id: IntervalReadingId,
    revision_id: IntervalReadingRevisionId,
) -> Result<(), StoreError> {
    let result = sqlx::query(
        r#"
        UPDATE interval_reading
        SET accepted_revision_id = ?
        WHERE id = ?
        "#,
    )
    .bind(revision_id.0)
    .bind(interval_reading_id.0)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StoreError::NotFound);
    }

    Ok(())
}

/// Outcome of applying one observed interval to the accepted revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationOutcome {
    /// Accepted source value and metadata already match this observation.
    Unchanged(IntervalReadingRevisionId),
    /// A different observation was stored and selected as accepted.
    Revised(IntervalReadingRevisionId),
}

/// Stores and accepts an observation only when it differs from the accepted
/// revision. This is the default operation for idempotent imports.
///
/// The comparison includes the value, quality, block, source identity and
/// publication/update timestamps. Provenance of unchanged reimports remains
/// in the ingestion run; a duplicate reading revision is not created.
///
/// Callers must serialize concurrent imports for a given acquisition source.
/// This check and subsequent insert are not a single database transaction.
#[allow(clippy::too_many_arguments)]
pub async fn accept_interval_observation(
    pool: &Pool<MySql>,
    interval_reading_id: IntervalReadingId,
    ingestion_run_id: IngestionRunId,
    interval_block_id: Option<IntervalBlockId>,
    source_external_id: Option<&str>,
    source_published_at: Option<NaiveDateTime>,
    source_updated_at: Option<NaiveDateTime>,
    reading_value: i64,
    reading_quality: Option<&str>,
) -> Result<ObservationOutcome, StoreError> {
    let current = get_interval_reading(pool, interval_reading_id)
        .await?
        .ok_or(StoreError::NotFound)?;

    if let Some(accepted_id) = current.accepted_revision_id {
        let revision = get_interval_reading_revision(
            pool,
            IntervalReadingRevisionId(accepted_id),
        )
        .await?
        .ok_or(StoreError::NotFound)?;

        if revision.reading_value == reading_value
            && revision.reading_quality.as_deref() == reading_quality
            && revision.interval_block_id == interval_block_id.map(|id| id.0)
            && revision.source_external_id.as_deref() == source_external_id
            && revision.source_published_at == source_published_at
            && revision.source_updated_at == source_updated_at
        {
            return Ok(ObservationOutcome::Unchanged(
                IntervalReadingRevisionId(revision.id),
            ));
        }
    }

    let revision_id = create_interval_reading_revision(
        pool,
        interval_reading_id,
        ingestion_run_id,
        interval_block_id,
        source_external_id,
        source_published_at,
        source_updated_at,
        reading_value,
        reading_quality,
    )
    .await?;

    set_accepted_interval_reading_revision(
        pool,
        interval_reading_id,
        revision_id,
    )
    .await?;
    Ok(ObservationOutcome::Revised(revision_id))
}
