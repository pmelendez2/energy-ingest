// SPDX-License-Identifier: MIT
//! Authenticated HTTP window validation with isolated test MariaDB.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use energy_store::{create_api_client, revoke_api_client};
use serde_json::Value;
use sqlx::MySqlPool;
use tower::ServiceExt;

const MISSING_METER: u64 = u64::MAX;

async fn get(
    pool: &MySqlPool,
    path: &str,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = energy_api::router(pool.clone())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let payload = serde_json::from_slice(&body).unwrap();
    (status, payload)
}

#[tokio::test]
async fn authenticated_window_contract() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL required for integration tests");
    let pool = MySqlPool::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();
    let issued = create_api_client(&pool, "window-http-test").await.unwrap();
    let prefix = format!("/v1/meters/{MISSING_METER}/window");
    let valid =
        format!("{prefix}?start=2026-01-01T00:00:00Z&end=2026-01-02T00:00:00Z");

    let (status, body) = get(&pool, &valid, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");

    let (status, body) = get(&pool, &valid, Some("invalid-token")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");

    let invalid = [
        ("2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z"),
        ("2026-01-02T00:00:00Z", "2026-01-01T00:00:00Z"),
        ("2026-01-01T00:00:00", "2026-01-02T00:00:00Z"),
        ("2026-01-01T00:00:00+00:00", "2026-01-02T00:00:00Z"),
        ("not-a-date", "2026-01-02T00:00:00Z"),
        ("2026-01-01T00:00:00Z", "2026-02-02T00:00:00Z"),
    ];
    for (start, end) in invalid {
        let path = format!("{prefix}?start={start}&end={end}");
        let (status, body) = get(&pool, &path, Some(&issued.token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(body["error"], "invalid_window");
    }

    let (status, body) = get(&pool, &valid, Some(&issued.token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");

    assert!(revoke_api_client(&pool, issued.client_id).await.unwrap());
    let (status, body) = get(&pool, &valid, Some(&issued.token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "unauthorized");
}
