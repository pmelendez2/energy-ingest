// SPDX-License-Identifier: MIT
//! Synthetic two-channel NEM database regression; isolated MariaDB required.
use energy_store::{
    create_acquisition_source, create_data_custodian, create_ingestion_run,
    import_gb_document_atomic,
};
use sqlx::MySqlPool;
use std::path::PathBuf;

#[tokio::test]
async fn duplicate_nem_usage_point_imports_both_flows_idempotently() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("isolated ENGY_T_DB_URL is required");
    let pool = MySqlPool::connect(&url).await.unwrap();
    let suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let custodian =
        create_data_custodian(&pool, &format!("nem-custodian-{suffix}"))
            .await
            .unwrap();
    let source = create_acquisition_source(
        &pool,
        custodian,
        &format!("nem-source-{suffix}"),
    )
    .await
    .unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../profiles/sdge/fixtures/")
        .join("sdge-nem-duplicate-usage-point-synthetic.xml");
    let doc = gb::parse_file(path).unwrap();

    let run = create_ingestion_run(&pool, source).await.unwrap();
    let first = import_gb_document_atomic(&pool, source, run, &doc)
        .await
        .expect("repeated identical UsagePoint must be accepted");
    assert_eq!(first.observations, 4);
    assert_eq!(first.accepted, 4);
    assert_eq!(first.resource_ids.len(), 2);

    let channel_data: Vec<(String, Option<u32>, Vec<i64>)> = {
        // Check the source document and persisted reading values separately.
        // SQL below independently confirms flow directions and cardinality.
        doc.meter_readings
            .iter()
            .enumerate()
            .map(|(i, m)| {
                (
                    m.external_id.clone(),
                    doc.reading_types[i].flow_direction,
                    doc.interval_blocks[i]
                        .readings
                        .iter()
                        .map(|r| r.value)
                        .collect(),
                )
            })
            .collect()
    };
    assert_eq!(channel_data[0].2, vec![1500, 1750]);
    assert_eq!(channel_data[1].2, vec![400, 600]);
    assert_eq!(channel_data[0].1, Some(1));
    assert_eq!(channel_data[1].1, Some(19));

    let rows: Vec<(Option<u32>, i64)> = sqlx::query_as(
        "SELECT rt.flow_direction, COUNT(*) \
         FROM interval_reading i \
         JOIN meter_reading m ON m.id = i.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         JOIN reading_type rt ON rt.id = m.reading_type_id \
         WHERE u.acquisition_source_id = ? \
         GROUP BY rt.flow_direction ORDER BY rt.flow_direction",
    )
    .bind(source.0)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows, vec![(Some(1), 2), (Some(19), 2)]);

    let run = create_ingestion_run(&pool, source).await.unwrap();
    let replay = import_gb_document_atomic(&pool, source, run, &doc)
        .await
        .expect("reimport must be idempotent");
    assert_eq!(replay.observations, 4);
    assert_eq!(replay.accepted, 0);
    assert_eq!(replay.unchanged, 4);
    assert_eq!(replay.resource_ids, first.resource_ids);
}
