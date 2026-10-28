-- SPDX-License-Identifier: MIT
-- Identical utility documents retain the first successful import identity.
CREATE TABLE import_document (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    acquisition_source_id BIGINT UNSIGNED NOT NULL,
    sha256_hex CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    first_ingestion_run_id BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6),
    PRIMARY KEY (id),
    UNIQUE KEY uq_import_document_source_hash (
        acquisition_source_id, sha256_hex
    ),
    CONSTRAINT fk_import_document_source FOREIGN KEY (
        acquisition_source_id
    ) REFERENCES acquisition_source (id),
    CONSTRAINT fk_import_document_run FOREIGN KEY (
        first_ingestion_run_id
    ) REFERENCES ingestion_run (id)
) ENGINE=InnoDB;
