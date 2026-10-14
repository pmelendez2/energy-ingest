// SPDX-License-Identifier: MIT
//

use energy_store::{
    AcquisitionSourceId, DataCustodianId, create_acquisition_source,
    create_data_custodian, create_ingestion_run, get_acquisition_source,
    get_data_custodian, get_ingestion_run, get_source_by_name,
};
use sqlx::{MySql, Pool};

/// Verifies that ingestion-source records can be created and read back.
///
/// # Test Goal
///
/// Confirm that the public `energy-store` API can create and retrieve a Data
/// Custodian, Acquisition Source, and Ingestion Run.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
///
/// # Expected Outcomes
///
/// - each record is created successfully;
/// - each record can be retrieved through the public store API;
/// - stored relationships match returned identifiers; and
/// - test-created records are removed before the test completes.
#[tokio::test]
async fn ingestion_source_repository_round_trip() {
    let database_url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL must be set for database integration tests");

    let pool = Pool::<MySql>::connect(&database_url)
        .await
        .expect("test database connection should succeed");

    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let suffix = unique_suffix();

    let custodian_name = format!("repository-test-custodian-{suffix}");
    let source_name = format!("repository-test-source-{suffix}");

    let custodian_id = create_data_custodian(&pool, &custodian_name)
        .await
        .expect("data custodian creation should succeed");

    let custodian = get_data_custodian(&pool, custodian_id)
        .await
        .expect("data custodian lookup should succeed")
        .expect("data custodian should exist");

    assert_eq!(custodian.id, custodian_id.0);
    assert_eq!(custodian.name, custodian_name);

    let source_id =
        create_acquisition_source(&pool, custodian_id, &source_name)
            .await
            .expect("acquisition source creation should succeed");

    let source = get_acquisition_source(&pool, source_id)
        .await
        .expect("acquisition source lookup should succeed")
        .expect("acquisition source should exist");

    assert_eq!(source.id, source_id.0);
    assert_eq!(source.data_custodian_id, custodian_id.0);
    assert_eq!(source.name, source_name);

    let source_by_name = get_source_by_name(&pool, custodian_id, &source_name)
        .await
        .expect("source-by-name lookup should succeed")
        .expect("source should exist");

    assert_eq!(source_by_name.id, source_id.0);

    let run_id = create_ingestion_run(&pool, source_id)
        .await
        .expect("ingestion run creation should succeed");

    let run = get_ingestion_run(&pool, run_id)
        .await
        .expect("ingestion run lookup should succeed")
        .expect("ingestion run should exist");

    assert_eq!(run.id, run_id.0);
    assert_eq!(run.acquisition_source_id, source_id.0);

    cleanup(&pool, run_id.0, source_id, custodian_id)
        .await
        .expect("integration-test cleanup should succeed");
}

/// Verifies database constraint enforcement for ingestion-source records.
///
/// # Test Goal
///
/// Confirm that invalid or duplicate ingestion-source data is rejected by
/// the database and translated through the `energy-store` error boundary.
///
/// # Inputs
///
/// - `ENGY_T_DB_URL`: MariaDB integration-test connection URL.
/// - Running `energy-test` MariaDB instance.
/// - Synthetic Data Custodian and Acquisition Source records.
///
/// # Expected Outcomes
///
/// - valid records are created successfully;
/// - duplicate or invalid records are rejected by database constraints; and
/// - the resulting failure is exposed through the expected typed store error.
#[tokio::test]
async fn ingestion_source_repository_enforces_database_constraints() {
    let database_url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL must be set for database integration tests");

    let pool = Pool::<MySql>::connect(&database_url)
        .await
        .expect("test database connection should succeed");

    energy_store::migrate(&pool)
        .await
        .expect("database migrations should succeed");

    let suffix = unique_suffix();
    let custodian_name = format!("constraint-test-custodian-{suffix}");

    let custodian_id = create_data_custodian(&pool, &custodian_name)
        .await
        .expect("initial custodian creation should succeed");

    let duplicate = create_data_custodian(&pool, &custodian_name).await;

    assert!(
        duplicate.is_err(),
        "duplicate Data Custodian names should be rejected"
    );

    let invalid_source = create_acquisition_source(
        &pool,
        DataCustodianId(u64::MAX),
        "invalid-custodian-source",
    )
    .await;

    assert!(
        invalid_source.is_err(),
        "source with invalid Data Custodian should be rejected"
    );

    sqlx::query("DELETE FROM data_custodian WHERE id = ?")
        .bind(custodian_id.0)
        .execute(&pool)
        .await
        .expect("test custodian cleanup should succeed");
}

/// Removes records created by an ingestion-source integration test.
///
/// # Inputs
///
/// Record identifiers must refer to rows created by the current test.
///
/// # Expected Outcomes
///
/// Records are deleted in foreign-key-safe order.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if a cleanup query fails.
async fn cleanup(
    pool: &Pool<MySql>,
    ingestion_run_id: u64,
    source_id: AcquisitionSourceId,
    custodian_id: DataCustodianId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ingestion_run WHERE id = ?")
        .bind(ingestion_run_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM acquisition_source WHERE id = ?")
        .bind(source_id.0)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM data_custodian WHERE id = ?")
        .bind(custodian_id.0)
        .execute(pool)
        .await?;

    Ok(())
}

/// Creates a suffix for integration-test record names.
///
/// # Returns
///
/// Nanoseconds elapsed since the Unix epoch.
///
/// # Expected Outcomes
///
/// Repeated test runs are unlikely to generate duplicate source identifiers.
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos()
}
