// SPDX-License-Identifier: MIT

//! Retrieval coverage and gap persistence.
//!
//! # Purpose
//!
//! Tracks requested or observed time ranges, their availability state, and
//! known unavailable subranges.
//!
//! # Architecture
//!
//! - ADR-0004 defines UTC timestamp handling.
//! - ADR-0010 defines idempotent handling of reimports, overlaps, and gaps.
//! - ADR-0016 defines MariaDB as the persistence boundary.
//! - ADR-0022 defines canonical retrieval coverage and gap storage.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use crate::{AcquisitionSourceId, IngestionRunId, StoreError, UsagePointId};
use sqlx::{FromRow, MySql, Pool, types::chrono::NaiveDateTime};

/// Application-controlled identifier for a retrieval coverage range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RetrievalCoverageId(pub u64);

/// Application-controlled identifier for one observed coverage state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RetrievalCoverageRevisionId(pub u64);

/// Application-controlled identifier for one known retrieval gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RetrievalGapId(pub u64);

/// Stored retrieval coverage range.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct RetrievalCoverage {
    /// Application-controlled database identifier.
    pub id: u64,
    /// Acquisition Source for which coverage is tracked.
    pub acquisition_source_id: u64,
    /// Usage Point scope, or `None` for source-wide coverage.
    pub usage_point_id: Option<u64>,
    /// UTC start of the tracked range.
    pub range_start: NaiveDateTime,
    /// UTC end of the tracked range.
    pub range_end: NaiveDateTime,
    /// UTC timestamp when this record was created.
    pub created_at: NaiveDateTime,
}

/// One observed state for a retrieval coverage range.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct RetrievalCoverageRevision {
    /// Application-controlled database identifier.
    pub id: u64,
    /// Coverage range whose state was observed.
    pub retrieval_coverage_id: u64,
    /// Ingestion run that observed this state.
    pub ingestion_run_id: u64,
    /// Database-constrained state: available, partial, or unavailable.
    pub status: String,
    /// UTC timestamp when the state was observed.
    pub observed_at: NaiveDateTime,
    /// UTC timestamp when this record was created.
    pub created_at: NaiveDateTime,
}

/// One unavailable subrange within a coverage revision.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct RetrievalGap {
    /// Application-controlled database identifier.
    pub id: u64,
    /// Coverage revision in which this gap was observed.
    pub retrieval_coverage_revision_id: u64,
    /// UTC start of the unavailable range.
    pub gap_start: NaiveDateTime,
    /// UTC end of the unavailable range.
    pub gap_end: NaiveDateTime,
    /// Optional source or application explanation.
    pub reason: Option<String>,
    /// UTC timestamp when this record was created.
    pub created_at: NaiveDateTime,
}

/// Gets or creates one canonical retrieval coverage range.
///
/// # Purpose
///
/// Makes repeated recording of the same source, scope, and time range
/// idempotent.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `acquisition_source_id`: Source being tracked.
/// - `usage_point_id`: Optional Usage Point scope; `None` means source-wide.
/// - `range_start`: UTC start of the range.
/// - `range_end`: UTC end of the range.
///
/// # Returns
///
/// Returns the existing or newly created [`RetrievalCoverageId`].
///
/// # Errors
///
/// Returns [`StoreError::Database`] for invalid references, invalid ranges, or
/// other database failures.
///
/// # Notes
///
/// ADR-0010 and ADR-0022 require stable coverage identity across repeated
/// imports. The schema-generated scope column makes `NULL` source-wide scope
/// participate correctly in MariaDB uniqueness.
pub async fn get_or_create_retrieval_coverage(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
    usage_point_id: Option<UsagePointId>,
    range_start: NaiveDateTime,
    range_end: NaiveDateTime,
) -> Result<RetrievalCoverageId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage (
            acquisition_source_id,
            usage_point_id,
            range_start,
            range_end
        )
        VALUES (?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            id = LAST_INSERT_ID(id)
        "#,
    )
    .bind(acquisition_source_id.0)
    .bind(usage_point_id.map(|id| id.0))
    .bind(range_start)
    .bind(range_end)
    .execute(pool)
    .await?;

    Ok(RetrievalCoverageId(result.last_insert_id()))
}

/// Returns a retrieval coverage range by identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Coverage identifier.
///
/// # Returns
///
/// Returns the matching [`RetrievalCoverage`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_retrieval_coverage(
    pool: &Pool<MySql>,
    id: RetrievalCoverageId,
) -> Result<Option<RetrievalCoverage>, StoreError> {
    Ok(sqlx::query_as::<_, RetrievalCoverage>(
        r#"
        SELECT
            id,
            acquisition_source_id,
            usage_point_id,
            range_start,
            range_end,
            created_at
        FROM retrieval_coverage
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Creates one observed coverage state.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `retrieval_coverage_id`: Coverage range being observed.
/// - `ingestion_run_id`: Ingestion run that observed the state.
/// - `status`: `available`, `partial`, or `unavailable`.
/// - `observed_at`: UTC observation timestamp.
///
/// # Returns
///
/// Returns the new [`RetrievalCoverageRevisionId`].
///
/// # Errors
///
/// Returns [`StoreError::Conflict`] for a duplicate coverage/run pair and
/// [`StoreError::Database`] for other database-enforced failures.
pub async fn create_retrieval_coverage_revision(
    pool: &Pool<MySql>,
    retrieval_coverage_id: RetrievalCoverageId,
    ingestion_run_id: IngestionRunId,
    status: &str,
    observed_at: NaiveDateTime,
) -> Result<RetrievalCoverageRevisionId, StoreError> {
    let result = sqlx::query(
        r#"
        INSERT INTO retrieval_coverage_revision (
            retrieval_coverage_id,
            ingestion_run_id,
            status,
            observed_at
        )
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(retrieval_coverage_id.0)
    .bind(ingestion_run_id.0)
    .bind(status)
    .bind(observed_at)
    .execute(pool)
    .await?;

    Ok(RetrievalCoverageRevisionId(result.last_insert_id()))
}

/// Returns one observed coverage state by identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: Coverage revision identifier.
///
/// # Returns
///
/// Returns the matching [`RetrievalCoverageRevision`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_retrieval_coverage_revision(
    pool: &Pool<MySql>,
    id: RetrievalCoverageRevisionId,
) -> Result<Option<RetrievalCoverageRevision>, StoreError> {
    Ok(sqlx::query_as::<_, RetrievalCoverageRevision>(
        r#"
        SELECT
            id,
            retrieval_coverage_id,
            ingestion_run_id,
            status,
            observed_at,
            created_at
        FROM retrieval_coverage_revision
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Creates one known unavailable subrange.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `retrieval_coverage_revision_id`: Coverage revision owning the gap.
/// - `gap_start`: UTC start of the gap.
/// - `gap_end`: UTC end of the gap.
/// - `reason`: Optional source or application explanation.
///
/// # Returns
///
/// Returns the new [`RetrievalGapId`].
///
/// # Errors
///
/// Returns [`StoreError::Conflict`] for duplicate gap identity and
/// [`StoreError::Database`] for invalid references, invalid ranges, or other
/// database failures.
pub async fn create_retrieval_gap(
    pool: &Pool<MySql>,
    retrieval_coverage_revision_id: RetrievalCoverageRevisionId,
    gap_start: NaiveDateTime,
    gap_end: NaiveDateTime,
    reason: Option<&str>,
) -> Result<RetrievalGapId, StoreError> {
    let result = sqlx::query(
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
    .bind(retrieval_coverage_revision_id.0)
    .bind(gap_start)
    .bind(gap_end)
    .bind(reason)
    .execute(pool)
    .await?;

    Ok(RetrievalGapId(result.last_insert_id()))
}

/// Returns all gaps for one coverage revision.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `retrieval_coverage_revision_id`: Coverage revision whose gaps are read.
///
/// # Returns
///
/// Returns gaps ordered by start time and identifier.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn list_retrieval_gaps(
    pool: &Pool<MySql>,
    retrieval_coverage_revision_id: RetrievalCoverageRevisionId,
) -> Result<Vec<RetrievalGap>, StoreError> {
    Ok(sqlx::query_as::<_, RetrievalGap>(
        r#"
        SELECT
            id,
            retrieval_coverage_revision_id,
            gap_start,
            gap_end,
            reason,
            created_at
        FROM retrieval_gap
        WHERE retrieval_coverage_revision_id = ?
        ORDER BY gap_start, id
        "#,
    )
    .bind(retrieval_coverage_revision_id.0)
    .fetch_all(pool)
    .await?)
}
