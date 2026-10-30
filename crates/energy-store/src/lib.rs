// SPDX-License-Identifier: MIT

//! MariaDB persistence for `energy_ingest`.
//!
//! # Purpose
//!
//! Exposes stable persistence types and functions while keeping
//! SQLx-specific behavior inside `energy-store`.
//!
//! # Architecture
//!
//! The persistence boundary follows the accepted project ADRs.
//! Database schema definitions remain authoritative in the SQL
//! migrations, while this crate provides typed application access to
//! that schema.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

mod api_client;
mod bootstrap;
mod error;
mod gb;
mod gb_import;
mod history_sync;
mod ingestion_source;
mod interval_reading;
mod meter_association;
mod oauth_authorization;
mod retrieval_coverage;
mod usage_stream;

pub use api_client::{
    ApiClient, IssuedApiToken, authenticate_api_token, create_api_client,
    list_api_clients, revoke_api_client,
};
pub use bootstrap::{
    DatabaseBootstrapError, DatabaseBootstrapResult, bootstrap_database,
};
pub use error::StoreError;
pub use gb::{
    IntervalBlock, IntervalBlockId, MeterReading, MeterReadingId, ReadingType,
    ReadingTypeId, ReadingTypeValues, UsagePoint, UsagePointId,
    create_interval_block, create_meter_reading, create_reading_type,
    create_reading_type_with_values, create_usage_point, get_interval_block,
    get_interval_block_by_external_id, get_meter_reading,
    get_meter_reading_by_external_id, get_reading_type,
    get_reading_type_by_external_id, get_usage_point,
    get_usage_point_by_external_id,
};
pub use gb_import::{
    GbImportError, GbImportReport, import_gb_document_atomic,
    import_gb_document_atomic_with_hash,
};
pub use history_sync::{
    HistoryAction, HistoryPlanError, HistoryWindow, HistoryWindowAction,
    plan_history_refresh,
};
pub use ingestion_source::{
    AcquisitionSource, AcquisitionSourceId, DataCustodian, DataCustodianId,
    IngestionRun, IngestionRunId, create_acquisition_source,
    create_data_custodian, create_ingestion_run, get_acquisition_source,
    get_data_custodian, get_ingestion_run, get_source_by_name,
};
pub use interval_reading::{
    IntervalReading, IntervalReadingId, IntervalReadingRevision,
    IntervalReadingRevisionId, ObservationOutcome, accept_interval_observation,
    create_interval_reading_revision, get_interval_reading,
    get_interval_reading_by_identity, get_interval_reading_revision,
    get_or_create_interval_reading, list_interval_reading_revisions,
    set_accepted_interval_reading_revision,
};
pub use meter_association::{
    AssociationError, AssociationOutcome, associate_meter_resource,
};
pub use oauth_authorization::{
    OAuthAuthorization, OAuthAuthorizationId, OAuthAuthorizationState,
    create_oauth_authorization, get_oauth_authorization,
    get_oauth_authorization_by_source, update_oauth_authorization_state,
};
pub use retrieval_coverage::{
    RetrievalCoverage, RetrievalCoverageId, RetrievalCoverageRevision,
    RetrievalCoverageRevisionId, RetrievalGap, RetrievalGapId,
    create_retrieval_coverage_revision, create_retrieval_gap,
    get_or_create_retrieval_coverage, get_retrieval_coverage,
    get_retrieval_coverage_revision, list_retrieval_gaps,
};

mod window_fingerprint;
pub use window_fingerprint::window_fingerprint;

pub use usage_stream::{
    MAX_WINDOW_CANDIDATES, MeterValidation, SelectedInterval, StreamCandidate,
    StreamIssue, StreamIssueKind, StreamResolution, ValidationStatus,
    resolve_candidates, resolve_candidates_with_gas_zone, resolve_meter_window,
    resolve_physical_meter_stream, summarize_meter_validation,
};

use sqlx::{MySql, Pool};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Applies all pending database migrations.
///
/// # Purpose
///
/// Brings the connected database to the schema version embedded in
/// this crate.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
///
/// # Returns
///
/// Returns `Ok(())` when all pending migrations are applied.
///
/// # Errors
///
/// Returns [`sqlx::migrate::MigrateError`] when the database cannot be
/// reached or a migration cannot be applied.
pub async fn migrate(
    pool: &Pool<MySql>,
) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}
