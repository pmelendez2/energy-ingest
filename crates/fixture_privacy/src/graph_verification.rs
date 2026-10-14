// SPDX-License-Identifier: MIT
//! Exact offline relationship checks using a private identity mapping.
//! Never serialize the mapping or original/candidate resource values.

use crate::{Result, validate_xml};
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Original ID or URL -> candidate ID or URL. Keep this private.
pub type IdentityMap = BTreeMap<String, String>;

type Edge = (String, String, String);
type Edges = BTreeMap<Edge, usize>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphVerificationReport {
    pub source_entries: usize,
    pub candidate_entries: usize,
    pub source_links: usize,
    pub candidate_links: usize,
    pub exact_relationships_preserved: bool,
    pub publication_authorized: bool,
}

#[derive(Default)]
struct Entry {
    id: Option<String>,
    links: Vec<(String, String)>,
}

fn attr(e: &BytesStart<'_>, key: &str) -> Result<Option<String>> {
    for value in e.attributes() {
        let value = value?;
        if value.key.local_name().as_ref() == key {
            return Ok(Some(
                value.normalized_value(XmlVersion::Explicit1_0)?.into_owned(),
            ));
        }
    }
    Ok(None)
}

fn add_link(e: &BytesStart<'_>, entry: &mut Option<Entry>) -> Result<()> {
    let rel = attr(e, "rel")?.unwrap_or_default();
    if let Some(href) = attr(e, "href")?
        && let Some(entry) = entry
    {
        entry.links.push((rel, href));
    }
    Ok(())
}

fn read_entries(xml_set: &[&[u8]]) -> Result<Vec<Entry>> {
    let mut result = Vec::new();
    for xml in xml_set {
        validate_xml(xml)?;
        let mut reader = Reader::from_reader(*xml);
        let mut current: Option<Entry> = None;
        let mut depth = 0usize;
        let mut id_text = false;
        loop {
            match reader.read_event()? {
                Event::Start(e) => {
                    let name = e.local_name();
                    if name.as_ref() == "entry" && current.is_none() {
                        current = Some(Entry::default());
                        depth = 1;
                    } else if current.is_some() {
                        depth += 1;
                    }
                    id_text = current.is_some()
                        && name.as_ref() == "id"
                        && depth == 2;
                    if name.as_ref() == "link" && depth == 2 {
                        add_link(&e, &mut current)?;
                    }
                }
                Event::Empty(e) => {
                    if e.local_name().as_ref() == "link" && depth == 1 {
                        add_link(&e, &mut current)?;
                    }
                }
                Event::Text(e) if id_text => {
                    if let Some(entry) = current.as_mut() {
                        let id = e.as_ref().trim();
                        if !id.is_empty() {
                            entry.id = Some(id.to_owned());
                        }
                    }
                }
                Event::End(e) => {
                    let name = e.local_name();
                    if name.as_ref() == "id" {
                        id_text = false;
                    }
                    if current.is_some() {
                        if depth == 1 && name.as_ref() == "entry" {
                            result.push(current.take().unwrap());
                        }
                        depth = depth.saturating_sub(1);
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
    }
    Ok(result)
}

fn edges(
    entries: &[Entry],
    map: Option<&IdentityMap>,
) -> Result<(Edges, usize)> {
    let mut result = Edges::new();
    let mut links = 0;
    for entry in entries {
        let id = entry
            .id
            .as_deref()
            .ok_or("entry missing Atom id; graph verification refused")?;
        let source_id = mapped(id, map)?;
        // Include entries without links: a missing resource is a failure.
        *result
            .entry((source_id.clone(), String::new(), String::new()))
            .or_default() += 1;
        for (rel, href) in &entry.links {
            let target = mapped(href, map)?;
            *result
                .entry((source_id.clone(), rel.clone(), target))
                .or_default() += 1;
            links += 1;
        }
    }
    Ok((result, links))
}

fn mapped(value: &str, map: Option<&IdentityMap>) -> Result<String> {
    match map {
        Some(map) => map
            .get(value)
            .cloned()
            .ok_or_else(|| "missing private identity correspondence".into()),
        None => Ok(value.to_owned()),
    }
}

/// Compare every Atom entry and its direct links (including `self`,
/// `up`, and `related`) as a multiset after applying private mappings.
///
/// The mapping must cover every source Atom ID and direct link href,
/// even intentionally unchanged external links. This fails closed.
/// Entry and link order does not affect the result; multiplicity does.
/// The caller must independently attest that mappings were produced by
/// the sanitizer rather than constructed to disguise changed edges.
/// Does not inspect nested ESPI references or feed-level links.
pub fn verify_graph(
    source: &[&[u8]],
    candidate: &[&[u8]],
    mapping: &IdentityMap,
) -> Result<GraphVerificationReport> {
    let source_entries = read_entries(source)?;
    let candidate_entries = read_entries(candidate)?;
    let (expected, source_links) = edges(&source_entries, Some(mapping))?;
    let (observed, candidate_links) = edges(&candidate_entries, None)?;
    Ok(GraphVerificationReport {
        source_entries: source_entries.len(),
        candidate_entries: candidate_entries.len(),
        source_links,
        candidate_links,
        exact_relationships_preserved: expected == observed,
        publication_authorized: false,
    })
}

/// Verify that structural sampling retained the exact Atom graph.
/// Unlike a sanitizer comparison, no identity mapping is permitted:
/// sampling must not alter any Atom entry IDs or direct links.
pub fn verify_unchanged_graph(
    before: &[&[u8]],
    after: &[&[u8]],
) -> Result<bool> {
    let original = read_entries(before)?;
    let sampled = read_entries(after)?;
    let (expected, _) = edges(&original, None)?;
    let (actual, _) = edges(&sampled, None)?;
    Ok(expected == actual)
}
