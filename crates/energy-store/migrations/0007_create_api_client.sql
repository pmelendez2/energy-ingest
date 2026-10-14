-- SPDX-License-Identifier: MIT

-- The SHA-256 verifier is stored, never the plaintext bearer token.
CREATE TABLE api_client (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    name VARCHAR(255) NOT NULL,
    token_verifier BINARY(32) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6),
    revoked_at DATETIME(6) NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_api_client_name (name),
    UNIQUE KEY uq_api_client_verifier (token_verifier),
    CONSTRAINT chk_api_client_name
        CHECK (CHAR_LENGTH(TRIM(name)) > 0)
) ENGINE=InnoDB
  COMMENT='Independently revocable local API client credentials';
