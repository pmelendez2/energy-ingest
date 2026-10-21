// SPDX-License-Identifier: MIT

//! Conservative planning for historical-window reconciliation.
//!
//! The input fingerprints must come from authoritative Energy Ingest window
//! reads. This module does not query storage, infer gaps, or write to HA.

use sqlx::types::chrono::NaiveDateTime;
use std::collections::BTreeMap;

/// One half-open UTC window with its authoritative content identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryWindow {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    /// None means the current window cannot safely be published.
    pub fingerprint: Option<String>,
}

/// Work required to bring a consumer's cached windows up to date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryAction {
    Unchanged,
    Refresh,
    Blocked,
}

impl HistoryAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Refresh => "refresh",
            Self::Blocked => "blocked",
        }
    }
}

/// One requested window and its conservative action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryWindowAction {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub action: HistoryAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryPlanError {
    InvalidWindow,
    DuplicateWindow,
    OverlappingWindows,
    MismatchedWindows,
}

/// Compare an optional previous cache against authoritative current windows.
///
/// Previous data may be absent (first synchronization). Both lists must be
/// sorted or sortable into *identical* nonoverlapping window partitions when
/// previous data exists. Changes are never silently ignored. A current
/// unresolved window blocks publication even if its old fingerprint matches.
///
/// No window is truncated, merged, backfilled, or interpolated here.
pub fn plan_history_refresh(
    previous: Option<&[HistoryWindow]>,
    current: &[HistoryWindow],
) -> Result<Vec<HistoryWindowAction>, HistoryPlanError> {
    fn index(
        windows: &[HistoryWindow],
    ) -> Result<
        BTreeMap<(NaiveDateTime, NaiveDateTime), &HistoryWindow>,
        HistoryPlanError,
    > {
        let mut indexed = BTreeMap::new();
        let mut last_end = None;
        let mut ordered: Vec<_> = windows.iter().collect();
        ordered.sort_by_key(|w| (w.start, w.end));
        for window in ordered {
            if window.start >= window.end {
                return Err(HistoryPlanError::InvalidWindow);
            }
            if last_end.is_some_and(|end| window.start < end) {
                if indexed.contains_key(&(window.start, window.end)) {
                    return Err(HistoryPlanError::DuplicateWindow);
                }
                return Err(HistoryPlanError::OverlappingWindows);
            }
            if indexed.insert((window.start, window.end), window).is_some() {
                return Err(HistoryPlanError::DuplicateWindow);
            }
            last_end = Some(window.end);
        }
        Ok(indexed)
    }

    let current = index(current)?;
    let previous = previous.map(index).transpose()?;
    if let Some(old) = &previous
        && old.keys().ne(current.keys())
    {
        return Err(HistoryPlanError::MismatchedWindows);
    }

    Ok(current
        .iter()
        .map(|(&(start, end), new)| {
            let action = match &new.fingerprint {
                None => HistoryAction::Blocked,
                Some(fingerprint) => match previous
                    .as_ref()
                    .and_then(|old| old.get(&(start, end)))
                    .and_then(|old| old.fingerprint.as_ref())
                {
                    Some(old) if old == fingerprint => HistoryAction::Unchanged,
                    _ => HistoryAction::Refresh,
                },
            };
            HistoryWindowAction { start, end, action }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: u32) -> NaiveDateTime {
        sqlx::types::chrono::NaiveDate::from_ymd_opt(2026, 10, day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn window(
        start: u32,
        end: u32,
        fingerprint: Option<&str>,
    ) -> HistoryWindow {
        HistoryWindow {
            start: at(start),
            end: at(end),
            fingerprint: fingerprint.map(str::to_owned),
        }
    }

    #[test]
    fn initial_sync_refreshes_only_resolved_windows() {
        let current = [window(1, 2, Some("a")), window(2, 3, None)];
        let actions = plan_history_refresh(None, &current).unwrap();
        assert_eq!(actions[0].action, HistoryAction::Refresh);
        assert_eq!(actions[1].action, HistoryAction::Blocked);
    }

    #[test]
    fn unchanged_fingerprint_skips_refresh() {
        let old = [window(1, 2, Some("a"))];
        let current = [window(1, 2, Some("a"))];
        let actions = plan_history_refresh(Some(&old), &current).unwrap();
        assert_eq!(actions[0].action, HistoryAction::Unchanged);
    }

    #[test]
    fn revision_change_requires_refresh() {
        let old = [window(1, 2, Some("a"))];
        let current = [window(1, 2, Some("b"))];
        let actions = plan_history_refresh(Some(&old), &current).unwrap();
        assert_eq!(actions[0].action, HistoryAction::Refresh);
    }

    #[test]
    fn previously_blocked_window_can_recover() {
        let old = [window(1, 2, None)];
        let current = [window(1, 2, Some("b"))];
        let actions = plan_history_refresh(Some(&old), &current).unwrap();
        assert_eq!(actions[0].action, HistoryAction::Refresh);
    }

    #[test]
    fn current_blocked_window_never_reuses_old_data() {
        let old = [window(1, 2, Some("a"))];
        let current = [window(1, 2, None)];
        let actions = plan_history_refresh(Some(&old), &current).unwrap();
        assert_eq!(actions[0].action, HistoryAction::Blocked);
    }

    #[test]
    fn order_does_not_affect_actions() {
        let current = [window(2, 3, Some("b")), window(1, 2, Some("a"))];
        let actions = plan_history_refresh(None, &current).unwrap();
        assert_eq!(actions[0].start, at(1));
        assert_eq!(actions[1].start, at(2));
    }

    #[test]
    fn rejects_duplicate_partition() {
        let current = [window(1, 2, Some("a")), window(1, 2, Some("a"))];
        assert_eq!(
            plan_history_refresh(None, &current),
            Err(HistoryPlanError::DuplicateWindow)
        );
    }

    #[test]
    fn rejects_overlap() {
        let current = [window(1, 3, Some("a")), window(2, 4, Some("b"))];
        assert_eq!(
            plan_history_refresh(None, &current),
            Err(HistoryPlanError::OverlappingWindows)
        );
    }

    #[test]
    fn rejects_invalid_bounds() {
        assert_eq!(
            plan_history_refresh(None, &[window(2, 2, None)]),
            Err(HistoryPlanError::InvalidWindow)
        );
    }

    #[test]
    fn rejects_changed_partition_instead_of_guessing() {
        let old = [window(1, 3, Some("a"))];
        let current = [window(1, 2, Some("a")), window(2, 3, Some("a"))];
        assert_eq!(
            plan_history_refresh(Some(&old), &current),
            Err(HistoryPlanError::MismatchedWindows)
        );
    }

    #[test]
    fn empty_partitions_have_no_actions() {
        assert!(plan_history_refresh(None, &[]).unwrap().is_empty());
    }
}
