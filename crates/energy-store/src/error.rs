// SPDX-License-Identifier: MIT

//! Typed errors returned by the `energy-store` persistence boundary.
//!
//! # Purpose
//!
//! Keeps SQLx-specific error details inside the persistence layer and exposes
//! stable error categories to higher application layers.
//!
//! # Maintainer
//!
//! Project Contributors. Individual changes are tracked by Git history.

use thiserror::Error;

/// Errors returned by the `energy-store` persistence layer.
#[derive(Debug, Error)]
pub enum StoreError {
    /// A database operation failed for a reason not represented by a more
    /// specific store error.
    #[error("database operation failed")]
    Database(#[source] sqlx::Error),

    /// A requested record does not exist.
    #[error("requested record was not found")]
    NotFound,

    /// A uniqueness or other data-conflict constraint was violated.
    #[error("record conflicts with existing data")]
    Conflict,
}

/// Converts a SQLx error into a stable `energy-store` error.
///
/// # Purpose
///
/// Prevents callers outside `energy-store` from having to interpret SQLx
/// errors directly.
///
/// # Inputs
///
/// - `error`: SQLx error returned by a database operation.
///
/// # Returns
///
/// - [`StoreError::Conflict`] for recognized duplicate/conflict conditions.
/// - [`StoreError::Database`] for all other SQLx errors.
impl From<sqlx::Error> for StoreError {
    fn from(error: sqlx::Error) -> Self {
        if is_conflict(&error) { Self::Conflict } else { Self::Database(error) }
    }
}

/// Checks whether a SQLx error represents a MariaDB data conflict.
///
/// # Purpose
///
/// Centralizes database-specific conflict-code interpretation.
///
/// # Inputs
///
/// - `error`: SQLx error to inspect.
///
/// # Returns
///
/// Returns `true` when the database error code represents a recognized
/// duplicate/conflict condition; otherwise returns `false`.
fn is_conflict(error: &sqlx::Error) -> bool {
    let sqlx::Error::Database(database_error) = error else {
        return false;
    };

    matches!(database_error.code().as_deref(), Some("1062") | Some("23000"))
}
