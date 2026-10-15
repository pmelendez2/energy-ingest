<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# Glossary

## Purpose

Provide authoritative terminology for project documentation.

**Intended Audience:** Developers, contributors, maintainers, and users.

This glossary is the single terminology reference for the `energy_ingest` project.

For established energy-industry and Green Button / Energy Services Provider Interface (ESPI) terms, this project does **not** redefine the term.

Instead, each industry term points to an authoritative external source. Any accompanying project note explains only how `energy_ingest` uses the term in project documentation and source code.

Project-specific software terms are defined directly in this document.

## Authoritative Industry and Green Button / ESPI Terms

### Authorized Third Party

Authoritative reference:

Green Button Alliance — Technical Information / Green Button Actors and Terms

https://www.greenbuttonalliance.org/technical-info

Project usage:

Use `Authorized Third Party` only when an `energy_ingest` deployment is actually operating in that Green Button/ESPI role.

Do not use the term as a generic synonym for an external application or integration.

### Community Choice Aggregator (CCA)

Authoritative reference:

California Public Utilities Commission — Community Choice Aggregation

https://www.cpuc.ca.gov/industries-and-topics/electrical-energy/electric-power-procurement/community-choice-aggregation-and-direct-access-/community-choice-aggregation

Project usage:

Use `CCA` when referring to an organization's market, regulatory, or operational role as a Community Choice Aggregator.

If that organization is specifically acting as the Green Button/ESPI Data Custodian for a particular integration, use `Data Custodian` when discussing that ESPI role.

### California Public Utilities Commission (CPUC)

Authoritative reference:

California Public Utilities Commission

https://www.cpuc.ca.gov/

Project usage:

Use `CPUC` when referring to the California regulatory body in that jurisdictional context.

Do not treat CPUC-specific terminology, rules, or responsibilities as generally applicable outside California.

### Connect My Data (CMD)

Authoritative reference:

Green Button Alliance — Green Button / Connect My Data

https://www.greenbuttonalliance.org/green-button

Project usage:

Use `Connect My Data` or `CMD` when referring to the Green Button mechanism for authorized electronic exchange of data between the applicable Green Button actors.

Do not redefine CMD behavior within project documentation.

### Data Custodian

Authoritative reference:

Green Button Alliance — Technical Information / Green Button Actors and Terms

https://www.greenbuttonalliance.org/technical-info

Project usage:

Use `Data Custodian` when referring to the external Green Button/ESPI actor or system in that role.

Do not use the generic term `provider` when `Data Custodian` is the more precise Green Button/ESPI term.

An EDC, utility, CCA, or another organization may act as a Data Custodian depending on the implementation.

### Distributed Energy Resource (DER)

Authoritative reference:

Use the applicable regulator, standards body, or system operator for the jurisdiction and context.

For California-specific usage, authoritative sources may include:

California Public Utilities Commission

https://www.cpuc.ca.gov/

California Independent System Operator

https://www.caiso.com/

Project usage:

Use `DER` only when referring to distributed energy resources in the established energy-industry sense.

Examples may include customer-sited generation, storage, or other qualifying resources, but project documentation should defer to the applicable authoritative definition rather than creating its own.

### Download My Data (DMD)

Authoritative reference:

Green Button Alliance — Green Button / Download My Data

https://www.greenbuttonalliance.org/green-button

Project usage:

Use `Download My Data` or `DMD` when referring to Green Button data made available to a Retail Customer for download.

Within `energy_ingest`, DMD commonly results in Green Button XML acquired as a file for local ingestion.

### Electric Distribution Company (EDC)

Authoritative reference:

Use the applicable jurisdictional or regulatory authority when discussing the formal meaning of an Electric Distribution Company.

For California-specific project work, prefer the California Public Utilities Commission.

https://www.cpuc.ca.gov/

Project usage:

Use `EDC` when discussing the organization's electric-distribution, regulatory, or market role.

If the EDC is acting as the Green Button/ESPI Data Custodian for an integration, use `Data Custodian` when discussing that protocol role.

### Distribution Company / DISCO

Authoritative reference:

Use the applicable market operator, regulator, or industry standard for the jurisdiction being discussed.

Project usage:

`DISCO` may be used where it is the established local or industry abbreviation for a distribution company.

Do not assume `DISCO` and `EDC` are always interchangeable across jurisdictions.

Where the relevant organization is acting in a Green Button/ESPI role, use `Data Custodian` for that protocol context.

### Energy Services Provider Interface (ESPI)

Authoritative reference:

Green Button Alliance — Technical Information

https://www.greenbuttonalliance.org/technical-info

Normative standard:

NAESB REQ.21 Energy Services Provider Interface (ESPI)

Project usage:

Use `ESPI` when referring to the Green Button technical standard, data model, resources, or interactions governed by that standard.

Project documentation must not redefine ESPI semantics.

### Energy Usage Information (EUI)

Authoritative reference:

Green Button Alliance — Technical Information

https://www.greenbuttonalliance.org/technical-info

Normative definition:

NAESB REQ.21 ESPI

Project usage:

Use `Energy Usage Information` when referring to the Green Button/ESPI source-domain concept.

The normalized internal representation used by `energy_ingest` is a project-specific concept and should not be described as redefining EUI.

### Green Button

Authoritative reference:

Green Button Alliance — Green Button

https://www.greenbuttonalliance.org/green-button

PProject usage:

Use Green Button for the established energy-data interchange program.
Provider profiles are local regression configurations, not a declaration
of Green Button Alliance certification or affiliation.

## Project Software and Operational Terms

### Database schema migration

A versioned, controlled change to the MariaDB schema, such as creating a
new table, index, column, or constraint. SQLx manages these changes under
`crates/energy-store/migrations/`. A schema migration is not a backup.

### Data migration

Moving or transforming persisted application data between storage systems
or data models, with appropriate integrity checks. Distinguish this from
schema migration when describing project tasks.

### Deployment migration

Moving application services or durable state between hosts, storage devices,
or deployment environments. For example, relocating container volumes to
another Linux host is deployment migration, not a schema migration.

### Backup

Creating a protected, recoverable representation of selected durable state.
The project does not mandate a backup product (ADR-0023). Backups of live
MariaDB storage must capture a consistent recoverable database state.

### Restore

Reconstructing selected durable state from a backup and checking that the
resulting installation and database are usable. Backup success alone is not
proof of a successful restore.

### Provider profile

A local, versioned configuration of tested Green Button source behavior,
with reviewed golden fixtures and expectations; not a provider whitelist.

### `gb_profile` utility release

A distributed version of the standalone profile CLI. Its SemVer version is
independent of an individual provider profile's revision.
