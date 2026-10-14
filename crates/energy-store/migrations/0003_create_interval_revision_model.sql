-- SPDX-License-Identifier: MIT

-- ADR-0004: canonical timestamps are UTC.
-- DATETIME(6) is used so MariaDB does not apply session-timezone conversion.

CREATE TABLE interval_reading (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for one canonical interval reading',
    meter_reading_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Meter Reading to which this interval reading belongs',
    interval_start DATETIME(6) NOT NULL
        COMMENT 'UTC start timestamp of this interval',
    interval_duration_seconds INT UNSIGNED NOT NULL
        COMMENT 'Length of this interval in seconds',
    accepted_revision_id BIGINT UNSIGNED NULL
        COMMENT 'Revision currently accepted as the value for this interval reading',
    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this canonical interval reading was created',

    PRIMARY KEY (id),

    CONSTRAINT uq_interval_reading_identity
        UNIQUE (
            meter_reading_id,
            interval_start,
            interval_duration_seconds
        ),

    CONSTRAINT fk_interval_reading_meter_reading
        FOREIGN KEY (meter_reading_id)
        REFERENCES meter_reading (id),

    CONSTRAINT chk_interval_reading_duration
        CHECK (interval_duration_seconds > 0)
)
COMMENT='Canonical interval identities used to keep reading values stable across re-imports';

CREATE TABLE interval_reading_revision (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for one observed reading revision',
    interval_reading_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Canonical interval reading represented by this revision',
    ingestion_run_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Ingestion run that observed this revision',
    interval_block_id BIGINT UNSIGNED NULL
        COMMENT 'Source Interval Block containing this revision when available',

    source_external_id VARCHAR(1024) NULL
        COMMENT 'Source-provided identifier for this reading revision when available',
    source_published_at DATETIME(6) NULL
        COMMENT 'UTC source publication timestamp when supplied',
    source_updated_at DATETIME(6) NULL
        COMMENT 'UTC source update timestamp when supplied',

    reading_value BIGINT NOT NULL
        COMMENT 'Raw Green Button reading value before applying the Reading Type multiplier',
    reading_quality VARCHAR(255) NULL
        COMMENT 'Source-provided reading quality or quality description when available',

    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this revision was stored',

    PRIMARY KEY (id),

    -- Required by the composite accepted-revision foreign key below.
    CONSTRAINT uq_interval_reading_revision_identity
        UNIQUE (id, interval_reading_id),

    CONSTRAINT fk_interval_revision_reading
        FOREIGN KEY (interval_reading_id)
        REFERENCES interval_reading (id),

    CONSTRAINT fk_interval_revision_ingestion_run
        FOREIGN KEY (ingestion_run_id)
        REFERENCES ingestion_run (id),

    CONSTRAINT fk_interval_revision_block
        FOREIGN KEY (interval_block_id)
        REFERENCES interval_block (id)
)
COMMENT='Observed versions of interval reading values retained across ingestion runs';

-- A canonical interval reading may only select a revision that belongs
-- to that same canonical reading.
ALTER TABLE interval_reading
    ADD CONSTRAINT fk_interval_reading_accepted_revision
        FOREIGN KEY (accepted_revision_id, id)
        REFERENCES interval_reading_revision (id, interval_reading_id);
