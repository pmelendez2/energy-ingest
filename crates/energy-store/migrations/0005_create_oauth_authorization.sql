-- SPDX-License-Identifier: MIT

CREATE TABLE oauth_authorization (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled OAuth authorization identifier.',

    acquisition_source_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Acquisition Source that owns this residential OAuth authorization.',

    external_authorization_id VARCHAR(1024) NULL
        COMMENT 'Optional authorization identifier supplied by the Data Custodian.',

    resource_owner_external_id VARCHAR(1024) NULL
        COMMENT 'Optional Data Custodian identifier for the authorized resource owner or account.',

    status VARCHAR(64) NOT NULL
        COMMENT 'Current application authorization state, such as active or reauthorization required.',

    token_type VARCHAR(64) NULL
        COMMENT 'OAuth token type reported by the Data Custodian, when available.',

    granted_scopes TEXT NULL
        COMMENT 'Scopes actually granted by the Data Custodian, when available.',

    access_token TEXT NULL
        COMMENT 'Current sensitive OAuth access token, when persisted.',

    refresh_token TEXT NULL
        COMMENT 'Current sensitive OAuth refresh token, when issued and persisted.',

    access_token_expires_at DATETIME(6) NULL
        COMMENT 'UTC expiration timestamp for the current access token.',

    authorized_at DATETIME(6) NULL
        COMMENT 'UTC timestamp when the current authorization was established.',

    last_refreshed_at DATETIME(6) NULL
        COMMENT 'UTC timestamp of the most recent successful token refresh.',

    last_successful_use_at DATETIME(6) NULL
        COMMENT 'UTC timestamp of the most recent successful authorized Data Custodian access.',

    revoked_at DATETIME(6) NULL
        COMMENT 'UTC timestamp when authorization was known to be revoked, when applicable.',

    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this authorization row was created.',

    updated_at DATETIME(6) NOT NULL
        DEFAULT CURRENT_TIMESTAMP(6)
        ON UPDATE CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this authorization row was last updated.',

    PRIMARY KEY (id),

    CONSTRAINT uq_oauth_authorization_source
        UNIQUE (acquisition_source_id),

    CONSTRAINT fk_oauth_authorization_source
        FOREIGN KEY (acquisition_source_id)
        REFERENCES acquisition_source (id)
)
COMMENT='Current mutable OAuth authorization state for one residential Acquisition Source.';
