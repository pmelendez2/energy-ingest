// SPDX-License-Identifier: MIT
//! HTTP contract checks for canonical resource association lookup.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use energy_store::{create_api_client, revoke_api_client};
use serde_json::Value;
use sqlx::MySqlPool;
use tower::ServiceExt;

async fn get(
    pool: &MySqlPool,
    path: &str,
    token: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder().uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    energy_api::router(pool.clone())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn lookup_requires_auth_and_distinguishes_invalid_unknown_ids() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL required for integration tests");
    let pool = MySqlPool::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();
    let client =
        create_api_client(&pool, "association-lookup-test").await.unwrap();

    let unauthenticated = get(&pool, "/v1/resources/1/association", None).await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let invalid =
        get(&pool, "/v1/resources/0/association", Some(&client.token)).await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let unknown = get(
        &pool,
        "/v1/resources/18446744073709551615/association",
        Some(&client.token),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(unknown.into_body(), 4096).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "resource_not_found");

    assert!(revoke_api_client(&pool, client.client_id).await.unwrap());
    let revoked =
        get(&pool, "/v1/resources/1/association", Some(&client.token)).await;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
}
