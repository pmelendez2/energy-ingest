// SPDX-License-Identifier: MIT
//! Local CLI for preparing regression fixtures under ADR-0028.
//! Source XML is never changed and preparation never grants approval.

use fixture_privacy::{Evidence, evaluate, save_candidate, verify_candidate};
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

/// Limit display labels, retaining the XML suffix and a stable row number.
/// Full filenames remain unchanged on disk and in private evidence.
fn display_name(filename: &str, number: usize) -> String {
    const WIDTH: usize = 27;
    let suffix = if filename.ends_with(".xml") { ".xml" } else { "" };
    let prefix_budget = WIDTH - suffix.len() - 5;
    let prefix: String = filename.chars().take(prefix_budget).collect();
    let abbreviated = if filename.chars().count() > WIDTH {
        format!("{prefix}…{suffix}")
    } else {
        filename.to_owned()
    };
    format!("{:>2} {}", number, abbreviated)
}

/// Display one concise row per file, with no private XML values.
/// A checkmark means the named processing step completed, not that
/// an independent privacy review or publication approval has occurred.
fn show_results(e: &fixture_privacy::VerifiedSetEvidence) {
    println!(
        "{:<30} {:>7} {:>7} {:>8}  {:>3} {:>3} {:>5}",
        "File", "Found", "Kept", "Removed", "Min", "ID", "Graph"
    );
    println!("{}", "-".repeat(72));
    let (mut found, mut kept, mut removed) = (0usize, 0usize, 0usize);
    for (index, row) in e.file_results.iter().enumerate() {
        found += row.readings_found;
        kept += row.readings_retained;
        removed += row.readings_removed;
        let minimized =
            row.operations.iter().any(|item| item == "history minimized");
        let pseudonymized = row
            .operations
            .iter()
            .any(|item| item == "identifiers pseudonymized");
        let graph =
            row.operations.iter().any(|item| item == "Atom graph verified");
        let mark = |passed: bool| if passed { "✓" } else { "-" };
        println!(
            "{:<30} {:>7} {:>7} {:>8}  {:>3} {:>3} {:>5}",
            display_name(&row.filename, index + 1),
            row.readings_found,
            row.readings_retained,
            row.readings_removed,
            mark(minimized),
            mark(pseudonymized),
            mark(graph)
        );
    }
    println!("{}", "-".repeat(72));
    println!(
        "{:<30} {:>7} {:>7} {:>8}",
        format!("TOTAL ({} files)", e.file_results.len()),
        found,
        kept,
        removed
    );
    println!("Min = history selection; ID = UUID pseudonymization;");
    println!("Graph = Atom graph check; ✓ = completed; - = not recorded.");
    println!("Checks do not establish privacy clearance.");
    let flagged = e
        .candidates
        .values()
        .filter(|item| !item.residual_risks.is_empty())
        .count();
    println!(
        "Privacy review: {} of {} files have flagged risks.",
        flagged,
        e.candidates.len()
    );
}

fn privacy_secret() -> Result<String> {
    env::var("FIXTURE_PRIVACY_SECRET").map_err(|_| {
        "Missing FIXTURE_PRIVACY_SECRET. See \
         docs/PRIVACY_SANITIZATION.md#first-time-setup-privacy-secret"
            .into()
    })
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// The public fixture workflow is intentionally limited to two commands.
// Other commands are diagnostics, not alternate release workflows.
const HELP: &str = "\
fixture_privacy — prepare private Green Button test fixtures

USAGE:
  fixture_privacy prepare SOURCE_DIR OUTPUT_DIR
  fixture_privacy verify SOURCE_DIR OUTPUT_DIR
  fixture_privacy audit INPUT.xml
  fixture_privacy sanitize INPUT.xml OUTPUT.xml REPORT.json
  fixture_privacy verify-file OUTPUT.xml REPORT.json
  fixture_privacy relationships DIRECTORY
  fixture_privacy --help

PREPARE:
  Minimize history, pseudonymize identifiers, check the Atom graph,
  and generate private XML and review reports. SOURCE_DIR must hold
  original private XML; OUTPUT_DIR must not exist and must not overlap
  the source. Requires FIXTURE_PRIVACY_SECRET (at least 32 characters).
  Defaults: seven days hourly, seven days daily, three monthly readings;
  keep complete short histories up to 31 days or three monthly readings.
  Why: retain regression scenarios without publishing full usage history.
  Unsupported reading frequencies fail closed.
  Per-file history operations are reported in a private table.

VERIFY:
  Recreate candidates using the source and original private secret;
  compare generated XML and review reports. Never grants approval.
  Secret setup: docs/PRIVACY_SANITIZATION.md

AUDIT / SANITIZE / VERIFY-FILE / RELATIONSHIPS:
  Diagnostic single-file and relationship operations. These do not
  create an approved public test fixture.

Neither preparation nor verification authorizes publication.
";

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("{HELP}");
        return Ok(());
    }
    match args[0].as_str() {
        "prepare" if args.len() == 3 => {
            let secret = privacy_secret()?;
            let evidence = fixture_privacy::generate_verified_minimized_set(
                Path::new(&args[1]),
                Path::new(&args[2]),
                &secret,
                fixture_privacy::SamplingPolicy::default(),
            )?;
            show_results(&evidence);
            println!("Minimized, graph-verified private candidates created.");
            println!("Privacy review: REQUIRED | Publication: NOT AUTHORIZED");
            Ok(())
        }
        "audit" if args.len() == 2 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&evaluate(&fs::read(&args[1])?)?)?
            );
            Ok(())
        }
        "sanitize" if args.len() == 4 => {
            let secret = privacy_secret()?;
            if Path::new(&args[3]).exists() {
                return Err("evidence output exists".into());
            }
            let evidence = save_candidate(
                Path::new(&args[1]),
                Path::new(&args[2]),
                &secret,
            )?;
            fs::write(&args[3], serde_json::to_vec_pretty(&evidence)?)?;
            println!("Candidate created; publication NOT authorized");
            Ok(())
        }
        "verify" if args.len() == 3 => {
            let secret = privacy_secret()?;
            let evidence = fixture_privacy::verify_generated_set(
                Path::new(&args[1]),
                Path::new(&args[2]),
                &secret,
            )?;
            show_results(&evidence);
            println!("Candidate XML and reports verified.");
            println!("Privacy review: REQUIRED | Publication: NOT AUTHORIZED");
            Ok(())
        }
        "verify-file" if args.len() == 3 => {
            let evidence: Evidence =
                serde_json::from_slice(&fs::read(&args[2])?)?;
            verify_candidate(Path::new(&args[1]), &evidence)?;
            println!("Evidence matches; manual review required");
            Ok(())
        }
        "relationships" if args.len() == 2 => {
            let mut files = Vec::new();
            for entry in fs::read_dir(&args[1])? {
                let entry = entry?;
                if entry.path().extension().is_some_and(|x| x == "xml") {
                    if !entry.file_type()?.is_file() {
                        return Err("XML input is not a regular file".into());
                    }
                    files.push(fs::read(entry.path())?);
                }
            }
            if files.is_empty() {
                return Err("no XML inputs found".into());
            }
            let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
            let report = fixture_privacy::inspect_relationships(&refs)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            println!("Relationship check is not privacy clearance.");
            Ok(())
        }
        _ => Err("unknown command; run fixture_privacy --help".into()),
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_have_bounded_width_and_keep_suffix() {
        let name =
            display_name("Electric_Daily_9-7-2025_10-6-2026_20261007.xml", 1);
        assert!(name.chars().count() <= 30);
        assert!(name.ends_with(".xml"));
        assert!(!name.contains("20261007"));
    }

    #[test]
    fn short_names_are_not_truncated() {
        assert_eq!(display_name("Gas.xml", 2), " 2 Gas.xml");
    }

    #[test]
    fn primary_commands_are_unambiguous() {
        assert!(HELP.contains("prepare SOURCE_DIR OUTPUT_DIR"));
        assert!(HELP.contains("verify SOURCE_DIR OUTPUT_DIR"));
        assert!(HELP.contains("verify-file OUTPUT.xml REPORT.json"));
        assert!(!HELP.contains("prepare-set"));
        assert!(!HELP.contains("generate-verified"));
        assert!(!HELP.contains("verify-generated"));
        assert!(!HELP.contains("verify-set"));
    }

    #[test]
    fn help_explains_publication_boundary_and_defaults() {
        assert!(HELP.contains("seven days hourly"));
        assert!(HELP.contains("three monthly readings"));
        assert!(HELP.contains("Neither preparation nor verification"));
    }
}
