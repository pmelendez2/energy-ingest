// SPDX-License-Identifier: MIT

//! Administrative database bootstrap and provenance registration.
//!
//! Source registration is an explicit administrative operation. It is
//! separate from importing customer Green Button XML.

use energy_store::{
    DataCustodianId, create_acquisition_source, create_data_custodian,
    get_data_custodian, get_source_by_name, resolve_physical_meter_stream,
    summarize_meter_validation,
};
use sqlx::mysql::MySqlPoolOptions;
use std::{
    env,
    io::{self, Read},
    process::ExitCode,
};

const USAGE: &str = "\
Usage:
  energy-store-admin database bootstrap
  energy-store-admin custodian create NAME
  energy-store-admin source create CUSTODIAN_ID NAME
  energy-store-admin source list CUSTODIAN_ID
  energy-store-admin stream analyze [--details]
  energy-store-admin stream discover [--details]
  energy-store-admin meter create CUSTODIAN_ID COMMODITY NAME
  energy-store-admin meter create-auto COMMODITY NAME
  energy-store-admin meter set-number METER_ID --stdin
  energy-store-admin meter rename METER_ID NAME
  energy-store-admin meter list
  energy-store-admin meter attach METER_ID RESOURCE_ID
  energy-store-admin meter attach-compatible METER_ID [--apply]
  energy-store-admin meter resources METER_ID
  energy-store-admin meter validate METER_ID
  energy-store-admin --help

Environment:
  ENGY_DB_ADMIN_URL  Bootstrap only; privileged database URL
  ENGY_DB_URL        Registration commands; application database URL

Notes:
  Names must be nonempty and at most 255 characters.
  Verify the configured database before administrative operations.
  Registration does not require XML or customer account numbers.
";

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Bootstrap,
    CustodianCreate(String),
    SourceCreate(u64, String),
    SourceList(u64),
    StreamAnalyze(bool),
    MeterCreate(u64, u32, String),
    MeterCreateAuto(u32, String),
    MeterRename(u64, String),
    MeterSetNumber(u64),
    MeterList,
    MeterAttach(u64, u64),
    MeterAttachCompatible(u64, bool),
    MeterResources(u64),
    MeterValidate(u64),
}

fn parse_id(value: &str) -> Result<u64, String> {
    let id = value
        .parse::<u64>()
        .map_err(|_| "ID must be a positive decimal integer")?;
    if id == 0 {
        return Err("ID must be greater than zero".into());
    }
    Ok(id)
}

fn parse_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 255 {
        return Err("name must contain 1 to 255 characters".into());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("name cannot contain control characters".into());
    }
    Ok(trimmed.to_owned())
}

fn parse(args: &[String]) -> Result<Command, String> {
    match args {
        [flag] if flag == "--help" || flag == "-h" => Ok(Command::Help),
        [a, b] if a == "database" && b == "bootstrap" => Ok(Command::Bootstrap),
        [a, b, name] if a == "custodian" && b == "create" => {
            Ok(Command::CustodianCreate(parse_name(name)?))
        }
        [a, b, id, name] if a == "source" && b == "create" => {
            Ok(Command::SourceCreate(parse_id(id)?, parse_name(name)?))
        }
        [a, b] if a == "stream" && (b == "analyze" || b == "discover") => {
            Ok(Command::StreamAnalyze(false))
        }
        [a, b, flag]
            if a == "stream"
                && (b == "analyze" || b == "discover")
                && flag == "--details" =>
        {
            Ok(Command::StreamAnalyze(true))
        }
        [a, b, id, commodity, name] if a == "meter" && b == "create" => {
            let kind = commodity
                .parse::<u32>()
                .map_err(|_| "commodity must be numeric")?;
            if kind != 1 && kind != 7 {
                return Err("commodity must be 1 or 7".into());
            }
            Ok(Command::MeterCreate(parse_id(id)?, kind, parse_name(name)?))
        }
        [a, b, commodity, name] if a == "meter" && b == "create-auto" => {
            let kind = commodity
                .parse::<u32>()
                .map_err(|_| "commodity must be numeric")?;
            if kind != 1 && kind != 7 {
                return Err("commodity must be 1 or 7".into());
            }
            Ok(Command::MeterCreateAuto(kind, parse_name(name)?))
        }
        [a, b, id, flag]
            if a == "meter" && b == "set-number" && flag == "--stdin" =>
        {
            Ok(Command::MeterSetNumber(parse_id(id)?))
        }
        [a, b, id, name] if a == "meter" && b == "rename" => {
            Ok(Command::MeterRename(parse_id(id)?, parse_name(name)?))
        }
        [a, b] if a == "meter" && b == "list" => Ok(Command::MeterList),
        [a, b, meter, resource] if a == "meter" && b == "attach" => {
            Ok(Command::MeterAttach(parse_id(meter)?, parse_id(resource)?))
        }
        [a, b, id] if a == "meter" && b == "attach-compatible" => {
            Ok(Command::MeterAttachCompatible(parse_id(id)?, false))
        }
        [a, b, id, flag]
            if a == "meter"
                && b == "attach-compatible"
                && flag == "--apply" =>
        {
            Ok(Command::MeterAttachCompatible(parse_id(id)?, true))
        }
        [a, b, id] if a == "meter" && b == "validate" => {
            Ok(Command::MeterValidate(parse_id(id)?))
        }
        [a, b, id] if a == "meter" && b == "resources" => {
            Ok(Command::MeterResources(parse_id(id)?))
        }
        [a, b, id] if a == "source" && b == "list" => {
            Ok(Command::SourceList(parse_id(id)?))
        }
        _ => Err("invalid command; see --help".into()),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    match execute(&args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn execute(args: &[String]) -> Result<(), String> {
    let command = parse(args)?;
    if command == Command::Help {
        print!("{USAGE}");
        return Ok(());
    }
    if command == Command::Bootstrap {
        let url = env::var("ENGY_DB_ADMIN_URL")
            .map_err(|_| "ENGY_DB_ADMIN_URL is required")?;
        let result = energy_store::bootstrap_database(&url)
            .await
            .map_err(|error| error.to_string())?;
        println!(
            "Database bootstrap complete: {} tables verified.",
            result.verified_tables
        );
        return Ok(());
    }

    let url = env::var("ENGY_DB_URL").map_err(|_| "ENGY_DB_URL is required")?;
    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .map_err(|_| "failed to connect to application database")?;
    match command {
        Command::CustodianCreate(name) => {
            let id = create_data_custodian(&pool, &name)
                .await
                .map_err(|e| e.to_string())?;
            println!("Custodian created: id={}", id.0);
        }
        Command::SourceCreate(custodian_id, name) => {
            let custodian = DataCustodianId(custodian_id);
            if get_data_custodian(&pool, custodian)
                .await
                .map_err(|e| e.to_string())?
                .is_none()
            {
                return Err("custodian ID does not exist".into());
            }
            if let Some(existing) = get_source_by_name(&pool, custodian, &name)
                .await
                .map_err(|e| e.to_string())?
            {
                println!("Source already exists: id={}", existing.id);
            } else {
                let id = create_acquisition_source(&pool, custodian, &name)
                    .await
                    .map_err(|e| e.to_string())?;
                println!("Source created: id={}", id.0);
            }
        }
        Command::SourceList(custodian_id) => {
            if get_data_custodian(&pool, DataCustodianId(custodian_id))
                .await
                .map_err(|e| e.to_string())?
                .is_none()
            {
                return Err("custodian ID does not exist".into());
            }
            // Query IDs and names only; no customer data or secrets.
            let records: Vec<(u64, String)> = sqlx::query_as(
                "SELECT id, name FROM acquisition_source \
                 WHERE data_custodian_id = ? ORDER BY id",
            )
            .bind(custodian_id)
            .fetch_all(&pool)
            .await
            .map_err(|_| "source listing failed")?;
            println!("SOURCE_ID  NAME");
            for (id, name) in records {
                println!("{id:<10} {name}");
            }
        }
        Command::MeterCreate(custodian, commodity, name) => {
            let exists: Option<(u64,)> =
                sqlx::query_as("SELECT id FROM data_custodian WHERE id = ?")
                    .bind(custodian)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|_| "custodian lookup failed")?;
            if exists.is_none() {
                return Err("custodian does not exist".into());
            }
            let result = sqlx::query(
                "INSERT INTO physical_meter \
                 (data_custodian_id, display_name, commodity) \
                 VALUES (?, ?, ?)",
            )
            .bind(custodian)
            .bind(&name)
            .bind(commodity)
            .execute(&pool)
            .await
            .map_err(|_| "meter creation failed; check name")?;
            println!(
                "Meter profile created: {} ({})",
                name,
                result.last_insert_id()
            );
        }
        Command::MeterCreateAuto(commodity, name) => {
            // No implicit selection is safe with multiple utilities.
            let custodians: Vec<(u64,)> = sqlx::query_as(
                "SELECT id FROM data_custodian ORDER BY id LIMIT 2",
            )
            .fetch_all(&pool)
            .await
            .map_err(|_| "utility lookup failed")?;
            let custodian = match custodians.as_slice() {
                [(id,)] => *id,
                [] => {
                    return Err(
                        "no utility registered; register one first".into()
                    );
                }
                _ => {
                    return Err(
                        "multiple utilities registered; use --custodian".into(),
                    );
                }
            };
            let result = sqlx::query(
                "INSERT INTO physical_meter \
                 (data_custodian_id, display_name, commodity) \
                 VALUES (?, ?, ?)",
            )
            .bind(custodian)
            .bind(&name)
            .bind(commodity)
            .execute(&pool)
            .await
            .map_err(|_| "meter creation failed; check name")?;
            println!(
                "Meter profile created: {} ({})",
                name,
                result.last_insert_id()
            );
        }
        Command::MeterSetNumber(meter) => {
            // Read privately via stdin, never from process arguments.
            // The value is not included in success/error output.
            let mut bytes = Vec::new();
            io::stdin()
                .take(1025)
                .read_to_end(&mut bytes)
                .map_err(|_| "cannot read meter number from stdin")?;
            if bytes.len() > 1024 {
                return Err("meter number exceeds input limit".into());
            }
            let number = String::from_utf8(bytes)
                .map_err(|_| "meter number is not UTF-8")?;
            let number = number.trim_end_matches(['\r', '\n']);
            let number = parse_name(number)
                .map_err(|_| "invalid utility meter number")?;
            let result = sqlx::query(
                "UPDATE physical_meter SET meter_number = ? WHERE id = ?",
            )
            .bind(&number)
            .bind(meter)
            .execute(&pool)
            .await
            .map_err(|_| "meter number update failed")?;
            if result.rows_affected() == 0 {
                return Err("meter profile not found".into());
            }
            println!("Utility meter number saved.");
        }
        Command::MeterRename(meter, name) => {
            let result = sqlx::query(
                "UPDATE physical_meter SET display_name = ? \
                 WHERE id = ?",
            )
            .bind(&name)
            .bind(meter)
            .execute(&pool)
            .await
            .map_err(|_| "meter rename failed; name may exist")?;
            if result.rows_affected() == 0 {
                return Err("meter profile not found".into());
            }
            println!("Meter profile renamed: {}", name);
        }
        Command::MeterList => {
            let rows: Vec<(u64, String, u32, i64)> = sqlx::query_as(
                "SELECT p.id, p.display_name, p.commodity, \
                     COUNT(a.meter_reading_id) \
                     FROM physical_meter p \
                     LEFT JOIN physical_meter_resource a \
                     ON a.physical_meter_id = p.id \
                     GROUP BY p.id, p.display_name, p.commodity \
                     ORDER BY p.id",
            )
            .fetch_all(&pool)
            .await
            .map_err(|_| "meter listing failed")?;
            if rows.is_empty() {
                println!("No meters configured.");
                println!("Use meter-prod.sh create to add a meter.");
                return Ok(());
            }
            println!("ID  Name                    Commodity  Resources");
            for (id, name, commodity, count) in rows {
                let kind = commodity_label(Some(commodity));
                println!("{id:<3} {name:<23} {kind:<10} {count}");
            }
        }
        Command::MeterAttach(meter, resource) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|_| "database transaction failed")?;
            let profile: Option<(u64, u32)> = sqlx::query_as(
                "SELECT data_custodian_id, commodity \
                 FROM physical_meter WHERE id = ? FOR UPDATE",
            )
            .bind(meter)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| "meter lookup failed")?;
            let (custodian, kind) = profile.ok_or("meter profile not found")?;
            let source: Option<(u64, Option<u32>)> = sqlx::query_as(
                "SELECT dc.id, rt.commodity \
                     FROM meter_reading mr \
                     JOIN usage_point up ON up.id = mr.usage_point_id \
                     JOIN acquisition_source src \
                       ON src.id = up.acquisition_source_id \
                     JOIN data_custodian dc \
                       ON dc.id = src.data_custodian_id \
                     JOIN reading_type rt \
                       ON rt.id = mr.reading_type_id \
                     WHERE mr.id = ?",
            )
            .bind(resource)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| "resource lookup failed")?;
            let (resource_custodian, resource_kind) =
                source.ok_or("resource not found")?;
            if custodian != resource_custodian || resource_kind != Some(kind) {
                return Err("utility or commodity mismatch".into());
            }
            let current: Option<(u64,)> = sqlx::query_as(
                "SELECT physical_meter_id \
                 FROM physical_meter_resource \
                 WHERE meter_reading_id = ?",
            )
            .bind(resource)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| "association lookup failed")?;
            match current {
                Some((id,)) if id == meter => {
                    println!("Resource already associated.");
                }
                Some(_) => {
                    return Err(
                        "resource belongs to another meter profile".into()
                    );
                }
                None => {
                    sqlx::query(
                        "INSERT INTO physical_meter_resource \
                         (physical_meter_id, meter_reading_id) \
                         VALUES (?, ?)",
                    )
                    .bind(meter)
                    .bind(resource)
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| "association failed")?;
                    println!("Resource associated with meter profile.");
                }
            }
            tx.commit().await.map_err(|_| "association commit failed")?;
        }
        Command::MeterAttachCompatible(meter, apply) => {
            attach_compatible(&pool, meter, apply).await?;
        }
        Command::MeterValidate(meter) => {
            let result = resolve_physical_meter_stream(&pool, meter)
                .await
                .map_err(|_| "meter validation failed")?;
            let summary = summarize_meter_validation(meter, &result);
            print!("{}", format_meter_validation(&summary));
        }
        Command::MeterResources(meter) => {
            let ids: Vec<(u64,)> = sqlx::query_as(
                "SELECT meter_reading_id FROM physical_meter_resource \
                 WHERE physical_meter_id = ? \
                 ORDER BY meter_reading_id",
            )
            .bind(meter)
            .fetch_all(&pool)
            .await
            .map_err(|_| "resource listing failed")?;
            println!("Associated resource IDs (diagnostics):");
            for (id,) in ids {
                println!("{id}");
            }
        }
        Command::StreamAnalyze(details) => {
            analyze_streams(&pool, details).await?
        }
        Command::Help | Command::Bootstrap => unreachable!(),
    }
    Ok(())
}

/// Presentation only; validation decisions come from energy-store.
fn format_meter_validation(summary: &energy_store::MeterValidation) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    writeln!(text, "Physical meter ID: {}", summary.physical_meter_id).unwrap();
    writeln!(text, "Status: {}", summary.status.as_str()).unwrap();
    writeln!(text, "Candidates: {}", summary.candidate_count).unwrap();
    writeln!(text, "Selected intervals: {}", summary.selected_interval_count)
        .unwrap();
    writeln!(text, "Content fingerprint: {}", summary.content_fingerprint)
        .unwrap();
    for issue in &summary.issues {
        writeln!(
            text,
            "Issue: {} interval={} other={:?}",
            issue.kind.as_str(),
            issue.interval_reading_id,
            issue.other_interval_reading_id
        )
        .unwrap();
    }
    text
}

/// Preview or atomically associate unambiguous, compatible resources.
///
/// This operation never infers that matching readings identify a physical
/// meter. It is available only when the utility has a single profile for
/// the chosen commodity. Multiple profiles require explicit associations.
async fn attach_compatible(
    pool: &sqlx::MySqlPool,
    meter: u64,
    apply: bool,
) -> Result<(), String> {
    let mut tx =
        pool.begin().await.map_err(|_| "database transaction failed")?;
    let profile: Option<(u64, u32, String)> = sqlx::query_as(
        "SELECT data_custodian_id, commodity, display_name \
         FROM physical_meter WHERE id = ? FOR UPDATE",
    )
    .bind(meter)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| "meter lookup failed")?;
    let (custodian, kind, name) = profile.ok_or("meter profile not found")?;

    let profiles: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM physical_meter \
         WHERE data_custodian_id = ? AND commodity = ?",
    )
    .bind(custodian)
    .bind(kind)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "meter profile count failed")?;
    if profiles.0 != 1 {
        return Err("multiple meters share this utility and commodity; \
             associate resources individually"
            .into());
    }

    // Resource associations are never inferred from interval values.
    // Restrict candidates to the same utility and commodity, then reject
    // assignments already owned by a different physical meter profile.
    let resources: Vec<(u64, Option<u64>)> = sqlx::query_as(
        "SELECT mr.id, pmr.physical_meter_id \
         FROM meter_reading mr \
         JOIN usage_point up ON up.id = mr.usage_point_id \
         JOIN acquisition_source src \
           ON src.id = up.acquisition_source_id \
         JOIN reading_type rt ON rt.id = mr.reading_type_id \
         LEFT JOIN physical_meter_resource pmr \
           ON pmr.meter_reading_id = mr.id \
         WHERE src.data_custodian_id = ? AND rt.commodity = ? \
         ORDER BY mr.id FOR UPDATE",
    )
    .bind(custodian)
    .bind(kind)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| "compatible-resource query failed")?;

    let mut pending = Vec::new();
    let mut existing = 0_usize;
    for (id, owner) in resources {
        match owner {
            None => pending.push(id),
            Some(owner) if owner == meter => existing += 1,
            Some(_) => {
                return Err("a compatible resource belongs to another meter; \
                 no associations were changed"
                    .into());
            }
        }
    }
    println!("Meter profile: {name}");
    println!("Already associated: {existing}");
    println!("Eligible to associate: {}", pending.len());
    if !pending.is_empty() {
        println!("Imported file data (provider metadata):");
        for id in &pending {
            let detail: (
                u64,
                String,
                Option<u32>,
                Option<String>,
                Option<String>,
            ) = sqlx::query_as(
                "SELECT up.id, src.name, rt.unit_of_measure, \
                 CAST(MIN(ib.interval_start) AS CHAR), \
                 CAST(MAX(DATE_ADD(ib.interval_start, \
                   INTERVAL ib.interval_duration_seconds SECOND)) AS CHAR) \
                 FROM meter_reading mr \
                 JOIN usage_point up ON up.id = mr.usage_point_id \
                 JOIN acquisition_source src \
                   ON src.id = up.acquisition_source_id \
                 JOIN reading_type rt ON rt.id = mr.reading_type_id \
                 LEFT JOIN interval_block ib ON ib.meter_reading_id = mr.id \
                 WHERE mr.id = ? \
                 GROUP BY up.id, src.name, rt.unit_of_measure",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "resource preview lookup failed")?;
            let (usage_point, source, unit, start, end) = detail;
            println!(
                "  File data entry {id}: source={source}; \
                usage point={usage_point}; unit code={unit:?}"
            );
            println!(
                "    Observed block coverage (UTC): {} to {}",
                start.as_deref().unwrap_or("not supplied"),
                end.as_deref().unwrap_or("not supplied")
            );
        }
        println!("Compatibility is not proof of physical ownership.");
    }
    if !apply {
        println!("Preview only. No associations were changed.");
        return Ok(());
    }

    for resource in &pending {
        sqlx::query(
            "INSERT INTO physical_meter_resource \
             (physical_meter_id, meter_reading_id) VALUES (?, ?)",
        )
        .bind(meter)
        .bind(resource)
        .execute(&mut *tx)
        .await
        .map_err(|_| "resource association failed")?;
    }
    tx.commit().await.map_err(|_| "association commit failed")?;
    println!("Associated {} file data entries.", pending.len());
    Ok(())
}

/// Analyze imported resources without changing source observations.
///
/// Agreement on overlapping values indicates possible equivalence,
/// not a confirmed physical-meter association.
async fn analyze_streams(
    pool: &sqlx::MySqlPool,
    details: bool,
) -> Result<(), String> {
    type ResourceRow = (
        u64,
        u64,
        Option<u32>,
        Option<u32>,
        Option<i16>,
        i64,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    );
    let resources: Vec<ResourceRow> = sqlx::query_as(
        "SELECT mr.id, up.acquisition_source_id, rt.commodity, \
         rt.unit_of_measure, rt.power_of_ten_multiplier, \
         COUNT(ir.id), \
         CAST(MIN(ir.interval_duration_seconds) AS SIGNED), \
         CAST(MAX(ir.interval_duration_seconds) AS SIGNED), \
         DATE_FORMAT(MIN(ir.interval_start), '%Y-%m-%d'), \
         DATE_FORMAT(MAX(DATE_ADD(ir.interval_start, \
           INTERVAL ir.interval_duration_seconds SECOND)), \
           '%Y-%m-%d') \
         FROM meter_reading mr \
         JOIN usage_point up ON up.id = mr.usage_point_id \
         JOIN reading_type rt ON rt.id = mr.reading_type_id \
         LEFT JOIN interval_reading ir \
           ON ir.meter_reading_id = mr.id \
         GROUP BY mr.id, up.acquisition_source_id, \
           rt.commodity, rt.unit_of_measure, \
           rt.power_of_ten_multiplier \
         ORDER BY rt.commodity, mr.id",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| format!("resource analysis query failed: {error}"))?;

    // Compare only identical source and measurement characteristics.
    // No association is inferred merely from matching readings.
    type PairRow = (u64, u64, i64, i64);
    let pairs: Vec<PairRow> = sqlx::query_as(
        "SELECT a.meter_reading_id, b.meter_reading_id, \
         COUNT(*), \
         CAST(SUM(CASE \
           WHEN va.reading_value <> vb.reading_value \
           THEN 1 ELSE 0 END) AS SIGNED) \
         FROM interval_reading a \
         JOIN interval_reading b \
           ON a.meter_reading_id < b.meter_reading_id \
          AND a.interval_start = b.interval_start \
          AND a.interval_duration_seconds = b.interval_duration_seconds \
         JOIN meter_reading ma ON ma.id = a.meter_reading_id \
         JOIN meter_reading mb ON mb.id = b.meter_reading_id \
         JOIN usage_point ua ON ua.id = ma.usage_point_id \
         JOIN usage_point ub ON ub.id = mb.usage_point_id \
         JOIN reading_type ta ON ta.id = ma.reading_type_id \
         JOIN reading_type tb ON tb.id = mb.reading_type_id \
         JOIN interval_reading_revision va \
           ON va.id = a.accepted_revision_id \
         JOIN interval_reading_revision vb \
           ON vb.id = b.accepted_revision_id \
         WHERE ua.acquisition_source_id = ub.acquisition_source_id \
           AND ta.commodity <=> tb.commodity \
           AND ta.unit_of_measure <=> tb.unit_of_measure \
           AND ta.power_of_ten_multiplier <=> \
               tb.power_of_ten_multiplier \
           AND ta.kind <=> tb.kind \
           AND ta.flow_direction <=> tb.flow_direction \
         GROUP BY a.meter_reading_id, b.meter_reading_id \
         ORDER BY a.meter_reading_id, b.meter_reading_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| format!("overlap analysis query failed: {error}"))?;

    println!("Energy Usage — Imported Data Analysis");
    println!("Imported resources: {}", resources.len());
    if resources.is_empty() {
        println!("No usage data has been imported.");
        println!("No database records were changed.");
        return Ok(());
    }

    // Group by commodity only for display. The grouping is NOT a meter
    // association, particularly for customers with multiple meters.
    let mut commodities: Vec<Option<u32>> = Vec::new();
    for row in &resources {
        if !commodities.contains(&row.2) {
            commodities.push(row.2);
        }
    }
    for commodity in commodities {
        let group: Vec<&ResourceRow> =
            resources.iter().filter(|row| row.2 == commodity).collect();
        let ids: Vec<u64> = group.iter().map(|row| row.0).collect();
        let relevant: Vec<&PairRow> = pairs
            .iter()
            .filter(|pair| ids.contains(&pair.0) && ids.contains(&pair.1))
            .collect();
        let conflicts: i64 = relevant.iter().map(|row| row.3).sum();
        let from = group
            .iter()
            .filter_map(|row| row.8.as_deref())
            .min()
            .unwrap_or("unknown");
        let to = group
            .iter()
            .filter_map(|row| row.9.as_deref())
            .max()
            .unwrap_or("unknown");
        let range: Vec<i64> = group
            .iter()
            .flat_map(|row| [row.6, row.7].into_iter().flatten())
            .collect();
        let granularity = if range.iter().all(|v| *v == 3600) {
            "Hourly"
        } else if range.iter().all(|v| *v == 86400) {
            "Daily"
        } else {
            "Mixed or unknown"
        };
        let label = commodity_label(commodity);
        println!("\n{label}");
        println!("  Imported resources: {}", group.len());
        println!("  Interval resolution: {granularity}");
        println!("  Coverage (UTC): {from} through {to}");
        println!("  Overlapping resource pairs: {}", relevant.len());
        if conflicts == 0 {
            println!("  Overlap results: No differing values found");
        } else {
            println!("  Overlap results: {conflicts} differing values");
        }
        println!("  Physical meter association: Not assessed");
    }
    println!("\nMatching values do not prove a shared physical meter.");
    println!("No data or physical meter associations were changed.");

    if details {
        println!("\nTechnical resource details");
        println!("ID Source Code Unit Count Min(s) Max(s)");
        for row in &resources {
            println!(
                "{:<3}{:>7}{:>5}{:>5}{:>6}{:>7}{:>7}",
                row.0,
                row.1,
                display_option(row.2),
                display_option(row.3),
                row.5,
                display_option(row.6),
                display_option(row.7)
            );
        }
        println!("\nOverlapping resource pairs: {}", pairs.len());
        println!("ID A  ID B   Overlap   Different");
        for (a, b, overlap, different) in &pairs {
            println!("{a:>4} {b:>5} {overlap:>9} {different:>11}");
        }
        println!("IDs above identify database resources, not meters.");

        // Unlike the pairwise counts above, these counts represent unique
        // conflicting intervals within an explicitly associated meter.
        // This is diagnostic evidence only: no value is chosen as a winner.
        let conflict_days: Vec<(u64, String, i64)> = sqlx::query_as(
            "SELECT conflicts.physical_meter_id, conflicts.day, COUNT(*) \
             FROM ( \
               SELECT DISTINCT pa.physical_meter_id, \
                 DATE_FORMAT(a.interval_start, '%Y-%m-%d') AS day, \
                 a.interval_start, a.interval_duration_seconds \
               FROM interval_reading a \
               JOIN interval_reading b \
                 ON a.meter_reading_id < b.meter_reading_id \
                AND a.interval_start = b.interval_start \
                AND a.interval_duration_seconds = \
                    b.interval_duration_seconds \
               JOIN physical_meter_resource pa \
                 ON pa.meter_reading_id = a.meter_reading_id \
               JOIN physical_meter_resource pb \
                 ON pb.meter_reading_id = b.meter_reading_id \
                AND pb.physical_meter_id = pa.physical_meter_id \
               JOIN interval_reading_revision va \
                 ON va.id = a.accepted_revision_id \
               JOIN interval_reading_revision vb \
                 ON vb.id = b.accepted_revision_id \
               WHERE va.reading_value <> vb.reading_value \
             ) AS conflicts \
             GROUP BY conflicts.physical_meter_id, conflicts.day \
             ORDER BY conflicts.physical_meter_id, conflicts.day",
        )
        .fetch_all(pool)
        .await
        .map_err(|error| {
            format!("associated conflict-date query failed: {error}")
        })?;
        print!("{}", format_conflict_days(&conflict_days));
    }
    Ok(())
}

/// Render only dates and counts; raw utility readings remain private.
fn format_conflict_days(rows: &[(u64, String, i64)]) -> String {
    use std::fmt::Write;

    let mut output = String::from(
        "\nUnique differing intervals by associated meter and UTC date\n",
    );
    if rows.is_empty() {
        output
            .push_str("No differing values among associated resource pairs.\n");
    } else {
        output.push_str("Meter ID  UTC date     Unique intervals\n");
        for (meter, day, count) in rows {
            writeln!(output, "{meter:>8}  {day}  {count:>16}")
                .expect("writing into String is infallible");
        }
    }
    output.push_str("Differences are unresolved; no values were replaced.\n");
    output
}

fn commodity_label(code: Option<u32>) -> String {
    match code {
        Some(1) => "Electricity".to_owned(),
        Some(7) => "Natural gas".to_owned(),
        Some(other) => format!("Other commodity (code {other})"),
        None => "Unspecified commodity".to_owned(),
    }
}

fn display_option<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "-".to_owned(), |v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::{Command, format_conflict_days, parse};

    #[test]
    fn conflict_report_counts_unique_hours_and_omits_values() {
        let report = format_conflict_days(&[
            (1, "2026-10-06".to_owned(), 24),
            (2, "2026-10-26".to_owned(), 1),
        ]);
        assert!(report.contains("2026-10-06"));
        assert!(report.contains("2026-10-26"));
        assert!(report.contains("      24"));
        assert!(report.contains("unresolved"));
        assert!(!report.contains("reading_value"));
    }

    #[test]
    fn no_conflicts_reported_without_candidate_pairs() {
        let report = format_conflict_days(&[]);
        assert!(report.contains("No differing values"));
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    #[test]
    fn canonical_meter_validation_command() {
        assert_eq!(
            parse(&args(&["meter", "validate", "5"])),
            Ok(Command::MeterValidate(5))
        );
        assert!(parse(&args(&["meter", "validate", "0"])).is_err());
        let summary = energy_store::summarize_meter_validation(
            5,
            &energy_store::StreamResolution::default(),
        );
        let text = super::format_meter_validation(&summary);
        assert!(text.contains("Content fingerprint: "));
        assert!(text.contains("Status: no_data"));
        assert!(text.contains("Physical meter ID: 5"));
    }

    #[test]
    fn source_creation_requires_valid_id_and_name() {
        assert_eq!(
            parse(&args(&["source", "create", "42", "SDGE download"])),
            Ok(Command::SourceCreate(42, "SDGE download".into()))
        );
        for value in ["0", "-1", "abc", "1x"] {
            assert!(
                parse(&args(&["source", "create", value, "Download"])).is_err()
            );
        }
        assert!(parse(&args(&["source", "create", "1", " "])).is_err());
    }

    #[test]
    fn meter_commands_validate_commodity_and_ids() {
        assert_eq!(
            parse(&args(&["meter", "create", "1", "1", "Home Electricity"])),
            Ok(Command::MeterCreate(1, 1, "Home Electricity".into()))
        );
        assert!(
            parse(&args(&["meter", "create", "1", "99", "Unknown"])).is_err()
        );
        assert_eq!(
            parse(&args(&["meter", "attach", "1", "2"])),
            Ok(Command::MeterAttach(1, 2))
        );
    }

    #[test]
    fn meter_profile_names_and_auto_selection_parse() {
        assert_eq!(
            parse(&args(&["meter", "create-auto", "1", "Home Electric"])),
            Ok(Command::MeterCreateAuto(1, "Home Electric".into()))
        );
        assert_eq!(
            parse(&args(&["meter", "rename", "2", "Gas"])),
            Ok(Command::MeterRename(2, "Gas".into()))
        );
        assert!(parse(&args(&["meter", "create-auto", "99", "Bad"])).is_err());
    }

    #[test]
    fn meter_number_requires_private_stdin_option() {
        assert_eq!(
            parse(&args(&["meter", "set-number", "2", "--stdin"])),
            Ok(Command::MeterSetNumber(2))
        );
        assert!(parse(&args(&["meter", "set-number", "2", "123"])).is_err());
        assert!(
            parse(&args(&["meter", "set-number", "0", "--stdin"])).is_err()
        );
    }

    #[test]
    fn bulk_association_requires_explicit_apply() {
        assert_eq!(
            parse(&args(&["meter", "attach-compatible", "2"])),
            Ok(Command::MeterAttachCompatible(2, false))
        );
        assert_eq!(
            parse(&args(&["meter", "attach-compatible", "2", "--apply"])),
            Ok(Command::MeterAttachCompatible(2, true))
        );
        assert!(parse(&args(&["meter", "attach-compatible", "0"])).is_err());
        assert!(
            parse(&args(&["meter", "attach-compatible", "2", "--force"]))
                .is_err()
        );
    }

    #[test]
    fn commodity_names_are_readable() {
        assert_eq!(super::commodity_label(Some(1)), "Electricity");
        assert_eq!(super::commodity_label(Some(7)), "Natural gas");
        assert_eq!(super::commodity_label(None), "Unspecified commodity");
    }

    #[test]
    fn listing_and_bootstrap_are_explicit() {
        assert_eq!(
            parse(&args(&["source", "list", "1"])),
            Ok(Command::SourceList(1))
        );
        assert_eq!(
            parse(&args(&["database", "bootstrap"])),
            Ok(Command::Bootstrap)
        );
        assert!(parse(&args(&["source", "list"])).is_err());
        assert_eq!(
            parse(&args(&["stream", "discover"])),
            Ok(Command::StreamAnalyze(false))
        );
        assert_eq!(
            parse(&args(&["stream", "analyze"])),
            Ok(Command::StreamAnalyze(false))
        );
        assert_eq!(
            parse(&args(&["stream", "analyze", "--details"])),
            Ok(Command::StreamAnalyze(true))
        );
    }
}
