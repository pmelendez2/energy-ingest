-- SPDX-License-Identifier: MIT

-- ADR-0022 canonical Green Button relational model.
--
-- Application-controlled numeric identities are primary keys.
-- Data Custodian identifiers are preserved as external_id values scoped
-- to the Acquisition Source that supplied them.
--
-- ADR-0004: canonical timestamps are UTC.
-- DATETIME(6) is used so MariaDB does not apply session-timezone conversion.

CREATE TABLE usage_point (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for the Usage Point',
    acquisition_source_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Acquisition Source that supplied this Usage Point',
    external_id VARCHAR(1024) NOT NULL
        COMMENT 'Usage Point identifier supplied by the acquisition source',
    service_category_kind INT UNSIGNED NULL
        COMMENT 'Green Button numeric service-category code, such as electricity or gas',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this record was created',

    PRIMARY KEY (id),

    CONSTRAINT uq_usage_point_source_external
        UNIQUE (acquisition_source_id, external_id),

    CONSTRAINT fk_usage_point_source
        FOREIGN KEY (acquisition_source_id)
        REFERENCES acquisition_source (id)
)
COMMENT='Green Button Usage Points supplied by acquisition sources';

CREATE TABLE reading_type (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for the Reading Type',
    acquisition_source_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Acquisition Source that supplied this Reading Type',
    external_id VARCHAR(1024) NOT NULL
        COMMENT 'Reading Type identifier supplied by the acquisition source',

    accumulation_behaviour INT UNSIGNED NULL
        COMMENT 'Green Button numeric code describing how interval values accumulate',
    commodity INT UNSIGNED NULL
        COMMENT 'Green Button numeric code identifying the measured commodity',
    data_qualifier INT UNSIGNED NULL
        COMMENT 'Green Button numeric code describing how the reading value is qualified',
    flow_direction INT UNSIGNED NULL
        COMMENT 'Green Button numeric code describing the direction of energy flow',
    kind INT UNSIGNED NULL
        COMMENT 'Green Button numeric code describing the measured quantity',
    phase INT UNSIGNED NULL
        COMMENT 'Green Button numeric code identifying the electrical phase or phase combination',
    power_of_ten_multiplier SMALLINT NULL
        COMMENT 'Base-10 exponent applied to stored reading values',
    unit_of_measure INT UNSIGNED NULL
        COMMENT 'Green Button numeric code identifying the reading unit of measure',

    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this record was created',

    PRIMARY KEY (id),

    CONSTRAINT uq_reading_type_source_external
        UNIQUE (acquisition_source_id, external_id),

    CONSTRAINT fk_reading_type_source
        FOREIGN KEY (acquisition_source_id)
        REFERENCES acquisition_source (id)
)
COMMENT='Green Button metadata describing the meaning and units of meter readings';

CREATE TABLE meter_reading (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for the Meter Reading',
    usage_point_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Usage Point to which this Meter Reading belongs',
    reading_type_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Reading Type that defines the meaning and units of this Meter Reading',
    external_id VARCHAR(1024) NOT NULL
        COMMENT 'Meter Reading identifier supplied by the acquisition source',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this record was created',

    PRIMARY KEY (id),

    CONSTRAINT uq_meter_reading_usage_external
        UNIQUE (usage_point_id, external_id),

    CONSTRAINT fk_meter_reading_usage_point
        FOREIGN KEY (usage_point_id)
        REFERENCES usage_point (id),

    CONSTRAINT fk_meter_reading_reading_type
        FOREIGN KEY (reading_type_id)
        REFERENCES reading_type (id)
)
COMMENT='Green Button Meter Reading resources linking a Usage Point to a Reading Type';

CREATE TABLE interval_block (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for the Interval Block',
    meter_reading_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Meter Reading that owns this Interval Block',
    external_id VARCHAR(1024) NOT NULL
        COMMENT 'Interval Block identifier supplied by the acquisition source',

    interval_start DATETIME(6) NOT NULL
        COMMENT 'UTC start timestamp of the time range represented by this block',
    interval_duration_seconds BIGINT UNSIGNED NOT NULL
        COMMENT 'Length of the Interval Block time range in seconds',

    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this record was created',

    PRIMARY KEY (id),

    CONSTRAINT uq_interval_block_meter_external
        UNIQUE (meter_reading_id, external_id),

    CONSTRAINT fk_interval_block_meter_reading
        FOREIGN KEY (meter_reading_id)
        REFERENCES meter_reading (id),

    CONSTRAINT chk_interval_block_duration
        CHECK (interval_duration_seconds > 0)
)
COMMENT='Green Button containers that group interval readings for a Meter Reading';
