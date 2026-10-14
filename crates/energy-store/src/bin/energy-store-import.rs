// SPDX-License-Identifier: MIT

//! Import Green Button XML files into the configured Energy Usage database.
//!
//! Imports one XML file per MariaDB transaction. A failed file rolls back.

use std::{env, path::PathBuf, process::ExitCode};

use energy_store::{
    AcquisitionSourceId, GbImportReport, create_ingestion_run,
    import_gb_document_atomic,
};
use gb::{GbDocument, parse_file};
use sqlx::MySqlPool;

const USAGE: &str = "\
Usage:
  energy-store-import --source-id ID FILE.xml [FILE.xml ...]
  energy-store-import --help

Environment:
  ENGY_DB_URL       MariaDB connection URL (required for imports)

Safety:
  Each file is imported as a MariaDB transaction. Files are committed
  separately; an error in a later file does not roll back earlier files.
  The command does not echo the database connection URL.
";

/// Fully parsed arguments. Reject unsafe invocations before opening a file.
#[derive(Debug, PartialEq, Eq)]
struct Args {
    source_id: u64,
    files: Vec<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<Option<Args>, String> {
    if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help") {
        return Ok(None);
    }
    let mut source_id = None;

    let mut files = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--source-id" => {
                index += 1;
                let value = args.get(index).ok_or(
                    "--source-id requires a positive integer".to_owned(),
                )?;
                let id = value.parse::<u64>().map_err(|_| {
                    "--source-id requires a positive integer".to_owned()
                })?;
                if id == 0 || source_id.replace(id).is_some() {
                    return Err("source ID must be nonzero and unique".into());
                }
            }
            value if value.starts_with('-') => {
                return Err(format!("unrecognized option: {value}"));
            }
            value => files.push(PathBuf::from(value)),
        }
        index += 1;
    }
    let source_id = source_id.ok_or("--source-id is required")?;
    if files.is_empty() {
        return Err("at least one XML filename is required".into());
    }
    if files.iter().any(|path| path.extension().is_none_or(|x| x != "xml")) {
        return Err("all input filenames must end in .xml".into());
    }
    Ok(Some(Args { source_id, files }))
}

// Keep the report at or below 80 display columns.
const FILE_WIDTH: usize = 35;

fn short_name(file: &std::path::Path) -> String {
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= FILE_WIDTH {
        return name.into_owned();
    }
    // Keep the suffix recognizable without affecting the actual path.
    let suffix = if name.ends_with(".xml") { ".xml" } else { "" };
    let keep = FILE_WIDTH - suffix.chars().count() - 3;
    let start: String = chars.iter().take(keep).collect();
    format!("{start}...{suffix}")
}

fn print_row(file: &std::path::Path, report: GbImportReport) {
    let display = short_name(file);
    println!(
        "{display:<35} {observed:>10} {unchanged:>10} {accepted:>10}",
        observed = report.observations,
        unchanged = report.unchanged,
        accepted = report.accepted,
    );
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Parses every source document before opening the database connection.
async fn run(args: &[String]) -> Result<(), String> {
    let Some(args) = parse_args(args)? else {
        print!("{USAGE}");
        return Ok(());
    };
    let mut documents: Vec<(PathBuf, GbDocument)> = Vec::new();
    for file in &args.files {
        let doc = parse_file(file).map_err(|_| {
            format!("GB XML parsing failed: {}", file.display())
        })?;
        documents.push((file.clone(), doc));
    }
    let url = env::var("ENGY_DB_URL")
        .map_err(|_| "ENGY_DB_URL is required".to_owned())?;
    let pool = MySqlPool::connect(&url).await.map_err(|_| {
        "could not connect to MariaDB using ENGY_DB_URL".to_owned()
    })?;

    println!(
        "{:<35} {:>10} {:>10} {:>10}",
        "File", "Observed", "Unchanged", "Accepted"
    );
    println!("{}", "-".repeat(68));
    let mut total = GbImportReport::default();
    for (path, document) in documents {
        // Run IDs are created only after all XML inputs have parsed.
        let run_id =
            create_ingestion_run(&pool, AcquisitionSourceId(args.source_id))
                .await
                .map_err(|_| "could not create ingestion run".to_owned())?;
        let report = import_gb_document_atomic(
            &pool,
            AcquisitionSourceId(args.source_id),
            run_id,
            &document,
        )
        .await
        .map_err(|_| {
            format!(
                "import failed for {} (file transaction rolled back)",
                path.display()
            )
        })?;
        print_row(&path, report);
        total.observations += report.observations;
        total.unchanged += report.unchanged;
        total.accepted += report.accepted;
    }
    println!("{}", "-".repeat(68));
    print_row(std::path::Path::new("TOTAL"), total);
    println!("Completed. Accepted includes first and revised observations.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_filename_stays_short_and_preserves_suffix() {
        let file = std::path::Path::new(
            "Electric_Daily_9-7-2025_10-6-2026_20261007.xml",
        );
        let name = short_name(file);
        assert!(name.chars().count() <= FILE_WIDTH);
        assert!(name.ends_with(".xml"));
    }

    #[test]
    fn source_id_is_required() {
        let args = vec!["--source-id".into(), "1".into(), "sample.xml".into()];
        assert!(parse_args(&args).is_ok());
        assert!(parse_args(&["sample.xml".into()]).is_err());
    }

    #[test]
    fn valid_arguments_are_parsed_without_database() {
        let args = vec!["--source-id".into(), "9".into(), "test.xml".into()];
        let parsed = parse_args(&args).unwrap().unwrap();
        assert_eq!(parsed.source_id, 9);
        assert_eq!(parsed.files, vec![PathBuf::from("test.xml")]);
    }

    #[test]
    fn unknown_options_and_missing_files_are_rejected() {
        assert!(parse_args(&["--bad".into()]).is_err());
        assert!(
            parse_args(&["--source-id".into(), "1".into(), "--unknown".into()])
                .is_err()
        );
    }
}
