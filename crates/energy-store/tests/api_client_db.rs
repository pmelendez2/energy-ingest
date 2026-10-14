// SPDX-License-Identifier: MIT

//! Database-backed API credential lifecycle using the isolated test DB.

use energy_store::{
    authenticate_api_token, create_api_client, list_api_clients,
    revoke_api_client,
};
use sqlx::MySqlPool;

#[tokio::test]
async fn token_lifecycle_and_independent_revocation() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL required for integration tests");
    let pool = MySqlPool::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();

    // Stable names are unique for the disposable test environment.
    let first = create_api_client(&pool, "test-ha-client").await.unwrap();
    let second = create_api_client(&pool, "test-diagnostics").await.unwrap();
    assert_ne!(first.token, second.token);
    assert_eq!(
        authenticate_api_token(&pool, &first.token).await.unwrap(),
        Some(first.client_id)
    );
    assert_eq!(
        authenticate_api_token(&pool, &second.token).await.unwrap(),
        Some(second.client_id)
    );
    assert_eq!(authenticate_api_token(&pool, "invalid").await.unwrap(), None);
    let clients = list_api_clients(&pool).await.unwrap();
    assert!(clients.iter().any(|client| client.id == first.client_id));
    assert!(revoke_api_client(&pool, first.client_id).await.unwrap());
    assert!(!revoke_api_client(&pool, first.client_id).await.unwrap());
    assert_eq!(
        authenticate_api_token(&pool, &first.token).await.unwrap(),
        None
    );
    assert_eq!(
        authenticate_api_token(&pool, &second.token).await.unwrap(),
        Some(second.client_id)
    );
}
