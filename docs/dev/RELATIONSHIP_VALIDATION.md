# Offline XML Relationship Inspection

**Audience:** EDC/CCA profile contributors and maintainers.

**Purpose:** Confirm that representative XML feeds retain useful resource
relationships without contacting an EDC/CCA or disclosing identifiers.

Run from the `energy_ingest` workspace:

```bash
cargo run -p fixture_privacy -- relationships \
  profiles/sdge/fixtures
```

The Rust inspector reads XML files locally. It associates each Atom
`entry` identifier with that entry's `rel="self"` URL. Both forms can
then resolve `up`, `related`, or other references from the same set.
An absolute reference absent from the set is **out-of-set**, not
necessarily broken: partial provider feeds are common.

The JSON report contains aggregate counts only:

- `identifiers`: distinct Atom entry IDs.
- `references`: link attributes inspected.
- `resolved_references`: references matching an entry ID or its self URL.
- `external_references`: references not represented in this set.
- `unresolved_local_references`: local fragments with no entry ID match.
- `duplicate_identifiers`: IDs encountered in multiple entries.
- `conflicting_identifiers`: one ID paired with multiple self URLs.
- `conflicting_self_links`: one self URL paired with multiple entry IDs.

Duplicates can be normal when feeds overlap; conflicting associations
require review. This is a structural audit, not a proof of Green Button
semantic correctness, complete coverage, or privacy clearance.

The inspector does not repair XML, rewrite provider URLs, approve fixtures,
or generate publication authorization. A future before/after check must
also validate graph consistency across sanitized candidates before sharing.

## Conflict classification and before/after comparison

The Rust report distinguishes `repeated_resource_pairs` (the same Atom ID
and `self` URL in more than one entry), `self_urls_with_multiple_ids`
(multiple Atom IDs for one URL), and `ids_with_multiple_self_urls`
(one Atom ID associated with different URLs). Overlapping EDC exports may
legitimately repeat resources; the latter two categories need human review.

`compare_relationships(before, after)` compares aggregate counts across
private input and prepared output sets without serializing identifiers or
URLs. `structural_counts_match` detects simple changes; it **does not prove
semantic graph preservation**. The result always sets
`regression_review_required` because validating changed identifiers needs
private original-to-synthetic correspondence and Green Button-specific
semantics. Do not use this report as publication approval.


### SDG&E shared-URL investigation (unconfirmed)

The ten original SDG&E XML downloads and ten previously derived XML
fixtures both report six `self` URLs each paired with multiple Atom
entry IDs (`self_urls_with_multiple_ids: 6`). This is an observed
inconsistency, **not a confirmed SDG&E implementation defect**.

The `conflicting_self_link_types` object groups those conflicting URLs
by recognized Green Button resource type. Only an allowlisted type
name or `other` is serialized; no URLs, account data, or Atom IDs are
included. Treat these as diagnostic classifications, not proof of the
provider's resource semantics. The detector assumes a conventional
`/<ResourceType>/<ResourceId>` URL path; unusual layouts fall under
`other`. Verify the parser's interpretation and the applicable resource
identity requirements before contacting a provider.

`compare_relationships` also compares category counts, but it still
cannot establish one-to-one graph preservation through sanitization.
Private original-to-synthetic correspondence and exact edge checks
remain required before fixture publication.
