// SPDX-License-Identifier: MIT

//! Stable version fingerprint for one successfully resolved meter window.
//! A fingerprint does not provide a globally ordered change cursor.

use crate::SelectedInterval;
use chrono::NaiveDateTime;
use sha2::{Digest, Sha256};

/// Hash a meter/window and the canonical selected observations.
///
/// The caller must first reject any unresolved readings or oversized
/// windows. An empty, resolved window is a valid distinct snapshot.
/// The hash covers returned observations, not changes to unrelated data.
pub fn window_fingerprint(
    meter_id: u64,
    start: NaiveDateTime,
    end: NaiveDateTime,
    intervals: &[SelectedInterval],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"energy-ingest-window-v1\0");
    hasher.update(meter_id.to_be_bytes());
    hasher.update(start.and_utc().timestamp().to_be_bytes());
    hasher.update(end.and_utc().timestamp().to_be_bytes());

    // The resolver already returns chronologically ordered intervals.
    // Sort again to make the hash independent of input vector order.
    let mut selected = intervals.to_vec();
    selected.sort_by_key(|i| (i.start, i.duration_seconds, i.meter_reading_id));
    hasher.update((selected.len() as u64).to_be_bytes());
    for item in &selected {
        hasher.update(item.start.and_utc().timestamp().to_be_bytes());
        hasher.update(item.duration_seconds.to_be_bytes());
        hasher.update(item.raw_value.to_be_bytes());
        hasher.update(item.meter_reading_id.to_be_bytes());
        hasher.update(item.accepted_revision_id.to_be_bytes());
        hasher.update(item.power_of_ten_multiplier.to_be_bytes());
        hasher.update(item.unit_of_measure.to_be_bytes());
    }
    let digest = hasher.finalize();
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("write to String");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .expect("valid time")
    }

    fn item() -> SelectedInterval {
        SelectedInterval {
            start: dt("2026-01-01T00:00:00"),
            duration_seconds: 3600,
            raw_value: 1000,
            meter_reading_id: 5,
            accepted_revision_id: 12,
            power_of_ten_multiplier: 0,
            unit_of_measure: 72,
        }
    }

    fn fingerprint(items: &[SelectedInterval]) -> String {
        window_fingerprint(
            1,
            dt("2026-01-01T00:00:00"),
            dt("2026-01-02T00:00:00"),
            items,
        )
    }

    #[test]
    fn repeatable_and_order_independent() {
        let a = item();
        let mut b = item();
        b.start = dt("2026-01-01T01:00:00");
        assert_eq!(fingerprint(&[a.clone(), b.clone()]), fingerprint(&[b, a]));
        assert_eq!(fingerprint(&[item()]), fingerprint(&[item()]));
    }

    #[test]
    fn corrected_revision_or_value_changes_fingerprint() {
        let original = fingerprint(&[item()]);
        let mut changed = item();
        changed.accepted_revision_id += 1;
        assert_ne!(original, fingerprint(&[changed]));
        let mut changed = item();
        changed.raw_value += 1;
        assert_ne!(original, fingerprint(&[changed]));
    }

    #[test]
    fn meter_window_and_units_are_part_of_identity() {
        let start = dt("2026-01-01T00:00:00");
        let end = dt("2026-01-02T00:00:00");
        let baseline = fingerprint(&[item()]);
        assert_ne!(baseline, window_fingerprint(2, start, end, &[item()]));
        assert_ne!(
            baseline,
            window_fingerprint(1, start, dt("2026-01-03T00:00:00"), &[item()])
        );
        let mut changed = item();
        changed.unit_of_measure = 169;
        assert_ne!(baseline, fingerprint(&[changed]));
    }

    #[test]
    fn empty_window_has_stable_fingerprint() {
        assert_eq!(fingerprint(&[]), fingerprint(&[]));
        assert_eq!(fingerprint(&[]).len(), 64);
        assert_ne!(fingerprint(&[]), fingerprint(&[item()]));
    }
}
