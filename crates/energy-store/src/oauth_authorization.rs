// SPDX-License-Identifier: MIT

//! OAuth authorization persistence for the initial residential scope.
//!
//! # Purpose
//!
//! Persists the current mutable OAuth authorization state for an Acquisition
//! Source without coupling authorization identity to Usage Points, meters, or
//! commodities.
//!
//! # Architecture
//!
//! - ADR-0004 defines UTC timestamp handling.
//! - ADR-0016 defines MariaDB as the persistence boundary.
//! - ADR-0018 defines credential and secret-handling boundaries.
//! - ADR-0021 defines the residential OAuth lifecycle model.
//! - ADR-0022 defines one OAuth authorization row per Acquisition Source.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use crate::{AcquisitionSourceId, StoreError};
use sqlx::{FromRow, MySql, Pool, types::chrono::NaiveDateTime};

/// Application-controlled identifier for one OAuth authorization row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OAuthAuthorizationId(pub u64);

/// Current durable OAuth authorization state for one Acquisition Source.
#[derive(Clone, PartialEq, Eq, FromRow)]
pub struct OAuthAuthorization {
    /// Application-controlled database identifier.
    pub id: u64,

    /// Acquisition Source that owns this authorization.
    pub acquisition_source_id: u64,

    /// Optional authorization identifier supplied by the Data Custodian.
    pub external_authorization_id: Option<String>,

    /// Optional Data Custodian resource-owner/account identifier.
    pub resource_owner_external_id: Option<String>,

    /// Current application authorization state.
    pub status: String,

    /// OAuth token type reported by the Data Custodian.
    pub token_type: Option<String>,

    /// Scopes actually granted by the Data Custodian.
    pub granted_scopes: Option<String>,

    /// Current sensitive access token, when persisted.
    pub access_token: Option<String>,

    /// Current sensitive refresh token, when issued and persisted.
    pub refresh_token: Option<String>,

    /// UTC expiration timestamp for the current access token.
    pub access_token_expires_at: Option<NaiveDateTime>,

    /// UTC timestamp when the current authorization was established.
    pub authorized_at: Option<NaiveDateTime>,

    /// UTC timestamp of the most recent successful refresh.
    pub last_refreshed_at: Option<NaiveDateTime>,

    /// UTC timestamp of the most recent successful authorized use.
    pub last_successful_use_at: Option<NaiveDateTime>,

    /// UTC timestamp when authorization was known to be revoked.
    pub revoked_at: Option<NaiveDateTime>,

    /// UTC timestamp when this row was created.
    pub created_at: NaiveDateTime,

    /// UTC timestamp when this row was last updated.
    pub updated_at: NaiveDateTime,
}

/// Mutable token and lifecycle fields for one authorization.
///
/// # Purpose
///
/// Groups state that may change during authorization, refresh, use,
/// reauthorization, or revocation.
#[derive(Clone, PartialEq, Eq)]
pub struct OAuthAuthorizationState<'a> {
    /// Current application authorization state.
    pub status: &'a str,

    /// OAuth token type reported by the Data Custodian.
    pub token_type: Option<&'a str>,

    /// Scopes actually granted by the Data Custodian.
    pub granted_scopes: Option<&'a str>,

    /// Current sensitive access token, when persisted.
    pub access_token: Option<&'a str>,

    /// Current sensitive refresh token, when persisted.
    pub refresh_token: Option<&'a str>,

    /// UTC expiration timestamp for the current access token.
    pub access_token_expires_at: Option<NaiveDateTime>,

    /// UTC timestamp when the current authorization was established.
    pub authorized_at: Option<NaiveDateTime>,

    /// UTC timestamp of the most recent successful refresh.
    pub last_refreshed_at: Option<NaiveDateTime>,

    /// UTC timestamp of the most recent successful authorized use.
    pub last_successful_use_at: Option<NaiveDateTime>,

    /// UTC timestamp when authorization was known to be revoked.
    pub revoked_at: Option<NaiveDateTime>,
}

/// Creates the OAuth authorization row for an Acquisition Source.
///
/// # Purpose
///
/// Establishes the single current OAuth authorization relationship permitted
/// for an Acquisition Source in the initial residential model.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `acquisition_source_id`: Acquisition Source that owns the authorization.
/// - `external_authorization_id`: Optional Data Custodian authorization ID.
/// - `resource_owner_external_id`: Optional Data Custodian account/owner ID.
/// - `status`: Initial application authorization state.
///
/// # Returns
///
/// Returns the new [`OAuthAuthorizationId`].
///
/// # Errors
///
/// Returns [`StoreError::Conflict`] when the Acquisition Source already has an
/// authorization row. Returns [`StoreError::Database`] for other database
/// failures.
///
/// # Notes
///
/// ADR-0021 and ADR-0022 intentionally model zero or one OAuth authorization
/// per Acquisition Source. Multiple Usage Points may still be accessible
/// through that one authorization.
pub async fn create_oauth_authorization(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
    external_authorization_id: Option<&str>,
    resource_owner_external_id: Option<&str>,
    status: &str,
) -> Result<OAuthAuthorizationId, StoreError> {
    let result = sqlx::query(
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
    .bind(acquisition_source_id.0)
    .bind(external_authorization_id)
    .bind(resource_owner_external_id)
    .bind(status)
    .execute(pool)
    .await?;

    Ok(OAuthAuthorizationId(result.last_insert_id()))
}

/// Returns one OAuth authorization by application identifier.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: OAuth authorization identifier.
///
/// # Returns
///
/// Returns the matching [`OAuthAuthorization`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_oauth_authorization(
    pool: &Pool<MySql>,
    id: OAuthAuthorizationId,
) -> Result<Option<OAuthAuthorization>, StoreError> {
    Ok(sqlx::query_as::<_, OAuthAuthorization>(
        r#"
        SELECT
            id,
            acquisition_source_id,
            external_authorization_id,
            resource_owner_external_id,
            status,
            token_type,
            granted_scopes,
            access_token,
            refresh_token,
            access_token_expires_at,
            authorized_at,
            last_refreshed_at,
            last_successful_use_at,
            revoked_at,
            created_at,
            updated_at
        FROM oauth_authorization
        WHERE id = ?
        "#,
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await?)
}

/// Returns the OAuth authorization for an Acquisition Source.
///
/// # Purpose
///
/// Provides the normal residential lookup path without inventing a separate
/// authorization key or account-management layer.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `acquisition_source_id`: Acquisition Source whose authorization is read.
///
/// # Returns
///
/// Returns the source's [`OAuthAuthorization`] when present.
///
/// # Errors
///
/// Returns [`StoreError::Database`] if the query fails.
pub async fn get_oauth_authorization_by_source(
    pool: &Pool<MySql>,
    acquisition_source_id: AcquisitionSourceId,
) -> Result<Option<OAuthAuthorization>, StoreError> {
    Ok(sqlx::query_as::<_, OAuthAuthorization>(
        r#"
        SELECT
            id,
            acquisition_source_id,
            external_authorization_id,
            resource_owner_external_id,
            status,
            token_type,
            granted_scopes,
            access_token,
            refresh_token,
            access_token_expires_at,
            authorized_at,
            last_refreshed_at,
            last_successful_use_at,
            revoked_at,
            created_at,
            updated_at
        FROM oauth_authorization
        WHERE acquisition_source_id = ?
        "#,
    )
    .bind(acquisition_source_id.0)
    .fetch_optional(pool)
    .await?)
}

/// Replaces mutable OAuth token and lifecycle state.
///
/// # Purpose
///
/// Updates the existing logical authorization during authorization, refresh,
/// reauthorization, normal use, or revocation.
///
/// # Inputs
///
/// - `pool`: Active MariaDB connection pool.
/// - `id`: OAuth authorization row to update.
/// - `state`: Complete mutable OAuth state to persist.
///
/// # Returns
///
/// Returns `Ok(())` when the authorization is updated.
///
/// # Errors
///
/// Returns [`StoreError::NotFound`] when the authorization does not exist.
/// Returns [`StoreError::Database`] for other database failures.
///
/// # Notes
///
/// This function does not implement OAuth protocol behavior. A maintained OAuth
/// implementation should produce the state that is persisted here.
pub async fn update_oauth_authorization_state(
    pool: &Pool<MySql>,
    id: OAuthAuthorizationId,
    state: &OAuthAuthorizationState<'_>,
) -> Result<(), StoreError> {
    let result = sqlx::query(
        r#"
        UPDATE oauth_authorization
        SET
            status = ?,
            token_type = ?,
            granted_scopes = ?,
            access_token = ?,
            refresh_token = ?,
            access_token_expires_at = ?,
            authorized_at = ?,
            last_refreshed_at = ?,
            last_successful_use_at = ?,
            revoked_at = ?
        WHERE id = ?
        "#,
    )
    .bind(state.status)
    .bind(state.token_type)
    .bind(state.granted_scopes)
    .bind(state.access_token)
    .bind(state.refresh_token)
    .bind(state.access_token_expires_at)
    .bind(state.authorized_at)
    .bind(state.last_refreshed_at)
    .bind(state.last_successful_use_at)
    .bind(state.revoked_at)
    .bind(id.0)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StoreError::NotFound);
    }

    Ok(())
}
