// SPDX-License-Identifier: MIT

//! MariaDB integration tests for physical-meter stream resolution.
//! Requires an isolated database through ENGY_T_DB_URL.

use energy_store::{
    StoreError, StreamIssueKind, resolve_physical_meter_stream,
};
use sqlx::{MySql, Pool};

#[tokio::test]
async fn associated_meter_revisions_and_conflicts() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL is required for integration tests");
    let pool = Pool::<MySql>::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();

    let label = format!(
        "usage-stream-db-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut fixture = Fixture::new(&pool, &label).await;

    let first = fixture.resource(1).await;
    let second = fixture.resource(1).await;
    let gas = fixture.resource(7).await;
    let electric_meter = fixture.meter(1).await;
    let gas_meter = fixture.meter(7).await;
    fixture.attach(electric_meter, first).await;
    fixture.attach(electric_meter, second).await;
    fixture.attach(gas_meter, gas).await;

    let empty =
        resolve_physical_meter_stream(&pool, electric_meter).await.unwrap();
    assert_eq!(empty.candidate_count, 0);
    assert!(empty.intervals.is_empty());

    fixture.interval(first, 0, 100).await;
    let b = fixture.interval(second, 0, 100).await;
    let _g = fixture.interval(gas, 0, 900).await;

    let selected =
        resolve_physical_meter_stream(&pool, electric_meter).await.unwrap();
    assert_eq!(selected.candidate_count, 2);
    assert_eq!(selected.intervals.len(), 1);
    assert!(selected.issues.is_empty());
    assert_eq!(selected.intervals[0].raw_value, 100);
    assert_eq!(selected.intervals[0].meter_reading_id, first);

    let gas_result =
        resolve_physical_meter_stream(&pool, gas_meter).await.unwrap();
    assert_eq!(gas_result.intervals.len(), 1);
    assert_eq!(gas_result.intervals[0].raw_value, 900);

    fixture.revise(b, 120).await;
    let conflict =
        resolve_physical_meter_stream(&pool, electric_meter).await.unwrap();
    assert!(conflict.intervals.is_empty());
    assert!(
        conflict.issues.iter().any(|issue| {
            issue.kind == StreamIssueKind::ConflictingDuplicate
        })
    );

    fixture.revise(b, 100).await;
    sqlx::query(
        "UPDATE interval_reading SET \
        accepted_revision_id = NULL WHERE id = ?",
    )
    .bind(b)
    .execute(&pool)
    .await
    .unwrap();
    let missing =
        resolve_physical_meter_stream(&pool, electric_meter).await.unwrap();
    assert!(missing.intervals.is_empty());
    assert!(missing.issues.iter().any(|issue| {
        issue.kind == StreamIssueKind::MissingAcceptedRevision
    }));
    fixture.revise(b, 100).await;

    sqlx::query(
        "UPDATE reading_type SET unit_of_measure = 38 \
        WHERE id = ?",
    )
    .bind(fixture.reading_types[1])
    .execute(&pool)
    .await
    .unwrap();
    let incompatible =
        resolve_physical_meter_stream(&pool, electric_meter).await.unwrap();
    assert!(incompatible.intervals.is_empty());
    assert!(incompatible.issues.iter().any(|issue| {
        issue.kind == StreamIssueKind::IncompatibleMeasurementType
    }));
    sqlx::query(
        "UPDATE reading_type SET unit_of_measure = 72 \
        WHERE id = ?",
    )
    .bind(fixture.reading_types[1])
    .execute(&pool)
    .await
    .unwrap();

    fixture.interval(second, 1800, 100).await;
    let overlap =
        resolve_physical_meter_stream(&pool, electric_meter).await.unwrap();
    assert!(overlap.intervals.is_empty());
    assert!(
        overlap.issues.iter().any(|issue| {
            issue.kind == StreamIssueKind::OverlappingIntervals
        })
    );

    let not_found =
        resolve_physical_meter_stream(&pool, u64::MAX).await.unwrap_err();
    assert!(matches!(not_found, StoreError::NotFound));

    // All synthetic rows are deleted in foreign-key order.
    fixture.cleanup().await;
}

struct Fixture<'a> {
    pool: &'a Pool<MySql>,
    custodian: u64,
    source: u64,
    run: u64,
    usage_points: Vec<u64>,
    reading_types: Vec<u64>,
    resources: Vec<u64>,
    meters: Vec<u64>,
    intervals: Vec<u64>,
}

impl<'a> Fixture<'a> {
    async fn new(pool: &'a Pool<MySql>, label: &str) -> Self {
        let custodian =
            insert(pool, "INSERT INTO data_custodian (name) VALUES (?)", label)
                .await;
        let source = insert_with_id(
            pool,
            "INSERT INTO acquisition_source (data_custodian_id, name) \
             VALUES (?, ?)",
            custodian,
            label,
        )
        .await;
        let run = sqlx::query(
            "INSERT INTO ingestion_run (acquisition_source_id, started_at) \
             VALUES (?, UTC_TIMESTAMP(6))",
        )
        .bind(source)
        .execute(pool)
        .await
        .unwrap()
        .last_insert_id();
        Self {
            pool,
            custodian,
            source,
            run,
            usage_points: vec![],
            reading_types: vec![],
            resources: vec![],
            meters: vec![],
            intervals: vec![],
        }
    }

    async fn resource(&mut self, commodity: u32) -> u64 {
        let n = self.resources.len();
        let usage = sqlx::query(
            "INSERT INTO usage_point \
             (acquisition_source_id, external_id) VALUES (?, ?)",
        )
        .bind(self.source)
        .bind(format!("usage-{n}"))
        .execute(self.pool)
        .await
        .unwrap()
        .last_insert_id();
        self.usage_points.push(usage);
        let rt = sqlx::query(
            "INSERT INTO reading_type \
             (acquisition_source_id, external_id, \
              accumulation_behaviour, commodity, data_qualifier, \
              flow_direction, kind, phase, power_of_ten_multiplier, \
              unit_of_measure) \
             VALUES (?, ?, 4, ?, 0, 1, 12, 0, 0, 72)",
        )
        .bind(self.source)
        .bind(format!("reading-type-{n}"))
        .bind(commodity)
        .execute(self.pool)
        .await
        .unwrap()
        .last_insert_id();
        self.reading_types.push(rt);
        let mr = sqlx::query(
            "INSERT INTO meter_reading \
             (usage_point_id, reading_type_id, external_id) \
             VALUES (?, ?, ?)",
        )
        .bind(usage)
        .bind(rt)
        .bind(format!("resource-{n}"))
        .execute(self.pool)
        .await
        .unwrap()
        .last_insert_id();
        self.resources.push(mr);
        mr
    }

    async fn meter(&mut self, commodity: u32) -> u64 {
        let id = sqlx::query(
            "INSERT INTO physical_meter \
             (data_custodian_id, display_name, commodity) \
             VALUES (?, ?, ?)",
        )
        .bind(self.custodian)
        .bind(format!("test-meter-{}", self.meters.len()))
        .bind(commodity)
        .execute(self.pool)
        .await
        .unwrap()
        .last_insert_id();
        self.meters.push(id);
        id
    }

    async fn attach(&self, meter: u64, resource: u64) {
        sqlx::query(
            "INSERT INTO physical_meter_resource \
             (physical_meter_id, meter_reading_id) VALUES (?, ?)",
        )
        .bind(meter)
        .bind(resource)
        .execute(self.pool)
        .await
        .unwrap();
    }

    async fn interval(
        &mut self,
        resource: u64,
        offset: u32,
        value: i64,
    ) -> u64 {
        let start = format!("2026-01-01 00:{:02}:00", offset / 60);
        let id = sqlx::query(
            "INSERT INTO interval_reading \
             (meter_reading_id, interval_start, \
              interval_duration_seconds) VALUES (?, ?, 3600)",
        )
        .bind(resource)
        .bind(start)
        .execute(self.pool)
        .await
        .unwrap()
        .last_insert_id();
        self.intervals.push(id);
        self.revise(id, value).await;
        id
    }

    async fn revise(&self, interval: u64, value: i64) {
        let revision = sqlx::query(
            "INSERT INTO interval_reading_revision \
             (interval_reading_id, ingestion_run_id, reading_value, \
              reading_quality) VALUES (?, ?, ?, 'reported')",
        )
        .bind(interval)
        .bind(self.run)
        .bind(value)
        .execute(self.pool)
        .await
        .unwrap()
        .last_insert_id();
        sqlx::query(
            "UPDATE interval_reading SET accepted_revision_id = ? \
             WHERE id = ?",
        )
        .bind(revision)
        .bind(interval)
        .execute(self.pool)
        .await
        .unwrap();
    }

    async fn cleanup(&self) {
        for &id in &self.intervals {
            sqlx::query(
                "UPDATE interval_reading SET \
                accepted_revision_id = NULL WHERE id = ?",
            )
            .bind(id)
            .execute(self.pool)
            .await
            .unwrap();
            sqlx::query(
                "DELETE FROM interval_reading_revision \
                WHERE interval_reading_id = ?",
            )
            .bind(id)
            .execute(self.pool)
            .await
            .unwrap();
            sqlx::query("DELETE FROM interval_reading WHERE id = ?")
                .bind(id)
                .execute(self.pool)
                .await
                .unwrap();
        }
        for &id in &self.resources {
            sqlx::query(
                "DELETE FROM physical_meter_resource \
                WHERE meter_reading_id = ?",
            )
            .bind(id)
            .execute(self.pool)
            .await
            .unwrap();
        }
        for &id in &self.meters {
            sqlx::query("DELETE FROM physical_meter WHERE id = ?")
                .bind(id)
                .execute(self.pool)
                .await
                .unwrap();
        }
        for &id in &self.resources {
            sqlx::query("DELETE FROM meter_reading WHERE id = ?")
                .bind(id)
                .execute(self.pool)
                .await
                .unwrap();
        }
        for &id in &self.reading_types {
            sqlx::query("DELETE FROM reading_type WHERE id = ?")
                .bind(id)
                .execute(self.pool)
                .await
                .unwrap();
        }
        for &id in &self.usage_points {
            sqlx::query("DELETE FROM usage_point WHERE id = ?")
                .bind(id)
                .execute(self.pool)
                .await
                .unwrap();
        }
        sqlx::query("DELETE FROM ingestion_run WHERE id = ?")
            .bind(self.run)
            .execute(self.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM acquisition_source WHERE id = ?")
            .bind(self.source)
            .execute(self.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM data_custodian WHERE id = ?")
            .bind(self.custodian)
            .execute(self.pool)
            .await
            .unwrap();
    }
}

async fn insert(pool: &Pool<MySql>, sql: &'static str, name: &str) -> u64 {
    sqlx::query(sql).bind(name).execute(pool).await.unwrap().last_insert_id()
}

async fn insert_with_id(
    pool: &Pool<MySql>,
    sql: &'static str,
    id: u64,
    name: &str,
) -> u64 {
    sqlx::query(sql)
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .unwrap()
        .last_insert_id()
}
