# Physical meter profiles

Energy Usage keeps the utility's imported Green Button resources separate
from customer-defined physical meters. An optional friendly name identifies
each physical meter, while the imported observations remain unchanged.

## Create profiles

After registering the utility and importing XML, run from
`deploy/docker-compose`:

```bash
bash meter-prod.sh create \
  --kind electric --name "Electric"

bash meter-prod.sh create \
  --kind gas --name "Gas"
```

Profiles are optional and can be configured after importing data.
When there is one registered utility, the command selects it. When
multiple utilities are registered, provide `--custodian ID` explicitly.

List profiles using `bash meter-prod.sh list`.

## Inspect imported resources

Use `bash analyze-prod-data.sh --details` for resource IDs and compatible
measurement characteristics. IDs are diagnostic database identifiers, not
customer-visible physical meter numbers.

Attach an individual resource after verifying its identity:

```bash
bash meter-prod.sh attach --meter "<METER_PROFILE_ID>" \
  --resource "<RESOURCE_ID>"
```

Each imported resource can belong to at most one physical meter profile.
The application rejects incompatible utility and commodity associations.
Repeat associations are idempotent; associations to another physical meter
are rejected rather than silently reassigned.

Check associations with:

```bash
bash meter-prod.sh resources --meter "<METER_PROFILE_ID>"
```

## Meter numbers and privacy

Physical meter numbers from a utility portal may not occur in its
Green Button XML. The schema supports an optional private meter-number
field, but this command deliberately does not accept or print it.
A secure, interactive customer interface is needed before entry of meter
numbers is enabled.

## Reporting boundaries

Associations are explicit and do not automatically merge or sum readings.
The next reporting stage will select one valid observation per logical
stream interval, with clear rules for overlapping sources and conflicts.
A verified association is necessary but not by itself sufficient to make
an aggregate suitable for Home Assistant.

## Meter names and utility selection

Meter profile names are customer-defined and can be changed without
altering usage readings or Green Button identities.

When one utility is registered, creation selects that utility:

```bash
bash meter-prod.sh create --kind electric --name "Home Electric"
bash meter-prod.sh create --kind gas --name "Gas"
```

When multiple utilities exist, specify `--custodian ID` explicitly.
For an existing profile, change only the friendly name:

```bash
bash meter-prod.sh rename --meter ID --name "Home Electric"
```

Run `bash meter-prod.sh list` to view existing profiles. An empty
installation reports `No meters configured.`

A name change does not associate Green Button resources. Associations
must still be approved explicitly with the `attach` operation.

## Associate compatible resources in one operation

After confirming the registered utility and physical meter profile, run:

```bash
bash meter-prod.sh attach-compatible --meter "<METER_PROFILE_ID>"
```

The command first lists compatible, unassociated Green Button resources.
It then requests confirmation. Canceling does not change any associations.
For a noninteractive, explicitly approved operation, add `--yes`.

The operation requires exactly one physical meter profile for the
utility and commodity. When multiple meters of the same commodity are
configured, associate resources individually after identifying which
physical meter each resource belongs to. Existing associations to
another meter are never reassigned. The confirmed operation checks
eligibility again and makes all changes in one transaction.

Resource identifiers are internal diagnostic references, not utility
meter numbers. Commodity compatibility does not establish physical
meter identity. Confirm the meter relationship before bulk attachment.

Re-running the command is safe: previously associated resources are
reported separately and are not inserted again. The operation changes
associations only; it does not merge or modify interval readings.

## Read-only resolution foundation

The conservative initial interval-selection API is documented in
[`USAGE_STREAM_RESOLUTION.md`](USAGE_STREAM_RESOLUTION.md). It is not yet
a user-facing command or Home Assistant-ready reporting endpoint.
