// SPDX-License-Identifier: MIT

//! Isolated MariaDB tests for bounded physical-meter windows.
//! Uses only synthetic test records, cleaned up after each test.

use energy_store::{StreamIssueKind, resolve_meter_window};
use sqlx::{MySql, Pool, types::chrono::NaiveDateTime};

fn instant(hour: u32, minute: u32) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(
        &format!("2026-01-01 {hour:02}:{minute:02}:00"),
        "%Y-%m-%d %H:%M:%S",
    )
    .unwrap()
}

#[tokio::test]
async fn window_boundaries_duplicates_and_revisions() {
    let url = std::env::var("ENGY_T_DB_URL")
        .expect("ENGY_T_DB_URL required for isolated MariaDB tests");
    let pool = Pool::<MySql>::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();
    let label = format!("window-db-{}", std::process::id());
    let mut fixture = Fixture::new(&pool, &label).await;

    let primary = fixture.resource(1).await;
    let mirror = fixture.resource(1).await;
    let gas_resource = fixture.resource(7).await;
    let meter = fixture.meter(1).await;
    let gas_meter = fixture.meter(7).await;
    fixture.attach(meter, primary).await;
    fixture.attach(meter, mirror).await;
    fixture.attach(gas_meter, gas_resource).await;

    let one = fixture.interval(primary, 0, 100).await;
    let duplicate = fixture.interval(mirror, 0, 100).await;
    fixture.interval(primary, 3600, 200).await;
    fixture.interval(gas_resource, 0, 900).await;

    // Start inclusive; end exclusive; empty windows are valid.
    let first =
        resolve_meter_window(&pool, meter, instant(0, 0), instant(1, 0))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(first.candidate_count, 2);
    assert_eq!(first.intervals.len(), 1);
    assert_eq!(first.intervals[0].raw_value, 100);
    assert_eq!(first.intervals[0].meter_reading_id, primary);

    let second =
        resolve_meter_window(&pool, meter, instant(1, 0), instant(2, 0))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(second.candidate_count, 1);
    assert_eq!(second.intervals[0].raw_value, 200);

    let empty =
        resolve_meter_window(&pool, meter, instant(2, 0), instant(3, 0))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(empty.candidate_count, 0);
    assert!(empty.intervals.is_empty());

    let gas =
        resolve_meter_window(&pool, gas_meter, instant(0, 0), instant(1, 0))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(gas.intervals[0].raw_value, 900);
    assert_eq!(gas.candidate_count, 1);

    // Updating the accepted revision changes the selected value, but
    // conflicting overlapping resources must never emit partial usage.
    fixture.revise(duplicate, 101).await;
    let conflict =
        resolve_meter_window(&pool, meter, instant(0, 0), instant(2, 0))
            .await
            .unwrap()
            .unwrap();
    assert_eq!(conflict.candidate_count, 3);
    assert!(conflict.intervals.is_empty());
    assert!(
        conflict
            .issues
            .iter()
            .any(|i| { i.kind == StreamIssueKind::ConflictingDuplicate })
    );

    fixture.revise(duplicate, 100).await;
    fixture.revise(one, 100).await;
    let repaired =
        resolve_meter_window(&pool, meter, instant(0, 0), instant(2, 0))
            .await
            .unwrap()
            .unwrap();
    assert!(repaired.issues.is_empty());
    assert_eq!(repaired.intervals.len(), 2);

    sqlx::query(
        "UPDATE interval_reading SET accepted_revision_id = NULL \
         WHERE id = ?",
    )
    .bind(one)
    .execute(&pool)
    .await
    .unwrap();
    let missing =
        resolve_meter_window(&pool, meter, instant(0, 0), instant(1, 0))
            .await
            .unwrap()
            .unwrap();
    assert!(missing.intervals.is_empty());
    assert!(
        missing
            .issues
            .iter()
            .any(|i| { i.kind == StreamIssueKind::MissingAcceptedRevision })
    );
    fixture.revise(one, 100).await;

    let absent =
        resolve_meter_window(&pool, u64::MAX, instant(0, 0), instant(1, 0))
            .await
            .unwrap();
    assert!(absent.is_none());
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
        let midnight = sqlx::types::chrono::NaiveDateTime::parse_from_str(
            "2026-01-01 00:00:00",
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();
        let start = midnight + chrono::Duration::seconds(i64::from(offset));
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
