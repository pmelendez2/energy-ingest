# Meter resource association verification

These commands apply to an existing Energy Usage installation with
registered physical meters and imported Green Button observations.
They do not require resetting the database or reimporting XML.

From `deploy/docker-compose`, review the profiles:

```bash
bash meter-prod.sh list
bash analyze-prod-data.sh
```

For a single electricity meter profile, preview resources before
changing associations:

```bash
bash meter-prod.sh attach-compatible --meter 1
```

Answer `n` at the confirmation prompt and verify the existing count
is unchanged with `bash meter-prod.sh list`.

Repeat the command and approve the eligible resources after reviewing
the preview. List the associations and run the operation again to test
idempotency:

```bash
bash meter-prod.sh resources --meter 1
bash meter-prod.sh list
bash meter-prod.sh attach-compatible --meter 1
```

Repeat for the gas profile using its assigned profile ID. Do not
assume profile numbers are the same in another installation.

For the existing SDG&E dataset used in verification, the expected
final associations are six electric resources and four gas resources.
The original 30,030 observations must remain unchanged. The analysis
of overlapping data is a separate operation and must not be interpreted
as confirmation of physical meter identity.

**Regression cases:** no eligible resources, re-running an approved
association, multiple meters of the same commodity, resources already
owned by another profile, invalid meter IDs, and confirmation declined.
The database-level transactional behavior must be validated on the
running MariaDB installation.
