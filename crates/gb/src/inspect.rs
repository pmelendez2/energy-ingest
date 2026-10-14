// SPDX-License-Identifier: MIT

//! Generic Green Button XML parsing and inspection.
//!
//! # Purpose
//!
//! Parses and inspects Green Button / ESPI XML without depending on provider
//! filenames, portal labels, or provider-specific parsing rules.
//!
//! Relationships are resolved through Atom `link` elements so the applicable
//! `ReadingType` is selected for each `MeterReading`.
//!
//! # Architecture
//!
//! One parser builds [`GbDocument`]. Human-readable inspection is a
//! transformation of that normalized document so ingestion and inspection use
//! identical XML and relationship rules.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use chrono::DateTime;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use thiserror::Error;

/// Green Button commodity associated with a MeterReading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Commodity {
    /// Electricity.
    Electricity,

    /// Natural gas.
    NaturalGas,

    /// Green Button commodity code not yet mapped by this crate.
    Other(u32),
}

impl Commodity {
    /// Maps a raw Green Button commodity code to the inspection enum.
    ///
    /// Unknown codes are retained through [`Commodity::Other`].
    fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Electricity,
            7 => Self::NaturalGas,
            other => Self::Other(other),
        }
    }
}

impl fmt::Display for Commodity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Electricity => formatter.write_str("Electricity"),
            Self::NaturalGas => formatter.write_str("Natural gas"),
            Self::Other(code) => write!(formatter, "Commodity code {code}"),
        }
    }
}

/// Parsed Green Button resources needed by the ingestion path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GbDocument {
    /// Usage Point resources found in the document.
    pub usage_points: Vec<UsagePointRecord>,

    /// Reading Type resources found in the document.
    pub reading_types: Vec<ReadingTypeRecord>,

    /// Meter Reading resources found in the document.
    pub meter_readings: Vec<MeterReadingRecord>,

    /// Interval Block resources found in the document.
    pub interval_blocks: Vec<IntervalBlockRecord>,
}

/// Parsed Green Button Usage Point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsagePointRecord {
    /// Source identity from the resource self link.
    pub external_id: String,

    /// Related Green Button resource links.
    pub related_hrefs: Vec<String>,

    /// Green Button ServiceCategory.kind value.
    pub service_category_kind: Option<u32>,
}

/// Parsed Green Button Reading Type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingTypeRecord {
    /// Source identity from the resource self link.
    pub external_id: String,

    /// Green Button accumulationBehaviour value.
    pub accumulation_behaviour: Option<u32>,

    /// Green Button commodity value.
    pub commodity: Option<u32>,

    /// Green Button dataQualifier value.
    pub data_qualifier: Option<u32>,

    /// Green Button flowDirection value.
    pub flow_direction: Option<u32>,

    /// Green Button kind value.
    pub kind: Option<u32>,

    /// Green Button phase value.
    pub phase: Option<u32>,

    /// Base-10 exponent applied to reading values.
    pub power_of_ten_multiplier: Option<i16>,

    /// Green Button unit-of-measure value.
    pub unit_of_measure: Option<u32>,

    /// Green Button intervalLength value in seconds.
    pub interval_length_seconds: Option<u64>,
}

/// Parsed Green Button Meter Reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeterReadingRecord {
    /// Source identity from the resource self link.
    pub external_id: String,

    /// Parent collection link.
    pub up_href: Option<String>,

    /// Related Green Button resource links.
    pub related_hrefs: Vec<String>,
}

/// Parsed Green Button Interval Block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervalBlockRecord {
    /// Source identity from the resource self link.
    pub external_id: String,

    /// Parent IntervalBlock collection link.
    pub up_href: String,

    /// Green Button block start as Unix time.
    pub start_unix: i64,

    /// Green Button block duration in seconds.
    pub duration_seconds: u64,

    /// Atom published timestamp as Unix time, when supplied.
    pub published_unix: Option<i64>,

    /// Atom updated timestamp as Unix time, when supplied.
    pub updated_unix: Option<i64>,

    /// Interval readings contained by this block.
    pub readings: Vec<IntervalReadingRecord>,
}

/// Parsed Green Button Interval Reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervalReadingRecord {
    /// UTC interval start as Unix time.
    pub start_unix: i64,

    /// Interval duration in seconds.
    pub duration_seconds: u32,

    /// Raw Green Button reading value.
    pub value: i64,

    /// Green Button ReadingQuality.quality value.
    pub quality: Option<u32>,
}

/// Human-readable inspection summary for one Green Button MeterReading.
///
/// This is an `energy_ingest` summary type built from the Green Button
/// `MeterReading`, its related `ReadingType`, and associated `IntervalBlock`
/// resources. It is not a new Green Button resource type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeterReadingSummary {
    /// Commodity reported by the related `ReadingType`.
    pub commodity: Commodity,

    /// Green Button flow-direction code.
    pub flow_direction: Option<u32>,

    /// Green Button unit-of-measure code.
    pub unit_of_measure: Option<u32>,

    /// Nominal interval length from `ReadingType`, in seconds.
    pub nominal_interval_seconds: Option<u64>,

    /// Distinct durations observed in `IntervalReading`, in seconds.
    pub observed_interval_seconds: Vec<u64>,

    /// Number of `IntervalReading` records associated with the MeterReading.
    pub reading_count: u64,

    /// Earliest observed reading start as Unix time.
    pub start_unix: Option<i64>,

    /// Latest observed reading end, exclusive, as Unix time.
    pub end_unix_exclusive: Option<i64>,
}

impl MeterReadingSummary {
    /// Returns true when exactly one reading duration was observed.
    pub fn has_uniform_observed_interval(&self) -> bool {
        self.observed_interval_seconds.len() == 1
    }

    /// Returns true when nominal and observed intervals agree.
    pub fn interval_matches_reading_type(&self) -> bool {
        match (
            self.nominal_interval_seconds,
            self.observed_interval_seconds.as_slice(),
        ) {
            (Some(nominal), [observed]) => nominal == *observed,
            _ => false,
        }
    }

    /// Returns the Green Button flow-direction label when known.
    pub fn flow_direction_label(&self) -> Option<&'static str> {
        match self.flow_direction {
            Some(0) => Some("N/A"),
            Some(1) => Some("Forward"),
            Some(19) => Some("Reverse"),
            Some(20) => Some("Total"),
            Some(21) => Some("Total by phase"),
            _ => None,
        }
    }

    /// Returns the Green Button unit-of-measure label when known.
    pub fn unit_of_measure_label(&self) -> Option<&'static str> {
        match self.unit_of_measure {
            Some(31) => Some("J"),
            Some(42) => Some("m3"),
            Some(72) => Some("Wh"),
            Some(119) => Some("ft3"),
            Some(132) => Some("Btu"),
            Some(169) => Some("therm"),
            _ => None,
        }
    }
}

/// Provider-independent summary of one Green Button XML document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GbInspection {
    /// MeterReadings resolved from Green Button relationships.
    pub meter_readings: Vec<MeterReadingSummary>,

    /// Number of `ReadingType` resources present.
    pub reading_type_count: usize,

    /// Number of `MeterReading` resources present.
    pub meter_reading_count: usize,

    /// Number of `IntervalBlock` resources present.
    pub interval_block_count: usize,
}

/// Typed errors returned while inspecting Green Button XML.
#[derive(Debug, Error)]
pub enum InspectionError {
    /// File access failed.
    #[error("failed to read Green Button XML: {0}")]
    Io(#[from] std::io::Error),

    /// XML parsing failed.
    #[error("invalid XML: {0}")]
    Xml(#[from] quick_xml::Error),

    /// An XML attribute could not be parsed.
    #[error("invalid XML attribute: {0}")]
    Attribute(String),

    /// A Green Button numeric field contained an invalid value.
    #[error("invalid Green Button value for {field}: {value}")]
    InvalidNumber {
        /// Field whose value could not be parsed.
        field: &'static str,

        /// Unparseable source value.
        value: String,
    },

    /// A required ESPI relationship could not be resolved.
    #[error("unable to resolve Green Button relationship: {0}")]
    Relationship(String),

    /// An interval reading was missing required data.
    #[error("IntervalReading is missing start, duration, or value")]
    IncompleteIntervalReading,

    /// An IntervalBlock was missing its required interval.
    #[error("IntervalBlock is missing start or duration")]
    IncompleteIntervalBlock,

    /// An Atom timestamp could not be parsed.
    #[error("invalid timestamp for {field}: {value}")]
    InvalidTimestamp {
        /// Timestamp field whose value could not be parsed.
        field: &'static str,

        /// Unparseable source value.
        value: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceKind {
    UsagePoint,
    ReadingType,
    MeterReading,
    IntervalBlock,
}

#[derive(Debug, Default)]
struct EntryBuilder {
    kind: Option<ResourceKind>,
    self_href: Option<String>,
    up_href: Option<String>,
    related_hrefs: Vec<String>,

    service_category_kind: Option<u32>,

    accumulation_behaviour: Option<u32>,
    commodity: Option<u32>,
    data_qualifier: Option<u32>,
    flow_direction: Option<u32>,
    reading_kind: Option<u32>,
    phase: Option<u32>,
    power_of_ten_multiplier: Option<i16>,
    unit_of_measure: Option<u32>,
    interval_length: Option<u64>,

    block_start: Option<i64>,
    block_duration: Option<u64>,

    published: Option<String>,
    updated: Option<String>,

    readings: Vec<IntervalReadingRecord>,
    current_reading: Option<PendingReading>,
}

#[derive(Debug, Default)]
struct PendingReading {
    start: Option<i64>,
    duration: Option<u32>,
    value: Option<i64>,
    quality: Option<u32>,
}

#[derive(Debug, Default, Clone)]
struct IntervalStats {
    reading_count: u64,
    durations: BTreeSet<u64>,
    start_unix: Option<i64>,
    end_unix_exclusive: Option<i64>,
}

impl IntervalStats {
    /// Adds one observed interval to aggregate inspection statistics.
    ///
    /// The method tracks count, distinct durations, earliest start, and latest
    /// exclusive end without altering the retained parsed interval records.
    fn add(&mut self, start: i64, duration: u64) {
        self.reading_count += 1;
        self.durations.insert(duration);

        self.start_unix = Some(match self.start_unix {
            Some(current) => current.min(start),
            None => start,
        });

        let duration_i64 = i64::try_from(duration).unwrap_or(i64::MAX);
        let end = start.saturating_add(duration_i64);

        self.end_unix_exclusive = Some(match self.end_unix_exclusive {
            Some(current) => current.max(end),
            None => end,
        });
    }
}

/// Parses a Green Button XML file into normalized resource records.
///
/// # Inputs
///
/// - `path`: Filesystem path to Green Button / ESPI XML.
///
/// # Returns
///
/// Returns a [`GbDocument`] containing retained source resources and
/// interval readings.
///
/// # Errors
///
/// Returns [`InspectionError`] for file access, XML, numeric, timestamp, or
/// required-resource validation failures.
pub fn parse_file(
    path: impl AsRef<Path>,
) -> Result<GbDocument, InspectionError> {
    let file = File::open(path)?;
    parse_reader(BufReader::new(file))
}

/// Parses Green Button XML from a buffered reader.
///
/// # Inputs
///
/// - `reader`: Buffered Green Button / ESPI XML input.
///
/// # Returns
///
/// Returns a normalized [`GbDocument`].
///
/// # Errors
///
/// Returns [`InspectionError`] for XML, numeric, timestamp, relationship, or
/// required-resource validation failures.
pub fn parse_reader<R>(reader: R) -> Result<GbDocument, InspectionError>
where
    R: BufRead,
{
    let mut reader = Reader::from_reader(reader);
    reader.config_mut().trim_text(true);

    let mut buffer = Vec::new();
    let mut path = Vec::<String>::new();
    let mut current_entry = None::<EntryBuilder>;

    let mut usage_points = Vec::<UsagePointRecord>::new();
    let mut reading_types = Vec::<ReadingTypeRecord>::new();
    let mut meter_readings = Vec::<MeterReadingRecord>::new();
    let mut interval_blocks = Vec::<IntervalBlockRecord>::new();

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) => {
                let name = local_name(&event);

                if name == "entry" {
                    current_entry = Some(EntryBuilder::default());
                }

                if let Some(entry) = current_entry.as_mut() {
                    classify_resource(entry, &name);

                    if name == "IntervalReading" {
                        entry.current_reading = Some(PendingReading::default());
                    }

                    if name == "link" {
                        read_link(entry, &event)?;
                    }
                }

                path.push(name);
            }

            Event::Empty(event) => {
                if let Some(entry) = current_entry.as_mut() {
                    let name = local_name(&event);
                    classify_resource(entry, &name);

                    if name == "link" {
                        read_link(entry, &event)?;
                    }
                }
            }

            Event::Text(event) => {
                if let Some(entry) = current_entry.as_mut() {
                    let text = event.as_ref();
                    read_text(entry, &path, text.trim())?;
                }
            }

            Event::End(event) => {
                let name = event.name().local_name().as_ref().to_string();

                if name == "IntervalReading" {
                    finish_interval_reading(current_entry.as_mut())?;
                }

                if name == "entry"
                    && let Some(entry) = current_entry.take()
                {
                    finish_entry(
                        entry,
                        &mut usage_points,
                        &mut reading_types,
                        &mut meter_readings,
                        &mut interval_blocks,
                    )?;
                }

                path.pop();
            }

            Event::Eof => break,

            _ => {}
        }

        buffer.clear();
    }

    Ok(GbDocument {
        usage_points,
        reading_types,
        meter_readings,
        interval_blocks,
    })
}

/// Inspects a Green Button XML file.
///
/// # Inputs
///
/// - `path`: Filesystem path to Green Button / ESPI XML.
///
/// # Returns
///
/// Returns a provider-independent [`GbInspection`] summary.
///
/// # Errors
///
/// Returns the same [`InspectionError`] values as [`parse_file`].
pub fn inspect_file(
    path: impl AsRef<Path>,
) -> Result<GbInspection, InspectionError> {
    build_inspection(parse_file(path)?)
}

/// Inspects Green Button XML from a buffered reader.
///
/// # Inputs
///
/// - `reader`: Buffered Green Button / ESPI XML input.
///
/// # Returns
///
/// Returns a provider-independent [`GbInspection`] summary.
///
/// # Errors
///
/// Returns the same [`InspectionError`] values as [`parse_reader`].
pub fn inspect_reader<R>(reader: R) -> Result<GbInspection, InspectionError>
where
    R: BufRead,
{
    build_inspection(parse_reader(reader)?)
}

/// Returns the namespace-independent local name of an XML start element.
fn local_name(event: &BytesStart<'_>) -> String {
    event.local_name().as_ref().to_string()
}

/// Updates an entry builder when a supported Green Button resource element is
/// encountered.
fn classify_resource(entry: &mut EntryBuilder, name: &str) {
    entry.kind = match name {
        "UsagePoint" => Some(ResourceKind::UsagePoint),
        "ReadingType" => Some(ResourceKind::ReadingType),
        "MeterReading" => Some(ResourceKind::MeterReading),
        "IntervalBlock" => Some(ResourceKind::IntervalBlock),
        _ => entry.kind,
    };
}

/// Reads an Atom link and retains supported `self`, `up`, and `related`
/// relationships on the current entry.
///
/// Returns [`InspectionError::Attribute`] when an XML attribute is malformed.
fn read_link(
    entry: &mut EntryBuilder,
    event: &BytesStart<'_>,
) -> Result<(), InspectionError> {
    let mut relation = None::<String>;
    let mut href = None::<String>;

    for attribute in event.attributes() {
        let attribute = attribute
            .map_err(|error| InspectionError::Attribute(error.to_string()))?;

        let key = attribute.key.local_name().as_ref().to_string();

        let value = attribute.value.as_ref().to_string();

        match key.as_str() {
            "rel" => relation = Some(value),
            "href" => href = Some(value),
            _ => {}
        }
    }

    let (Some(relation), Some(href)) = (relation, href) else {
        return Ok(());
    };

    match relation.as_str() {
        "self" => entry.self_href = Some(href),
        "up" => entry.up_href = Some(href),
        "related" => entry.related_hrefs.push(href),
        _ => {}
    }

    Ok(())
}

/// Maps text at the current XML path into normalized Green Button fields.
///
/// Numeric fields remain numeric and timestamps are converted to exact Unix
/// instants. Unsupported fields are ignored.
fn read_text(
    entry: &mut EntryBuilder,
    path: &[String],
    text: &str,
) -> Result<(), InspectionError> {
    if text.is_empty() {
        return Ok(());
    }

    if path_ends_with(path, &["UsagePoint", "ServiceCategory", "kind"]) {
        entry.service_category_kind =
            Some(parse_u32("ServiceCategory.kind", text)?);
    } else if path_ends_with(path, &["ReadingType", "accumulationBehaviour"]) {
        entry.accumulation_behaviour =
            Some(parse_u32("accumulationBehaviour", text)?);
    } else if path_ends_with(path, &["ReadingType", "commodity"]) {
        entry.commodity = Some(parse_u32("commodity", text)?);
    } else if path_ends_with(path, &["ReadingType", "dataQualifier"]) {
        entry.data_qualifier = Some(parse_u32("dataQualifier", text)?);
    } else if path_ends_with(path, &["ReadingType", "flowDirection"]) {
        entry.flow_direction = Some(parse_u32("flowDirection", text)?);
    } else if path_ends_with(path, &["ReadingType", "kind"]) {
        entry.reading_kind = Some(parse_u32("ReadingType.kind", text)?);
    } else if path_ends_with(path, &["ReadingType", "phase"]) {
        entry.phase = Some(parse_u32("phase", text)?);
    } else if path_ends_with(path, &["ReadingType", "powerOfTenMultiplier"]) {
        entry.power_of_ten_multiplier =
            Some(parse_i16("powerOfTenMultiplier", text)?);
    } else if path_ends_with(path, &["ReadingType", "uom"]) {
        entry.unit_of_measure = Some(parse_u32("uom", text)?);
    } else if path_ends_with(path, &["ReadingType", "intervalLength"]) {
        entry.interval_length = Some(parse_u64("intervalLength", text)?);
    } else if path_ends_with(path, &["IntervalBlock", "interval", "start"]) {
        entry.block_start = Some(parse_i64("IntervalBlock.start", text)?);
    } else if path_ends_with(path, &["IntervalBlock", "interval", "duration"]) {
        entry.block_duration = Some(parse_u64("IntervalBlock.duration", text)?);
    } else if path_ends_with(path, &["IntervalReading", "timePeriod", "start"])
        && let Some(reading) = entry.current_reading.as_mut()
    {
        reading.start = Some(parse_i64("IntervalReading.start", text)?);
    } else if path_ends_with(
        path,
        &["IntervalReading", "timePeriod", "duration"],
    ) && let Some(reading) = entry.current_reading.as_mut()
    {
        reading.duration = Some(parse_u32("IntervalReading.duration", text)?);
    } else if path_ends_with(path, &["IntervalReading", "value"])
        && let Some(reading) = entry.current_reading.as_mut()
    {
        reading.value = Some(parse_i64("IntervalReading.value", text)?);
    } else if path_ends_with(
        path,
        &["IntervalReading", "ReadingQuality", "quality"],
    ) && let Some(reading) = entry.current_reading.as_mut()
    {
        reading.quality = Some(parse_u32("ReadingQuality.quality", text)?);
    } else if path_ends_with(path, &["entry", "published"]) {
        entry.published = Some(text.to_string());
    } else if path_ends_with(path, &["entry", "updated"]) {
        entry.updated = Some(text.to_string());
    }

    Ok(())
}

/// Validates and appends the current `IntervalReading` to its parent block.
///
/// Returns [`InspectionError::IncompleteIntervalReading`] when start, duration,
/// or value is absent.
fn finish_interval_reading(
    entry: Option<&mut EntryBuilder>,
) -> Result<(), InspectionError> {
    let Some(entry) = entry else {
        return Ok(());
    };

    let Some(reading) = entry.current_reading.take() else {
        return Ok(());
    };

    let (Some(start), Some(duration), Some(value)) =
        (reading.start, reading.duration, reading.value)
    else {
        return Err(InspectionError::IncompleteIntervalReading);
    };

    entry.readings.push(IntervalReadingRecord {
        start_unix: start,
        duration_seconds: duration,
        value,
        quality: reading.quality,
    });

    Ok(())
}

/// Converts a completed Atom entry builder into one normalized resource.
///
/// Required Green Button identities and IntervalBlock timing fields are
/// validated before the resource is appended to the document.
fn finish_entry(
    entry: EntryBuilder,
    usage_points: &mut Vec<UsagePointRecord>,
    reading_types: &mut Vec<ReadingTypeRecord>,
    meter_readings: &mut Vec<MeterReadingRecord>,
    interval_blocks: &mut Vec<IntervalBlockRecord>,
) -> Result<(), InspectionError> {
    match entry.kind {
        Some(ResourceKind::UsagePoint) => {
            let external_id = entry.self_href.ok_or_else(|| {
                InspectionError::Relationship(
                    "UsagePoint has no self link".to_string(),
                )
            })?;

            usage_points.push(UsagePointRecord {
                external_id,
                related_hrefs: entry.related_hrefs,
                service_category_kind: entry.service_category_kind,
            });
        }

        Some(ResourceKind::ReadingType) => {
            let external_id = entry.self_href.ok_or_else(|| {
                InspectionError::Relationship(
                    "ReadingType has no self link".to_string(),
                )
            })?;

            reading_types.push(ReadingTypeRecord {
                external_id,
                accumulation_behaviour: entry.accumulation_behaviour,
                commodity: entry.commodity,
                data_qualifier: entry.data_qualifier,
                flow_direction: entry.flow_direction,
                kind: entry.reading_kind,
                phase: entry.phase,
                power_of_ten_multiplier: entry.power_of_ten_multiplier,
                unit_of_measure: entry.unit_of_measure,
                interval_length_seconds: entry.interval_length,
            });
        }

        Some(ResourceKind::MeterReading) => {
            let external_id = entry.self_href.ok_or_else(|| {
                InspectionError::Relationship(
                    "MeterReading has no self link".to_string(),
                )
            })?;

            meter_readings.push(MeterReadingRecord {
                external_id,
                up_href: entry.up_href,
                related_hrefs: entry.related_hrefs,
            });
        }

        Some(ResourceKind::IntervalBlock) => {
            let external_id = entry.self_href.ok_or_else(|| {
                InspectionError::Relationship(
                    "IntervalBlock has no self link".to_string(),
                )
            })?;

            let up_href = entry.up_href.ok_or_else(|| {
                InspectionError::Relationship(
                    "IntervalBlock has no up link".to_string(),
                )
            })?;

            let (Some(start_unix), Some(duration_seconds)) =
                (entry.block_start, entry.block_duration)
            else {
                return Err(InspectionError::IncompleteIntervalBlock);
            };

            let published_unix = parse_optional_timestamp_unix(
                "published",
                entry.published.as_deref(),
            )?;

            let updated_unix = parse_optional_timestamp_unix(
                "updated",
                entry.updated.as_deref(),
            )?;

            interval_blocks.push(IntervalBlockRecord {
                external_id,
                up_href,
                start_unix,
                duration_seconds,
                published_unix,
                updated_unix,
                readings: entry.readings,
            });
        }

        None => {}
    }

    Ok(())
}

/// Builds the reporting-only inspection model from a normalized document.
///
/// MeterReading relationships select the applicable ReadingType and associated
/// IntervalBlocks; resources are never selected by file order.
fn build_inspection(
    document: GbDocument,
) -> Result<GbInspection, InspectionError> {
    let reading_type_count = document.reading_types.len();
    let meter_reading_count = document.meter_readings.len();
    let interval_block_count = document.interval_blocks.len();

    let mut summaries = Vec::new();

    for meter_reading in &document.meter_readings {
        let reading_type =
            meter_reading.related_hrefs.iter().find_map(|href| {
                document
                    .reading_types
                    .iter()
                    .find(|record| &record.external_id == href)
            });

        let Some(reading_type) = reading_type else {
            return Err(InspectionError::Relationship(
                "MeterReading has no related ReadingType".to_string(),
            ));
        };

        let commodity = reading_type.commodity.ok_or_else(|| {
            InspectionError::Relationship(
                "ReadingType has no commodity".to_string(),
            )
        })?;

        let mut stats = IntervalStats::default();

        for block in &document.interval_blocks {
            let related = meter_reading
                .related_hrefs
                .iter()
                .any(|href| href == &block.up_href);

            if !related {
                continue;
            }

            for reading in &block.readings {
                stats.add(
                    reading.start_unix,
                    u64::from(reading.duration_seconds),
                );
            }
        }

        summaries.push(MeterReadingSummary {
            commodity: Commodity::from_code(commodity),
            flow_direction: reading_type.flow_direction,
            unit_of_measure: reading_type.unit_of_measure,
            nominal_interval_seconds: reading_type.interval_length_seconds,
            observed_interval_seconds: stats.durations.into_iter().collect(),
            reading_count: stats.reading_count,
            start_unix: stats.start_unix,
            end_unix_exclusive: stats.end_unix_exclusive,
        });
    }

    summaries.sort_by_key(|summary| {
        (
            summary.commodity,
            summary.flow_direction,
            summary.nominal_interval_seconds,
        )
    });

    Ok(GbInspection {
        meter_readings: summaries,
        reading_type_count,
        meter_reading_count,
        interval_block_count,
    })
}

/// Returns whether the current XML element path ends with `suffix`.
fn path_ends_with(path: &[String], suffix: &[&str]) -> bool {
    path.len() >= suffix.len()
        && path[path.len() - suffix.len()..]
            .iter()
            .map(String::as_str)
            .eq(suffix.iter().copied())
}

/// Parses a signed 16-bit Green Button numeric field.
fn parse_i16(field: &'static str, value: &str) -> Result<i16, InspectionError> {
    value.parse().map_err(|_| InspectionError::InvalidNumber {
        field,
        value: value.to_string(),
    })
}

/// Parses an RFC 3339 Atom timestamp and returns Unix seconds.
fn parse_timestamp_unix(
    field: &'static str,
    value: &str,
) -> Result<i64, InspectionError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.timestamp())
        .map_err(|_| InspectionError::InvalidTimestamp {
            field,
            value: value.to_string(),
        })
}

/// Parses an optional RFC 3339 Atom timestamp and returns Unix seconds.
fn parse_optional_timestamp_unix(
    field: &'static str,
    value: Option<&str>,
) -> Result<Option<i64>, InspectionError> {
    value.map(|value| parse_timestamp_unix(field, value)).transpose()
}

/// Parses an unsigned 32-bit Green Button numeric field.
fn parse_u32(field: &'static str, value: &str) -> Result<u32, InspectionError> {
    value.parse().map_err(|_| InspectionError::InvalidNumber {
        field,
        value: value.to_string(),
    })
}

/// Parses an unsigned 64-bit Green Button numeric field.
fn parse_u64(field: &'static str, value: &str) -> Result<u64, InspectionError> {
    value.parse().map_err(|_| InspectionError::InvalidNumber {
        field,
        value: value.to_string(),
    })
}

/// Parses a signed 64-bit Green Button numeric field.
fn parse_i64(field: &'static str, value: &str) -> Result<i64, InspectionError> {
    value.parse().map_err(|_| InspectionError::InvalidNumber {
        field,
        value: value.to_string(),
    })
}
