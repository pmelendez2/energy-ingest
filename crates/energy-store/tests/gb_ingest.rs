// SPDX-License-Identifier: MIT

//! Golden-fixture regression tests for transactional GB ingestion.
//!
//! Requires `ENGY_T_DB_URL` pointing to an isolated MariaDB test database.
//! Repository CRUD behavior is tested in the dedicated store test modules.

use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use energy_store::{
    AcquisitionSourceId, create_acquisition_source, create_data_custodian,
    create_ingestion_run, import_gb_document_atomic,
};
use gb::{GbDocument, parse_file};
use serde::Deserialize;
use sqlx::{MySql, MySqlPool, Transaction};

#[derive(Debug, Deserialize)]
struct FixtureManifest {
    fixture: Vec<ExpectedFixture>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFixture {
    file: String,
    reading_count: usize,
    interval_seconds: u32,
    #[serde(default)]
    require_zero: bool,
}

#[tokio::test]
async fn golden_fixtures_ingest_idempotently() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("isolated ENGY_T_DB_URL is required");
    let pool = MySqlPool::connect(&url)
        .await
        .expect("test database connection should succeed");
    energy_store::migrate(&pool)
        .await
        .expect("test database migrations should succeed");

    for (directory, expected) in all_fixtures() {
        check_fixture(&pool, &directory, &expected).await;
    }
}

async fn check_fixture(
    pool: &MySqlPool,
    directory: &Path,
    expected: &ExpectedFixture,
) {
    let started = Instant::now();
    let path = directory.join("fixtures").join(&expected.file);
    let document = parse_file(&path)
        .unwrap_or_else(|e| panic!("fixture XML should parse: {e}"));
    verify_fixture(&document, expected);
    let parsed_at = started.elapsed();

    let suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos(),
    );
    let custodian =
        create_data_custodian(pool, &format!("gb-fixture-custodian-{suffix}"))
            .await
            .expect("create test custodian");
    let source = create_acquisition_source(
        pool,
        custodian,
        &format!("gb-fixture-source-{suffix}"),
    )
    .await
    .expect("create test acquisition source");
    let prepared_at = started.elapsed();

    let first_run = create_ingestion_run(pool, source)
        .await
        .expect("create first test run");
    let first = import_gb_document_atomic(pool, source, first_run, &document)
        .await
        .expect("initial atomic import");
    assert_eq!(first.observations, expected.reading_count);
    assert_eq!(first.accepted, expected.reading_count);
    assert_eq!(first.unchanged, 0);
    let first_at = started.elapsed();

    let second_run = create_ingestion_run(pool, source)
        .await
        .expect("create second test run");
    let second = import_gb_document_atomic(pool, source, second_run, &document)
        .await
        .expect("unchanged atomic reimport");
    assert_eq!(second.observations, expected.reading_count);
    assert_eq!(second.accepted, 0);
    assert_eq!(second.unchanged, expected.reading_count);
    let second_at = started.elapsed();

    let mut corrected = document.clone();
    corrected.interval_blocks[0].readings[0].value += 1;
    let correction_run = create_ingestion_run(pool, source)
        .await
        .expect("create correction test run");
    let correction =
        import_gb_document_atomic(pool, source, correction_run, &corrected)
            .await
            .expect("corrected atomic import");
    assert_eq!(correction.observations, expected.reading_count);
    assert_eq!(correction.accepted, 1);
    assert_eq!(correction.unchanged, expected.reading_count - 1);
    let corrected_at = started.elapsed();

    // Aggregate checks exercise the persisted state without an SQL query
    // per fixture reading. A correction adds only one historical revision.
    let (intervals, revisions): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(DISTINCT i.id), COUNT(r.id) \
         FROM interval_reading i \
         JOIN meter_reading m ON m.id = i.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         LEFT JOIN interval_reading_revision r \
           ON r.interval_reading_id = i.id \
         WHERE u.acquisition_source_id = ?",
    )
    .bind(source.0)
    .fetch_one(pool)
    .await
    .expect("read imported interval counts");
    assert_eq!(intervals as usize, expected.reading_count);
    assert_eq!(revisions as usize, expected.reading_count + 1);

    let (accepted_count, corrected_count): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), \
         COUNT(CASE WHEN r.ingestion_run_id = ? THEN 1 END) \
         FROM interval_reading i \
         JOIN meter_reading m ON m.id = i.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         JOIN interval_reading_revision r \
           ON r.id = i.accepted_revision_id \
         WHERE u.acquisition_source_id = ?",
    )
    .bind(correction_run.0)
    .bind(source.0)
    .fetch_one(pool)
    .await
    .expect("read accepted revision counts");
    assert_eq!(accepted_count as usize, expected.reading_count);
    assert_eq!(corrected_count, 1);
    let verified_at = started.elapsed();

    delete_fixture(pool, source, custodian.0)
        .await
        .expect("clean up source-scoped fixture records");
    if std::env::var_os("ENGY_T_PROFILE").is_some() {
        eprintln!(
            "fixture={} readings={} parse={:?} setup={:?} \
             atomic_first={:?} atomic_same={:?} correction={:?} \
             verification={:?} cleanup={:?}",
            expected.file,
            expected.reading_count,
            parsed_at,
            prepared_at.saturating_sub(parsed_at),
            first_at.saturating_sub(prepared_at),
            second_at.saturating_sub(first_at),
            corrected_at.saturating_sub(second_at),
            verified_at.saturating_sub(corrected_at),
            started.elapsed().saturating_sub(verified_at),
        );
    }
}

fn verify_fixture(document: &GbDocument, expected: &ExpectedFixture) {
    assert!(!document.meter_readings.is_empty());
    assert!(!document.interval_blocks.is_empty());
    let readings: Vec<_> = document
        .interval_blocks
        .iter()
        .flat_map(|block| block.readings.iter())
        .collect();
    assert_eq!(readings.len(), expected.reading_count);
    assert!(readings.iter().all(|reading| {
        reading.duration_seconds == expected.interval_seconds
    }));
    if expected.require_zero {
        assert!(readings.iter().any(|reading| reading.value == 0));
    }
}

/// Delete only the records owned by this disposable source. The order
/// respects the accepted-revision and parent foreign-key dependencies.
/// All cleanup statements are committed together; other sources are intact.
async fn delete_fixture(
    pool: &MySqlPool,
    source: AcquisitionSourceId,
    custodian_id: u64,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    clear_acceptance(&mut tx, source).await?;
    let statements = [
        "DELETE r FROM interval_reading_revision r \
         JOIN interval_reading i ON i.id = r.interval_reading_id \
         JOIN meter_reading m ON m.id = i.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         WHERE u.acquisition_source_id = ?",
        "DELETE i FROM interval_reading i \
         JOIN meter_reading m ON m.id = i.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         WHERE u.acquisition_source_id = ?",
        "DELETE b FROM interval_block b \
         JOIN meter_reading m ON m.id = b.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         WHERE u.acquisition_source_id = ?",
        "DELETE m FROM meter_reading m \
         JOIN usage_point u ON u.id = m.usage_point_id \
         WHERE u.acquisition_source_id = ?",
        "DELETE FROM reading_type WHERE acquisition_source_id = ?",
        "DELETE FROM usage_point WHERE acquisition_source_id = ?",
        "DELETE FROM ingestion_run WHERE acquisition_source_id = ?",
        "DELETE FROM acquisition_source WHERE id = ?",
    ];
    for statement in statements {
        sqlx::query(statement).bind(source.0).execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM data_custodian WHERE id = ?")
        .bind(custodian_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

async fn clear_acceptance(
    tx: &mut Transaction<'_, MySql>,
    source: AcquisitionSourceId,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE interval_reading i \
         JOIN meter_reading m ON m.id = i.meter_reading_id \
         JOIN usage_point u ON u.id = m.usage_point_id \
         SET i.accepted_revision_id = NULL \
         WHERE u.acquisition_source_id = ?",
    )
    .bind(source.0)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn all_fixtures() -> Vec<(PathBuf, ExpectedFixture)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let profiles = gb::profile::discover(&root)
        .expect("provider profile discovery must succeed");
    assert!(!profiles.is_empty(), "no provider profiles found");
    let mut result = Vec::new();
    for profile in profiles {
        let path = profile.directory.join("profile.toml");
        let content = fs::read_to_string(&path)
            .expect("profile manifest should be readable");
        let manifest: FixtureManifest = toml::from_str(&content)
            .expect("fixture expectations should parse");
        assert!(!manifest.fixture.is_empty(), "fixture list is empty");
        for expected in manifest.fixture {
            result.push((profile.directory.clone(), expected));
        }
    }
    result
}
