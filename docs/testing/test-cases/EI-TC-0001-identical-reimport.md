<!-- AI provenance: annotated 2026-10-08; origin not asserted. -->

# EI-TC-0001: Re-import identical source data

## Objective

Verify that importing identical energy data more than once does not create
duplicate interval records.

## Preconditions

- Empty test database.
- Known deterministic test fixture.
- Importer configured for file input.

## Procedure

1. Import the fixture.
2. Record the number of stored interval records.
3. Import the same fixture again.
4. Query the stored interval count and import history.

## Expected result

- No duplicate interval records are created.
- The stored interval count remains unchanged.
- The repeated import completes without corrupting existing data.
- Import history identifies the repeated input appropriately.
