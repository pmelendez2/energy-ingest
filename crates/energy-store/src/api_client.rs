// SPDX-License-Identifier: MIT

//! Local API client credential lifecycle (ADR-0024).
//! Plaintext tokens are returned only at creation and never persisted.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use sqlx::{MySql, Pool};

use crate::StoreError;

const TOKEN_PREFIX: &str = "ei_";
const TOKEN_BYTES: usize = 32;
const ENCODED_BYTES: usize = 43;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiClient {
    pub id: u64,
    pub name: String,
    pub created_at: chrono::NaiveDateTime,
    pub revoked_at: Option<chrono::NaiveDateTime>,
}

/// The caller must securely deliver the token; it cannot be retrieved later.
/// Avoid debug-printing this structure or writing tokens to log files.
pub struct IssuedApiToken {
    pub client_id: u64,
    pub token: String,
}

fn token_digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

fn valid_format(token: &str) -> bool {
    let Some(encoded) = token.strip_prefix(TOKEN_PREFIX) else {
        return false;
    };
    if encoded.len() != ENCODED_BYTES {
        return false;
    }
    let Ok(bytes) = URL_SAFE_NO_PAD.decode(encoded) else {
        return false;
    };
    bytes.len() == TOKEN_BYTES && URL_SAFE_NO_PAD.encode(bytes) == encoded
}

/// Generates a 256-bit cryptographically random credential.
fn generate_token() -> Result<String, getrandom::Error> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes)?;
    Ok(format!("{TOKEN_PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes)))
}

/// Creates a client and returns its plaintext token exactly once.
pub async fn create_api_client(
    pool: &Pool<MySql>,
    name: &str,
) -> Result<IssuedApiToken, StoreError> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > 255
        || name.chars().any(char::is_control)
    {
        return Err(StoreError::Conflict);
    }
    let token = generate_token().map_err(|_| StoreError::Conflict)?;
    let digest = token_digest(&token);
    let result = sqlx::query(
        "INSERT INTO api_client (name, token_verifier) VALUES (?, ?)",
    )
    .bind(name)
    .bind(&digest[..])
    .execute(pool)
    .await?;
    Ok(IssuedApiToken { client_id: result.last_insert_id(), token })
}

/// Returns metadata without token values or verifiers.
pub async fn list_api_clients(
    pool: &Pool<MySql>,
) -> Result<Vec<ApiClient>, StoreError> {
    Ok(sqlx::query_as::<_, ApiClient>(
        "SELECT id, name, created_at, revoked_at \
         FROM api_client ORDER BY id",
    )
    .fetch_all(pool)
    .await?)
}

/// Permanently revokes a credential; repeat revocation is idempotent.
pub async fn revoke_api_client(
    pool: &Pool<MySql>,
    id: u64,
) -> Result<bool, StoreError> {
    let result = sqlx::query(
        "UPDATE api_client SET revoked_at = UTC_TIMESTAMP(6) \
         WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() != 0)
}

/// Validates the complete bearer token and checks revocation state.
/// Never logs the token or returns a stored verifier.
pub async fn authenticate_api_token(
    pool: &Pool<MySql>,
    token: &str,
) -> Result<Option<u64>, StoreError> {
    if !valid_format(token) {
        return Ok(None);
    }
    let digest = token_digest(token);
    let row: Option<(u64,)> = sqlx::query_as(
        "SELECT id FROM api_client \
         WHERE token_verifier = ? AND revoked_at IS NULL",
    )
    .bind(&digest[..])
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id,)| id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_token_has_correct_entropy_and_encoding() {
        let first = generate_token().unwrap();
        let second = generate_token().unwrap();
        assert!(valid_format(&first));
        assert_ne!(first, second);
        assert_eq!(first.len(), TOKEN_PREFIX.len() + ENCODED_BYTES);
    }

    #[test]
    fn rejects_malformed_and_noncanonical_tokens() {
        assert!(!valid_format(""));
        assert!(!valid_format("ei_incorrect"));
        assert!(!valid_format(&format!("ei_{}", "a".repeat(43))));
        let token = generate_token().unwrap();
        assert!(!valid_format(&format!("{token}=")));
    }

    #[test]
    fn verifier_is_hash_of_entire_token() {
        let token = generate_token().unwrap();
        assert_ne!(token_digest(&token), token_digest(&token[3..]));
    }
}
