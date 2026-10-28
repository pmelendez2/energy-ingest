// SPDX-License-Identifier: MIT

//! Database-backed regression coverage for atomic Green Button imports.
//! Requires an isolated database through `ENGY_T_DB_URL`.

use energy_store::{
    create_acquisition_source, create_data_custodian, create_ingestion_run,
    import_gb_document_atomic, import_gb_document_atomic_with_hash,
};
use gb::{
    GbDocument, IntervalBlockRecord, IntervalReadingRecord, MeterReadingRecord,
    ReadingTypeRecord, UsagePointRecord,
};
use sqlx::MySqlPool;

fn document(block_name: &str, value: i64) -> GbDocument {
    GbDocument {
        usage_points: vec![UsagePointRecord {
            external_id: "sample-usage".into(),
            related_hrefs: vec!["usage-link".into()],
            service_category_kind: Some(0),
        }],
        reading_types: vec![ReadingTypeRecord {
            external_id: "sample-type".into(),
            accumulation_behaviour: None,
            commodity: None,
            data_qualifier: None,
            flow_direction: None,
            kind: None,
            phase: None,
            power_of_ten_multiplier: None,
            unit_of_measure: None,
            interval_length_seconds: Some(3600),
        }],
        meter_readings: vec![MeterReadingRecord {
            external_id: "sample-meter".into(),
            up_href: Some("usage-link".into()),
            related_hrefs: vec!["sample-type".into(), "block-link".into()],
        }],
        interval_blocks: vec![IntervalBlockRecord {
            external_id: block_name.into(),
            up_href: "block-link".into(),
            start_unix: 1_700_000_000,
            duration_seconds: 3600,
            published_unix: None,
            updated_unix: None,
            readings: vec![IntervalReadingRecord {
                start_unix: 1_700_000_000,
                duration_seconds: 3600,
                value,
                quality: None,
            }],
        }],
    }
}

#[tokio::test]
async fn exact_document_replay_preserves_first_successful_import() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("isolated ENGY_T_DB_URL is required");
    let pool = MySqlPool::connect(&url).await.unwrap();
    let suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    let custodian = create_data_custodian(
        &pool,
        &format!("replay-test-custodian-{suffix}"),
    )
    .await
    .unwrap();
    let source = create_acquisition_source(
        &pool,
        custodian,
        &format!("replay-test-source-{suffix}"),
    )
    .await
    .unwrap();
    let first = create_ingestion_run(&pool, source).await.unwrap();
    let hash = "a".repeat(64);
    let input = document("replay-block", 11);
    let report = import_gb_document_atomic_with_hash(
        &pool,
        source,
        first,
        &input,
        Some(&hash),
    )
    .await
    .unwrap();
    assert_eq!(report.accepted, 1);

    let second = create_ingestion_run(&pool, source).await.unwrap();
    let replay = import_gb_document_atomic_with_hash(
        &pool,
        source,
        second,
        &input,
        Some(&hash),
    )
    .await
    .unwrap();
    assert_eq!(replay.observations, 0);
    assert_eq!(replay.accepted, 0);
    let retained: u64 = sqlx::query_scalar(
        "SELECT first_ingestion_run_id FROM import_document \
         WHERE acquisition_source_id = ? AND sha256_hex = ?",
    )
    .bind(source.0)
    .bind(&hash)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(retained, first.0);
}

#[tokio::test]
async fn atomic_import_rolls_back_and_preserves_revisions() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("isolated ENGY_T_DB_URL is required");
    let pool = MySqlPool::connect(&url).await.unwrap();
    let suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    let custodian = create_data_custodian(
        &pool,
        &format!("atomic-test-custodian-{suffix}"),
    )
    .await
    .unwrap();
    let source = create_acquisition_source(
        &pool,
        custodian,
        &format!("atomic-test-source-{suffix}"),
    )
    .await
    .unwrap();

    let run = create_ingestion_run(&pool, source).await.unwrap();
    let initial = document("block-a", 0);
    let report =
        import_gb_document_atomic(&pool, source, run, &initial).await.unwrap();
    assert_eq!((report.accepted, report.unchanged), (1, 0));

    let run = create_ingestion_run(&pool, source).await.unwrap();
    let report =
        import_gb_document_atomic(&pool, source, run, &initial).await.unwrap();
    assert_eq!((report.accepted, report.unchanged), (0, 1));

    let run = create_ingestion_run(&pool, source).await.unwrap();
    let corrected = document("block-a", 7);
    let report = import_gb_document_atomic(&pool, source, run, &corrected)
        .await
        .unwrap();
    assert_eq!((report.accepted, report.unchanged), (1, 0));

    // A very long block identifier violates VARCHAR(1024). This occurs
    // after the parent inserts and proves those writes roll back.
    let run = create_ingestion_run(&pool, source).await.unwrap();
    let failed = document(&"x".repeat(5000), 9);
    assert!(
        import_gb_document_atomic(&pool, source, run, &failed,).await.is_err()
    );

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM interval_reading_revision r \
         JOIN interval_reading i ON i.id = r.interval_reading_id \
         JOIN meter_reading m ON m.id = i.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         WHERE u.acquisition_source_id = ?",
    )
    .bind(source.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 2, "failed import must not add a revision");

    let block_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM interval_block b \
         JOIN meter_reading m ON m.id = b.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         WHERE u.acquisition_source_id = ?",
    )
    .bind(source.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(block_count, 1);

    // Clean up this test's isolated acquisition-source hierarchy.
    // Tests use a dedicated database; cascades are not assumed here.
}
