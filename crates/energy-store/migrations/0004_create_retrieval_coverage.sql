-- SPDX-License-Identifier: MIT

-- ADR-0004: canonical timestamps are UTC.
-- DATETIME(6) is used so MariaDB does not apply session-timezone conversion.

CREATE TABLE retrieval_coverage (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for a requested or observed coverage range',
    acquisition_source_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Acquisition Source for which coverage is being tracked',
    usage_point_id BIGINT UNSIGNED NULL
        COMMENT 'Usage Point covered by this range; NULL means the range applies to the whole source',

    -- MariaDB UNIQUE constraints permit multiple NULL values.
    -- Normalize source-wide coverage (NULL usage_point_id) to zero so the
    -- uniqueness constraint below also protects source-wide ranges.
    usage_point_scope_id BIGINT UNSIGNED
        AS (COALESCE(usage_point_id, 0)) PERSISTENT
        COMMENT 'Generated value used to make source-wide and Usage Point coverage ranges uniquely comparable',

    range_start DATETIME(6) NOT NULL
        COMMENT 'UTC start of the tracked coverage range',
    range_end DATETIME(6) NOT NULL
        COMMENT 'UTC end of the tracked coverage range',

    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this coverage range was created',

    PRIMARY KEY (id),

    CONSTRAINT uq_retrieval_coverage_identity
        UNIQUE (
            acquisition_source_id,
            usage_point_scope_id,
            range_start,
            range_end
        ),

    CONSTRAINT fk_retrieval_coverage_source
        FOREIGN KEY (acquisition_source_id)
        REFERENCES acquisition_source (id),

    CONSTRAINT fk_retrieval_coverage_usage_point
        FOREIGN KEY (usage_point_id)
        REFERENCES usage_point (id),

    CONSTRAINT chk_retrieval_coverage_range
        CHECK (range_end > range_start)
)
COMMENT='Time ranges used to track what data was requested, retrieved, or found unavailable';

CREATE TABLE retrieval_coverage_revision (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for one observed state of a coverage range',
    retrieval_coverage_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Coverage range whose observed state is recorded here',
    ingestion_run_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Ingestion run that observed this coverage state',

    status VARCHAR(32) NOT NULL
        COMMENT 'Observed coverage state: available, partial, or unavailable',
    observed_at DATETIME(6) NOT NULL
        COMMENT 'UTC timestamp when this coverage state was observed',

    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this coverage-state record was created',

    PRIMARY KEY (id),

    CONSTRAINT uq_retrieval_coverage_revision
        UNIQUE (
            retrieval_coverage_id,
            ingestion_run_id
        ),

    CONSTRAINT fk_retrieval_coverage_revision_coverage
        FOREIGN KEY (retrieval_coverage_id)
        REFERENCES retrieval_coverage (id),

    CONSTRAINT fk_retrieval_coverage_revision_run
        FOREIGN KEY (ingestion_run_id)
        REFERENCES ingestion_run (id),

    CONSTRAINT chk_retrieval_coverage_revision_status
        CHECK (
            status IN (
                'available',
                'partial',
                'unavailable'
            )
        )
)
COMMENT='Observed availability state for a tracked retrieval coverage range';

CREATE TABLE retrieval_gap (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT
        COMMENT 'Application-controlled identifier for one unavailable time range',
    retrieval_coverage_revision_id BIGINT UNSIGNED NOT NULL
        COMMENT 'Coverage-state record in which this gap was observed',

    gap_start DATETIME(6) NOT NULL
        COMMENT 'UTC start of the unavailable or missing time range',
    gap_end DATETIME(6) NOT NULL
        COMMENT 'UTC end of the unavailable or missing time range',
    reason VARCHAR(1024) NULL
        COMMENT 'Source or application explanation for why data is unavailable',

    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6)
        COMMENT 'UTC timestamp when this gap record was created',

    PRIMARY KEY (id),

    CONSTRAINT uq_retrieval_gap_identity
        UNIQUE (
            retrieval_coverage_revision_id,
            gap_start,
            gap_end
        ),

    CONSTRAINT fk_retrieval_gap_revision
        FOREIGN KEY (retrieval_coverage_revision_id)
        REFERENCES retrieval_coverage_revision (id),

    CONSTRAINT chk_retrieval_gap_range
        CHECK (gap_end > gap_start)
)
COMMENT='Known missing or unavailable subranges within a tracked coverage range';
