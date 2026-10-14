// SPDX-License-Identifier: MIT
//! Isolated MariaDB-backed HTTP authorization tests.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use energy_store::{create_api_client, revoke_api_client};
use serde_json::Value;
use sqlx::MySqlPool;
use tower::ServiceExt;

async fn request(
    pool: &MySqlPool,
    path: &str,
    token: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().uri(path);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    energy_api::router(pool.clone())
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn token_auth_and_revocation_protect_http_routes() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL required for integration tests");
    let pool = MySqlPool::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();
    let issued = create_api_client(&pool, "http-test-client").await.unwrap();

    let unauthorized = request(&pool, "/v1/meters", None).await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let malformed = request(&pool, "/v1/meters", Some("wrong")).await;
    assert_eq!(malformed.status(), StatusCode::UNAUTHORIZED);

    let valid = request(&pool, "/v1/meters", Some(&issued.token)).await;
    assert_eq!(valid.status(), StatusCode::OK);
    let body = to_bytes(valid.into_body(), 1024 * 1024).await.unwrap();
    let meters: Value = serde_json::from_slice(&body).unwrap();
    assert!(meters.is_array());

    let missing = request(
        &pool,
        "/v1/meters/18446744073709551615/stream",
        Some(&issued.token),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    assert!(revoke_api_client(&pool, issued.client_id).await.unwrap());
    let revoked = request(&pool, "/v1/meters", Some(&issued.token)).await;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
}
