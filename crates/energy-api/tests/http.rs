// SPDX-License-Identifier: MIT

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::mysql::MySqlPoolOptions;
use tower::ServiceExt;

fn app() -> axum::Router {
    let pool = MySqlPoolOptions::new()
        .connect_lazy("mysql://user:pass@localhost/test")
        .expect("valid synthetic URL");
    energy_api::router(pool)
}

#[tokio::test]
async fn health_does_not_require_database_connection() {
    let response = app()
        .oneshot(
            Request::builder().uri("/healthz").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn protected_routes_reject_missing_bearer() {
    for path in ["/v1/meters", "/v1/meters/1/stream"] {
        let response = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn unsupported_route_is_not_found() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/v1/admin/tokens")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn openapi_matches_public_routes_and_bearer_contract() {
    let api = serde_json::to_value(energy_api::openapi()).unwrap();
    assert!(api["openapi"].as_str().unwrap().starts_with("3."));
    let paths = &api["paths"];
    for path in ["/healthz", "/v1/meters", "/v1/meters/{id}/stream"] {
        assert!(paths[path]["get"].is_object(), "missing {path}");
    }
    assert!(paths["/healthz"]["get"]["security"].is_null());
    for path in ["/v1/meters", "/v1/meters/{id}/stream"] {
        assert_eq!(
            paths[path]["get"]["security"][0]["bearer_auth"],
            serde_json::json!([]),
        );
    }
    assert_eq!(
        api["components"]["securitySchemes"]["bearer_auth"]["scheme"],
        "bearer",
    );
}

#[tokio::test]
async fn developer_documentation_is_opt_in() {
    let plain = app()
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(plain.status(), StatusCode::NOT_FOUND);

    let pool = MySqlPoolOptions::new()
        .connect_lazy("mysql://user:pass@localhost/test")
        .unwrap();
    let docs = energy_api::developer_router(pool)
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(docs.status(), StatusCode::OK);
}
