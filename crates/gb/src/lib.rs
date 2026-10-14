// SPDX-License-Identifier: MIT

//! Green Button parsing and normalization support for `energy_ingest`.
//!
//! # Purpose
//!
//! Provides Green Button-specific parsing and data conversion while keeping
//! provider acquisition and persistent storage concerns in separate crates.
//!
//! # Architecture
//!
//! This crate owns Green Button / ESPI XML parsing and normalized source
//! records. Canonical interval identity, revisions, and MariaDB persistence
//! remain responsibilities of `energy-store`.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

mod inspect;

pub use inspect::{
    Commodity, GbDocument, GbInspection, InspectionError, IntervalBlockRecord,
    IntervalReadingRecord, MeterReadingRecord, MeterReadingSummary,
    ReadingTypeRecord, UsagePointRecord, inspect_file, inspect_reader,
    parse_file, parse_reader,
};

pub mod profile;
