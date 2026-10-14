-- SPDX-License-Identifier: MIT

-- ADR-0022: foundational identities and ingestion source tracking.
-- ADR-0004: canonical timestamps are UTC.
-- DATETIME(6) is used so MariaDB does not apply session-timezone conversion.

CREATE TABLE data_custodian (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for the Data Custodian',
    name VARCHAR(255) NOT NULL
        COMMENT 'Human-readable name of the organization responsible for source data',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this record was created',

    PRIMARY KEY (id),
    UNIQUE KEY uq_data_custodian_name (name)
) ENGINE=InnoDB
  COMMENT='Organizations responsible for one or more energy data sources';

CREATE TABLE acquisition_source (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for the acquisition source',
    data_custodian_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Data Custodian responsible for this source',
    name VARCHAR(255) NOT NULL
        COMMENT 'Human-readable name of the source from which energy data is acquired',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this record was created',

    PRIMARY KEY (id),

    UNIQUE KEY uq_acquisition_source_name (
        data_custodian_id,
        name
    ),

    CONSTRAINT fk_acquisition_source_data_custodian
        FOREIGN KEY (data_custodian_id)
        REFERENCES data_custodian (id)
) ENGINE=InnoDB
  COMMENT='Named sources from which energy data is acquired';

CREATE TABLE ingestion_run (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for one ingestion run',
    acquisition_source_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Acquisition Source processed by this ingestion run',
    started_at DATETIME(6) NOT NULL
        COMMENT 'UTC timestamp when ingestion started',
    completed_at DATETIME(6) NULL
        COMMENT 'UTC timestamp when ingestion completed; NULL while incomplete or unknown',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this database record was created',

    PRIMARY KEY (id),

    KEY ix_ingestion_run_source_started (
        acquisition_source_id,
        started_at
    ),

    CONSTRAINT fk_ingestion_run_acquisition_source
        FOREIGN KEY (acquisition_source_id)
        REFERENCES acquisition_source (id)
) ENGINE=InnoDB
  COMMENT='Execution history for energy data ingestion from an acquisition source';
