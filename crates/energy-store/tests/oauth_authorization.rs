// SPDX-License-Identifier: MIT

//! Integration tests for residential OAuth authorization persistence.
//!
//! # Architecture
//!
//! These tests provide implementation evidence for ADR-0016, ADR-0018,
//! ADR-0020, ADR-0021, and ADR-0022.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use energy_store::{
    OAuthAuthorizationId, OAuthAuthorizationState, StoreError,
    create_acquisition_source, create_data_custodian,
    create_oauth_authorization, create_usage_point, get_oauth_authorization,
    get_oauth_authorization_by_source, update_oauth_authorization_state,
};
use sqlx::{MySql, Pool, types::chrono::NaiveDateTime};

/// Verifies one authorization can coexist with multiple Usage Points.
///
/// # Test Goal
///
/// Confirm the residential model in ADR-0021 and ADR-0022: OAuth authorization
/// belongs to the Acquisition Source rather than to an individual Usage Point,
/// meter, or commodity.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - One synthetic Acquisition Source with electric and gas Usage Points.
///
/// # Expected Outcomes
///
/// - two Usage Points can exist under one Acquisition Source;
/// - only one OAuth authorization is needed for that source;
/// - authorization lookup by source succeeds;
/// - mutable token/lifecycle state can be updated; and
/// - test-created records are removed.
#[tokio::test]
async fn oauth_authorization_repository_supports_multiple_usage_points() {
    let pool = test_pool().await;
    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let suffix = unique_suffix();

    let custodian_id =
        create_data_custodian(&pool, &format!("oauth-custodian-{suffix}"))
            .await
            .expect("custodian creation should succeed");

    let source_id = create_acquisition_source(
        &pool,
        custodian_id,
        &format!("oauth-source-{suffix}"),
    )
    .await
    .expect("source creation should succeed");

    create_usage_point(
        &pool,
        source_id,
        &format!("electric-{suffix}"),
        Some(0),
    )
    .await
    .expect("electric Usage Point creation should succeed");

    create_usage_point(&pool, source_id, &format!("gas-{suffix}"), Some(0))
        .await
        .expect("gas Usage Point creation should succeed");

    let external_authorization_id = format!("external-auth-{suffix}");
    let resource_owner_external_id = format!("resource-owner-{suffix}");

    let authorization_id = create_oauth_authorization(
        &pool,
        source_id,
        Some(&external_authorization_id),
        Some(&resource_owner_external_id),
        "active",
    )
    .await
    .expect("OAuth authorization creation should succeed");

    let by_source = get_oauth_authorization_by_source(&pool, source_id)
        .await
        .expect("OAuth authorization source lookup should succeed")
        .expect("OAuth authorization should exist");

    assert_eq!(by_source.id, authorization_id.0);
    assert_eq!(by_source.acquisition_source_id, source_id.0);

    let state = OAuthAuthorizationState {
        status: "active",
        token_type: Some("Bearer"),
        granted_scopes: Some("read"),
        access_token: Some("synthetic-access-token"),
        refresh_token: Some("synthetic-refresh-token"),
        access_token_expires_at: Some(timestamp("2026-10-02 18:00:00")),
        authorized_at: Some(timestamp("2026-10-02 16:00:00")),
        last_refreshed_at: Some(timestamp("2026-10-02 17:00:00")),
        last_successful_use_at: Some(timestamp("2026-10-02 17:05:00")),
        revoked_at: None,
    };

    update_oauth_authorization_state(&pool, authorization_id, &state)
        .await
        .expect("OAuth authorization state update should succeed");

    let updated = get_oauth_authorization(&pool, authorization_id)
        .await
        .expect("updated OAuth authorization lookup should succeed")
        .expect("updated OAuth authorization should exist");

    assert_eq!(updated.status, "active");
    assert_eq!(updated.token_type.as_deref(), Some("Bearer"));
    assert_eq!(updated.granted_scopes.as_deref(), Some("read"));
    assert_eq!(updated.access_token.as_deref(), Some("synthetic-access-token"));
    assert_eq!(
        updated.refresh_token.as_deref(),
        Some("synthetic-refresh-token")
    );

    cleanup(&pool, source_id.0, custodian_id.0)
        .await
        .expect("integration-test cleanup should succeed");
}

/// Verifies one OAuth authorization row per Acquisition Source.
///
/// # Test Goal
///
/// Confirm the database enforces the simplified residential invariant defined
/// by ADR-0021 and ADR-0022.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - One synthetic Acquisition Source.
///
/// # Expected Outcomes
///
/// - the first authorization is accepted;
/// - a second authorization for the same Acquisition Source returns
///   [`StoreError::Conflict`]; and
/// - test-created records are removed.
#[tokio::test]
async fn oauth_authorization_repository_enforces_one_per_source() {
    let pool = test_pool().await;
    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let suffix = unique_suffix();

    let custodian_id = create_data_custodian(
        &pool,
        &format!("oauth-constraint-custodian-{suffix}"),
    )
    .await
    .expect("custodian creation should succeed");

    let source_id = create_acquisition_source(
        &pool,
        custodian_id,
        &format!("oauth-constraint-source-{suffix}"),
    )
    .await
    .expect("source creation should succeed");

    create_oauth_authorization(&pool, source_id, None, None, "active")
        .await
        .expect("first OAuth authorization creation should succeed");

    let duplicate = create_oauth_authorization(
        &pool,
        source_id,
        Some("different-external-id"),
        None,
        "active",
    )
    .await
    .expect_err("second OAuth authorization for one source should fail");

    assert!(
        matches!(duplicate, StoreError::Conflict),
        "duplicate source authorization should return StoreError::Conflict"
    );

    cleanup(&pool, source_id.0, custodian_id.0)
        .await
        .expect("integration-test cleanup should succeed");
}

/// Verifies typed not-found behavior for unknown authorization updates.
///
/// # Test Goal
///
/// Confirm an update cannot silently succeed when the authorization row does
/// not exist.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - A deliberately nonexistent authorization identifier.
///
/// # Expected Outcomes
///
/// The update returns [`StoreError::NotFound`].
#[tokio::test]
async fn oauth_authorization_repository_returns_not_found_for_unknown_update() {
    let pool = test_pool().await;
    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let state = OAuthAuthorizationState {
        status: "active",
        token_type: None,
        granted_scopes: None,
        access_token: None,
        refresh_token: None,
        access_token_expires_at: None,
        authorized_at: None,
        last_refreshed_at: None,
        last_successful_use_at: None,
        revoked_at: None,
    };

    let error = update_oauth_authorization_state(
        &pool,
        OAuthAuthorizationId(u64::MAX),
        &state,
    )
    .await
    .expect_err("unknown OAuth authorization update should fail");

    assert!(
        matches!(error, StoreError::NotFound),
        "unknown OAuth authorization should return StoreError::NotFound"
    );
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
fn timestamp(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .expect("static test timestamp should parse")
}

/// Removes synthetic OAuth and parent records.
///
/// # Inputs
///
/// - `pool`: Active MariaDB test connection pool.
/// - `source_id`: Acquisition Source created by the test.
/// - `custodian_id`: Data Custodian created by the test.
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
        "DELETE FROM oauth_authorization WHERE acquisition_source_id = ?",
    )
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
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos()
}
