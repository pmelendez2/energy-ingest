// SPDX-License-Identifier: MIT
//! Database-backed HTTP regression for resource association state.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use energy_store::{
    AssociationOutcome, associate_meter_resource, create_acquisition_source,
    create_api_client, create_data_custodian,
};
use serde_json::Value;
use sqlx::MySqlPool;
use tower::ServiceExt;

async fn lookup(
    pool: &MySqlPool,
    token: &str,
    resource: u64,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .uri(format!("/v1/resources/{resource}/association"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response =
        energy_api::router(pool.clone()).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn lookup_reflects_canonical_database_association() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("isolated ENGY_T_DB_URL is required");
    let pool = MySqlPool::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();
    let label = format!(
        "lookup-state-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let custodian = create_data_custodian(&pool, &label).await.unwrap();
    let source =
        create_acquisition_source(&pool, custodian, &format!("source-{label}"))
            .await
            .unwrap();
    let usage_point = sqlx::query(
        "INSERT INTO usage_point \
         (acquisition_source_id, external_id) VALUES (?, ?)",
    )
    .bind(source.0)
    .bind(format!("usage-{label}"))
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let reading_type = sqlx::query(
        "INSERT INTO reading_type \
         (acquisition_source_id, external_id, commodity) \
         VALUES (?, ?, ?)",
    )
    .bind(source.0)
    .bind(format!("type-{label}"))
    .bind(1_u32)
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let resource = sqlx::query(
        "INSERT INTO meter_reading \
         (usage_point_id, reading_type_id, external_id) \
         VALUES (?, ?, ?)",
    )
    .bind(usage_point)
    .bind(reading_type)
    .bind(format!("reading-{label}"))
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let meter = sqlx::query(
        "INSERT INTO physical_meter \
         (data_custodian_id, display_name, commodity) \
         VALUES (?, ?, ?)",
    )
    .bind(custodian.0)
    .bind(format!("meter-{label}"))
    .bind(1_u32)
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let client =
        create_api_client(&pool, &format!("client-{label}")).await.unwrap();

    let (status, body) = lookup(&pool, &client.token, resource).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["resource_id"], resource);
    assert_eq!(body["commodity"], 1);
    assert_eq!(body["status"], "unassociated");
    assert!(body["physical_meter_id"].is_null());

    assert_eq!(
        associate_meter_resource(&pool, meter, resource).await.unwrap(),
        AssociationOutcome::Associated
    );
    let (status, body) = lookup(&pool, &client.token, resource).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["resource_id"], resource);
    assert_eq!(body["commodity"], 1);
    assert_eq!(body["status"], "associated");
    assert_eq!(body["physical_meter_id"], meter);

    assert_eq!(
        associate_meter_resource(&pool, meter, resource).await.unwrap(),
        AssociationOutcome::AlreadyAssociated
    );
    let (_, replay) = lookup(&pool, &client.token, resource).await;
    assert_eq!(replay, body);
}
