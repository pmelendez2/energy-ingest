-- SPDX-License-Identifier: MIT

-- Customer-defined meter identity is distinct from Green Button identity.
-- Original source records and accepted revisions remain unchanged.
CREATE TABLE physical_meter (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    data_custodian_id BIGINT UNSIGNED NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    meter_number VARCHAR(255) NULL,
    commodity INT UNSIGNED NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6),
    PRIMARY KEY (id),
    UNIQUE KEY uq_physical_meter_name (
        data_custodian_id, display_name
    ),
    CONSTRAINT fk_physical_meter_custodian
        FOREIGN KEY (data_custodian_id)
        REFERENCES data_custodian (id),
    CONSTRAINT chk_physical_meter_name
        CHECK (CHAR_LENGTH(TRIM(display_name)) > 0)
) ENGINE=InnoDB
  COMMENT='Customer-maintained physical meter profiles';

CREATE TABLE physical_meter_resource (
    physical_meter_id BIGINT UNSIGNED NOT NULL,
    meter_reading_id BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT UTC_TIMESTAMP(6),
    PRIMARY KEY (meter_reading_id),
    KEY ix_physical_meter_resource_meter (physical_meter_id),
    CONSTRAINT fk_meter_resource_profile
        FOREIGN KEY (physical_meter_id)
        REFERENCES physical_meter (id),
    CONSTRAINT fk_meter_resource_reading
        FOREIGN KEY (meter_reading_id)
        REFERENCES meter_reading (id)
) ENGINE=InnoDB
  COMMENT='Explicitly approved physical meter to resource mapping';
