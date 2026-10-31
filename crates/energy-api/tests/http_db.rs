// SPDX-License-Identifier: MIT
//! Isolated MariaDB-backed HTTP authorization tests.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use energy_store::{
    create_acquisition_source, create_api_client, create_data_custodian,
    revoke_api_client,
};
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

/// Importing the same XML twice must expose stable canonical IDs over HTTP.
#[tokio::test]
async fn import_response_preserves_resource_ids_on_replay() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL required for integration tests");
    let pool = MySqlPool::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();

    let suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let custodian = create_data_custodian(
        &pool,
        &format!("http-import-custodian-{suffix}"),
    )
    .await
    .unwrap();
    let source = create_acquisition_source(
        &pool,
        custodian,
        &format!("http-import-source-{suffix}"),
    )
    .await
    .unwrap();
    let issued =
        create_api_client(&pool, &format!("http-import-client-{suffix}"))
            .await
            .unwrap();
    let path = format!("/v1/imports/green-button?source_id={}", source.0);
    let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom"
        xmlns:espi="http://naesb.org/espi">
      <entry>
        <link rel="self" href="usage-point-1" />
        <link rel="related" href="usage-point-1/MeterReading" />
        <content><espi:UsagePoint>
          <espi:ServiceCategory><espi:kind>0</espi:kind>
          </espi:ServiceCategory>
        </espi:UsagePoint></content>
      </entry>
      <entry>
        <link rel="self" href="reading-type-1" />
        <content><espi:ReadingType>
          <espi:commodity>1</espi:commodity>
          <espi:intervalLength>3600</espi:intervalLength>
          <espi:powerOfTenMultiplier>0</espi:powerOfTenMultiplier>
          <espi:uom>72</espi:uom>
        </espi:ReadingType></content>
      </entry>
      <entry>
        <link rel="self" href="meter-reading-1" />
        <link rel="up" href="usage-point-1/MeterReading" />
        <link rel="related" href="meter-reading-1/IntervalBlock" />
        <link rel="related" href="reading-type-1" />
        <content><espi:MeterReading /></content>
      </entry>
      <entry>
        <link rel="self" href="interval-block-1" />
        <link rel="up" href="meter-reading-1/IntervalBlock" />
        <content><espi:IntervalBlock>
          <espi:interval>
            <espi:duration>3600</espi:duration>
            <espi:start>1767254400</espi:start>
          </espi:interval>
          <espi:IntervalReading>
            <espi:timePeriod>
              <espi:duration>3600</espi:duration>
              <espi:start>1767254400</espi:start>
            </espi:timePeriod>
            <espi:value>11</espi:value>
          </espi:IntervalReading>
        </espi:IntervalBlock></content>
      </entry>
    </feed>"#;

    let post = |token: &str| {
        let request = Request::builder()
            .method("POST")
            .uri(&path)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/xml")
            .body(Body::from(xml))
            .unwrap();
        energy_api::router(pool.clone()).oneshot(request)
    };

    let first = post(&issued.token).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first: Value = serde_json::from_slice(
        &to_bytes(first.into_body(), 1024 * 1024).await.unwrap(),
    )
    .unwrap();
    let ids = first["resource_ids"].as_array().unwrap();
    assert_eq!(ids.len(), 1);
    assert!(ids[0].as_u64().unwrap() > 0);
    assert_eq!(first["accepted"], 1);

    let replay = post(&issued.token).await.unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    let replay: Value = serde_json::from_slice(
        &to_bytes(replay.into_body(), 1024 * 1024).await.unwrap(),
    )
    .unwrap();
    assert_eq!(replay["resource_ids"], first["resource_ids"]);
    assert_eq!(replay["observations"], 0);
    assert_eq!(replay["accepted"], 0);
    assert!(revoke_api_client(&pool, issued.client_id).await.unwrap());
}
