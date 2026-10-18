// SPDX-License-Identifier: MIT

//! Private utility-meter-number registration through the admin CLI.
//! Requires ENGY_T_DB_URL and an isolated test MariaDB database.

use sqlx::{MySql, Pool, Row};
use std::io::Write;
use std::process::{Command, Stdio};

#[tokio::test]
async fn meter_number_stdin_persists_without_echo() {
    let url =
        std::env::var("ENGY_T_DB_URL").expect("ENGY_T_DB_URL is required");
    let pool = Pool::<MySql>::connect(&url).await.unwrap();
    energy_store::migrate(&pool).await.unwrap();

    let unique = format!(
        "meter-number-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let custodian = sqlx::query("INSERT INTO data_custodian (name) VALUES (?)")
        .bind(&unique)
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_id();
    let meter = sqlx::query(
        "INSERT INTO physical_meter \
         (data_custodian_id, display_name, commodity) \
         VALUES (?, ?, 1)",
    )
    .bind(custodian)
    .bind("Electric")
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();

    let number = "UTILITY-XY-0294";
    let mut child = Command::new(env!("CARGO_BIN_EXE_energy-store-admin"))
        .args(["meter", "set-number", &meter.to_string(), "--stdin"])
        .env("ENGY_DB_URL", &url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{number}\n").as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();

    let saved: Option<String> =
        sqlx::query("SELECT meter_number FROM physical_meter WHERE id = ?")
            .bind(meter)
            .fetch_one(&pool)
            .await
            .unwrap()
            .try_get("meter_number")
            .unwrap();

    sqlx::query("DELETE FROM physical_meter WHERE id = ?")
        .bind(meter)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM data_custodian WHERE id = ?")
        .bind(custodian)
        .execute(&pool)
        .await
        .unwrap();

    assert!(result.status.success());
    assert_eq!(saved.as_deref(), Some(number));
    let output = [result.stdout, result.stderr].concat();
    assert!(!String::from_utf8_lossy(&output).contains(number));
}
