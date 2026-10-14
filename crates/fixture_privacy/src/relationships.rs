// SPDX-License-Identifier: MIT
//! Offline Atom/Green Button relationship inspection (ADR-0028).
//!
//! The index associates an entry's Atom ID with its `self` URL.
//! Reports contain counts only; the inspector never fetches URLs.

use crate::{Result, validate_xml};
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Aggregate results; no resource identifier or URL is serialized.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationshipReport {
    pub documents: usize,
    pub identifiers: usize,
    pub references: usize,
    pub resolved_references: usize,
    pub external_references: usize,
    pub unresolved_local_references: usize,
    pub duplicate_identifiers: usize,
    /// Atom IDs mapping to more than one distinct `self` URL.
    pub conflicting_identifiers: usize,
    /// `self` URLs mapping to more than one distinct Atom ID.
    pub conflicting_self_links: usize,
    /// Exact entry-ID/self-URL pairs repeated in overlapping exports.
    pub repeated_resource_pairs: usize,
    /// `self` URLs used with several IDs, excluding identical pairs.
    pub self_urls_with_multiple_ids: usize,
    /// Atom IDs used with several URLs, excluding identical pairs.
    pub ids_with_multiple_self_urls: usize,
    /// Conflicting self URLs grouped by a recognized resource type.
    /// Unknown URL path components are never serialized.
    pub conflicting_self_link_types: BTreeMap<String, usize>,
}

#[derive(Default)]
struct Entry {
    id: Option<String>,
    self_url: Option<String>,
}

fn attribute_value(
    element: &BytesStart<'_>,
    key: &str,
) -> Result<Option<String>> {
    for attribute in element.attributes() {
        let attribute = attribute?;
        if attribute.key.local_name().as_ref() == key {
            return Ok(Some(
                attribute
                    .normalized_value(XmlVersion::Explicit1_0)?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

/// Return only recognized resource types, never private URL components.
/// A typical resource URL ends with `/<ResourceType>/<ResourceId>`.
fn resource_type_from_url(url: &str) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let mut components = path.trim_end_matches('/').rsplit('/');
    let _resource_id = components.next();
    let kind = components.next().unwrap_or("");
    match kind {
        "UsagePoint" => "UsagePoint",
        "ReadingType" => "ReadingType",
        "MeterReading" => "MeterReading",
        "IntervalBlock" => "IntervalBlock",
        "RetailCustomer" => "RetailCustomer",
        "CustomerAgreement" => "CustomerAgreement",
        "ElectricPowerUsageSummary" => "ElectricPowerUsageSummary",
        _ => "other",
    }
}

fn register_entry(
    entry: Entry,
    ids: &mut BTreeMap<String, BTreeSet<String>>,
    urls: &mut BTreeMap<String, BTreeSet<String>>,
) {
    if let Some(id) = entry.id {
        let url = entry.self_url.unwrap_or_default();
        ids.entry(id.clone()).or_default().insert(url.clone());
        if !url.is_empty() {
            urls.entry(url).or_default().insert(id);
        }
    }
}

/// Inspect resource references across a set of complete XML documents.
///
/// Exact Atom IDs and each entry's `self` URL are equivalent resource
/// identities for lookup. Other absolute links missing from the set are
/// classified as external, because a partial export can be legitimate.
/// A duplicate resource with a different ID/URL association is reported
/// as a conflict, never silently merged.
pub fn inspect_relationships(xml_set: &[&[u8]]) -> Result<RelationshipReport> {
    let mut report = RelationshipReport {
        documents: xml_set.len(),
        ..RelationshipReport::default()
    };
    let mut ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut urls: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut id_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut references: Vec<String> = Vec::new();
    let mut pair_counts: BTreeMap<(String, String), usize> = BTreeMap::new();

    for xml in xml_set {
        validate_xml(xml)?;
        let mut reader = Reader::from_reader(*xml);
        let mut entry: Option<Entry> = None;
        let mut depth = 0usize;
        let mut id_text = false;

        loop {
            match reader.read_event()? {
                Event::Start(e) => {
                    let name = e.local_name();
                    if name.as_ref() == "entry" && entry.is_none() {
                        entry = Some(Entry::default());
                        depth = 1;
                    } else if entry.is_some() {
                        depth += 1;
                    }
                    id_text =
                        entry.is_some() && name.as_ref() == "id" && depth == 2;
                    if name.as_ref() == "link" {
                        let href = attribute_value(&e, "href")?;
                        let rel = attribute_value(&e, "rel")?;
                        if let Some(href) = href {
                            if rel.as_deref() == Some("self")
                                && let Some(ref mut current) = entry
                                && depth == 2
                            {
                                current.self_url = Some(href.clone());
                            }
                            references.push(href);
                        }
                    }
                }
                Event::Empty(e) if e.local_name().as_ref() == "link" => {
                    let href = attribute_value(&e, "href")?;
                    let rel = attribute_value(&e, "rel")?;
                    if let Some(href) = href {
                        if rel.as_deref() == Some("self")
                            && let Some(ref mut current) = entry
                            && depth == 1
                        {
                            current.self_url = Some(href.clone());
                        }
                        references.push(href);
                    }
                }
                Event::Text(e) if id_text => {
                    if let Some(ref mut current) = entry {
                        let id = e.as_ref().trim();
                        if !id.is_empty() {
                            current.id = Some(id.to_owned());
                            *id_counts.entry(id.to_owned()).or_default() += 1;
                        }
                    }
                }
                Event::End(e) => {
                    let name = e.local_name();
                    if name.as_ref() == "id" {
                        id_text = false;
                    }
                    if entry.is_some() {
                        if depth == 1 && name.as_ref() == "entry" {
                            let completed = entry.take().unwrap();
                            if let Some(ref id) = completed.id {
                                let self_url = completed
                                    .self_url
                                    .clone()
                                    .unwrap_or_default();
                                *pair_counts
                                    .entry((id.clone(), self_url))
                                    .or_default() += 1;
                            }
                            register_entry(completed, &mut ids, &mut urls);
                        }
                        depth = depth.saturating_sub(1);
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
    }

    report.identifiers = ids.len();
    report.references = references.len();
    report.duplicate_identifiers =
        id_counts.values().filter(|&&count| count > 1).count();
    report.conflicting_identifiers =
        ids.values().filter(|urls| urls.len() > 1).count();
    report.conflicting_self_links =
        urls.values().filter(|ids| ids.len() > 1).count();
    report.ids_with_multiple_self_urls = report.conflicting_identifiers;
    report.self_urls_with_multiple_ids = report.conflicting_self_links;
    report.repeated_resource_pairs =
        pair_counts.values().filter(|&&count| count > 1).count();
    for (url, atom_ids) in &urls {
        if atom_ids.len() > 1 {
            let kind = resource_type_from_url(url);
            *report
                .conflicting_self_link_types
                .entry(kind.to_owned())
                .or_default() += 1;
        }
    }

    for href in references {
        let lookup = href.strip_prefix('#').unwrap_or(&href);
        if ids.contains_key(lookup) || urls.contains_key(lookup) {
            report.resolved_references += 1;
        } else if href.starts_with('#') {
            report.unresolved_local_references += 1;
        } else {
            report.external_references += 1;
        }
    }
    Ok(report)
}

/// Compare aggregate relationship integrity before and after processing.
///
/// This checks invariant counts and reported conflicts, NOT equality of
/// identities. A graph with rewritten, broken links can still have the same
/// aggregate statistics. Semantic equivalence requires a private mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipComparison {
    pub before: RelationshipReport,
    pub after: RelationshipReport,
    pub structural_counts_match: bool,
    pub regression_review_required: bool,
}

/// Inspect two offline XML sets and compare privacy-safe diagnostics.
pub fn compare_relationships(
    before: &[&[u8]],
    after: &[&[u8]],
) -> Result<RelationshipComparison> {
    let before = inspect_relationships(before)?;
    let after = inspect_relationships(after)?;
    let structural_counts_match = before.documents == after.documents
        && before.identifiers == after.identifiers
        && before.references == after.references
        && before.resolved_references == after.resolved_references
        && before.external_references == after.external_references
        && before.unresolved_local_references
            == after.unresolved_local_references
        && before.conflicting_identifiers == after.conflicting_identifiers
        && before.conflicting_self_links == after.conflicting_self_links
        && before.conflicting_self_link_types
            == after.conflicting_self_link_types;
    // Equal counts are necessary for this comparison, never sufficient
    // to certify that each original resource remains connected.
    Ok(RelationshipComparison {
        before,
        after,
        structural_counts_match,
        regression_review_required: true,
    })
}
