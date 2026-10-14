// SPDX-License-Identifier: MIT
//! Read-only, privacy-safe Atom/ESPI structure diagnostic.
//!
//! This command deliberately emits only counts and fixed category labels.
//! It does not contact provider URLs or modify input XML.

use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Default, Serialize)]
struct StructureReport {
    documents: usize,
    atom_entries: usize,
    atom_entry_ids: usize,
    links: usize,
    entry_self_links: usize,
    self_link_matches_entry_id: usize,
    self_link_differs_from_entry_id: usize,
    link_relationships: BTreeMap<&'static str, usize>,
    resource_paths: BTreeMap<&'static str, usize>,
    /// This diagnostic compares textual identities, not ESPI semantics.
    exact_link_to_entry_id_matches: usize,
}

fn attribute(e: &BytesStart<'_>, name: &str) -> Result<Option<String>> {
    for attribute in e.attributes() {
        let attribute = attribute?;
        if attribute.key.local_name().as_ref() == name {
            let value = attribute
                .normalized_value(XmlVersion::Explicit1_0)?
                .into_owned();
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn rel_class(value: Option<&str>) -> &'static str {
    let Some(value) = value else {
        return "unspecified";
    };
    // A URL may encode an Atom relation in its final segment.
    match value.rsplit('/').next().unwrap_or(value) {
        "self" => "self",
        "related" => "related",
        "up" => "up",
        "alternate" => "alternate",
        "next" => "next",
        "previous" | "prev" => "previous",
        _ => "other",
    }
}

fn resource_class(value: &str) -> &'static str {
    // Only these fixed labels are emitted; raw provider paths stay private.
    for (fragment, label) in [
        ("/UsagePoint/", "UsagePoint"),
        ("/MeterReading/", "MeterReading"),
        ("/ReadingType/", "ReadingType"),
        ("/IntervalBlock/", "IntervalBlock"),
        ("/ElectricPowerUsageSummary/", "UsageSummary"),
        ("/UsageSummary/", "UsageSummary"),
        ("/RetailCustomer/", "RetailCustomer"),
    ] {
        if value.contains(fragment) {
            return label;
        }
    }
    "other"
}

fn inspect(data: &[Vec<u8>]) -> Result<StructureReport> {
    let mut report =
        StructureReport { documents: data.len(), ..StructureReport::default() };
    let mut ids = BTreeSet::<String>::new();
    let mut links = Vec::<String>::new();
    for xml in data {
        fixture_privacy::validate_xml(xml)?;
        let mut reader = Reader::from_reader(xml.as_slice());
        let mut in_entry = false;
        let mut in_id = false;
        let mut entry_id: Option<String> = None;
        let mut self_link: Option<String> = None;
        loop {
            match reader.read_event()? {
                Event::Start(e) => match e.local_name().as_ref() {
                    "entry" => {
                        in_entry = true;
                        entry_id = None;
                        self_link = None;
                        report.atom_entries += 1;
                    }
                    "id" if in_entry => in_id = true,
                    "link" => {
                        visit_link(
                            &e,
                            in_entry,
                            &mut self_link,
                            &mut report,
                            &mut links,
                        )?;
                    }
                    _ => {}
                },
                Event::Empty(e) if e.local_name().as_ref() == "link" => {
                    visit_link(
                        &e,
                        in_entry,
                        &mut self_link,
                        &mut report,
                        &mut links,
                    )?;
                }
                Event::Text(t) if in_id => {
                    let value = t.as_ref().trim();
                    if !value.is_empty() {
                        let value = value.to_owned();
                        ids.insert(value.clone());
                        entry_id = Some(value);
                        report.atom_entry_ids += 1;
                    }
                }
                Event::End(e) => match e.local_name().as_ref() {
                    "id" => in_id = false,
                    "entry" => {
                        if let Some(ref link) = self_link {
                            report.entry_self_links += 1;
                            if entry_id.as_deref() == Some(link.as_str()) {
                                report.self_link_matches_entry_id += 1;
                            } else {
                                report.self_link_differs_from_entry_id += 1;
                            }
                        }
                        in_entry = false;
                        in_id = false;
                    }
                    _ => {}
                },
                Event::Eof => break,
                _ => {}
            }
        }
    }
    report.exact_link_to_entry_id_matches =
        links.iter().filter(|link| ids.contains(link.as_str())).count();
    Ok(report)
}

fn visit_link(
    element: &BytesStart<'_>,
    in_entry: bool,
    self_link: &mut Option<String>,
    report: &mut StructureReport,
    links: &mut Vec<String>,
) -> Result<()> {
    let Some(href) = attribute(element, "href")? else {
        return Ok(());
    };
    let rel = attribute(element, "rel")?;
    let kind = rel_class(rel.as_deref());
    *report.link_relationships.entry(kind).or_default() += 1;
    *report.resource_paths.entry(resource_class(&href)).or_default() += 1;
    report.links += 1;
    if in_entry && kind == "self" {
        *self_link = Some(href.clone());
    }
    links.push(href);
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 1 || args[0] == "--help" {
        println!(
            "USAGE: relationship_structure DIRECTORY\n\
            Read-only Atom/ESPI structure counts for *.xml files.\n\
            No identifiers, URLs, or customer data are printed.\n\
            No network requests or writes are performed."
        );
        return if args.len() == 1 && args[0] == "--help" {
            Ok(())
        } else {
            Err("expected one directory argument".into())
        };
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(Path::new(&args[0]))? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "xml") {
            if !entry.file_type()?.is_file() {
                return Err("XML input must be a regular file".into());
            }
            paths.push(entry.path());
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err("no XML files found".into());
    }
    let mut xml = Vec::new();
    for path in paths {
        xml.push(fs::read(path)?);
    }
    println!("{}", serde_json::to_string_pretty(&inspect(&xml)?)?);
    println!("Structure diagnostics are not privacy clearance.");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::inspect;

    #[test]
    fn distinguishes_urn_id_from_self_url_without_leaking_them() {
        let xml = br#"<feed><entry><id>urn:uuid:secret-123</id>
          <link rel="self" href="https://example.invalid/UsagePoint/private"/>
          <link rel="related"
           href="https://example.invalid/MeterReading/private"/>
          </entry></feed>"#;
        let report = inspect(&[xml.to_vec()]).unwrap();
        assert_eq!(report.atom_entries, 1);
        assert_eq!(report.self_link_differs_from_entry_id, 1);
        assert_eq!(report.links, 2);
        let output = serde_json::to_string(&report).unwrap();
        assert!(!output.contains("secret-123"));
        assert!(!output.contains("example.invalid"));
    }

    #[test]
    fn exact_self_link_is_counted() {
        let xml = br#"<feed><entry><id>urn:uuid:test</id>
          <link rel="self" href="urn:uuid:test"/></entry></feed>"#;
        let report = inspect(&[xml.to_vec()]).unwrap();
        assert_eq!(report.self_link_matches_entry_id, 1);
        assert_eq!(report.exact_link_to_entry_id_matches, 1);
    }
}
