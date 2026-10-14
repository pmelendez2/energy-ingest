// SPDX-License-Identifier: MIT
//! Fail-closed graph verification bound to UUID candidate generation.
//! Original identifiers and correspondence never enter serializable reports.

use crate::graph_verification::{
    GraphVerificationReport, IdentityMap, verify_graph,
};
use crate::{Result, sanitize_uuid_references};
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::collections::BTreeMap;

/// A verified in-memory candidate set. Write only after all checks pass.
/// Source/candidate bytes must remain in private staging until review.
#[derive(Debug)]
pub struct VerifiedCandidateSet {
    pub candidates: Vec<Vec<u8>>,
    pub graph_report: GraphVerificationReport,
}

fn attribute(element: &BytesStart<'_>, key: &str) -> Result<Option<String>> {
    for attr in element.attributes() {
        let attr = attr?;
        if attr.key.local_name().as_ref() == key {
            return Ok(Some(
                attr.normalized_value(XmlVersion::Explicit1_0)?.into_owned(),
            ));
        }
    }
    Ok(None)
}

fn insert_mapping(
    mapping: &mut IdentityMap,
    value: &str,
    secret: &str,
) -> Result<()> {
    let (mapped, _) = sanitize_uuid_references(value.as_bytes(), secret)
        .or_else(|_| {
            // sanitize_uuid_references validates XML, not arbitrary ID strings.
            // Wrap escaped text as XML attribute instead; the sanitizer works
            // on the full document and preserves all remaining characters.
            let escaped = value
                .replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('<', "&lt;");
            let xml = format!("<v id=\"{escaped}\"/>");
            let (sanitized, count) =
                sanitize_uuid_references(xml.as_bytes(), secret)?;
            let mut reader = Reader::from_reader(sanitized.as_slice());
            let result = match reader.read_event()? {
                Event::Empty(e) => {
                    attribute(&e, "id")?.ok_or("mapped attribute missing")?
                }
                _ => return Err("mapped XML is malformed".into()),
            };
            Ok::<_, Box<dyn std::error::Error>>((result.into_bytes(), count))
        })?;
    let replacement = String::from_utf8(mapped)?;
    if let Some(existing) =
        mapping.insert(value.to_owned(), replacement.clone())
        && existing != replacement
    {
        return Err("ambiguous private identity mapping".into());
    }
    Ok(())
}

fn capture_source_links(
    xml: &[u8],
    map: &mut IdentityMap,
    secret: &str,
) -> Result<()> {
    let mut reader = Reader::from_reader(xml);
    let mut entry_depth = 0usize;
    let mut in_id = false;
    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                let name = e.local_name();
                if name.as_ref() == "entry" && entry_depth == 0 {
                    entry_depth = 1;
                } else if entry_depth > 0 {
                    entry_depth += 1;
                }
                in_id = entry_depth == 2 && name.as_ref() == "id";
                if entry_depth == 2
                    && name.as_ref() == "link"
                    && let Some(href) = attribute(&e, "href")?
                {
                    insert_mapping(map, &href, secret)?;
                }
            }
            Event::Empty(e)
                if entry_depth == 1 && e.local_name().as_ref() == "link" =>
            {
                if let Some(href) = attribute(&e, "href")? {
                    insert_mapping(map, &href, secret)?;
                }
            }
            Event::Text(t) if in_id => {
                insert_mapping(map, t.as_ref().trim(), secret)?;
            }
            Event::End(e) => {
                if e.local_name().as_ref() == "id" {
                    in_id = false;
                }
                entry_depth = entry_depth.saturating_sub(1);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

/// Sanitize all documents with the same secret and independently check
/// every Atom entry ID and direct link using sanitizer-derived mapping.
/// A mismatch is an error, including when overall counts are unchanged.
/// No candidate bytes are returned on error; no file is ever written here.
/// This is relationship integrity, NOT privacy or publication clearance.
pub fn sanitize_set_verified(
    originals: &[Vec<u8>],
    secret: &str,
) -> Result<VerifiedCandidateSet> {
    if originals.is_empty() {
        return Err("empty source set".into());
    }
    let mut mapping: IdentityMap = BTreeMap::new();
    let mut candidates = Vec::with_capacity(originals.len());
    for source in originals {
        let (candidate, _) = sanitize_uuid_references(source, secret)?;
        capture_source_links(source, &mut mapping, secret)?;
        candidates.push(candidate);
    }
    let source: Vec<&[u8]> = originals.iter().map(Vec::as_slice).collect();
    let target: Vec<&[u8]> = candidates.iter().map(Vec::as_slice).collect();
    let graph_report = verify_graph(&source, &target, &mapping)?;
    if !graph_report.exact_relationships_preserved {
        return Err("sanitized relationship graph changed".into());
    }
    Ok(VerifiedCandidateSet { candidates, graph_report })
}
