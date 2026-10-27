<!-- SPDX-License-Identifier: MIT -->

# Analyze imported Energy Usage data

## Purpose

Explain analysis of imported usage streams without modifying data.

**Intended Audience:** Operators and developers investigating imported data.

Run the analysis after importing Green Button XML. The command works
without access to the original XML files and does not change data.

```bash
cd deploy/docker-compose
bash analyze-prod-data.sh
```

The report groups imported resources by electricity, natural gas, or
another commodity. It reports the number of imported resources, interval
resolution, UTC coverage, overlapping resource pairs, and differences
between accepted values for comparable overlapping intervals.

**Imported resources are not physical meters.** One utility meter may
appear in several XML downloads. Matching readings suggest possible
equivalence, but do not establish that records belong to the same meter.
The report does not associate meters or select Home Assistant streams.

For technical troubleshooting, show database resource identifiers,
original Green Button numeric commodity/unit codes, and overlap pairs:

```bash
bash analyze-prod-data.sh --details
```

The detailed report also identifies the UTC dates and **unique interval
counts** where two resources attached to the same physical meter contain
different accepted values. These counts differ from the pairwise totals:
one hour may appear in several resource pairs but is counted only once
per physical meter and date. The report does not expose reading values.

A conflict is not automatically a utility correction. The canonical
usage stream continues to reject conflicting overlaps until an explicit
resolution policy has been verified. Do not add overlapping resources
together or import unresolved totals into Home Assistant.

For command help:

```bash
bash analyze-prod-data.sh --help
```

No meter numbers, external Green Button identifiers, credentials, or
individual reading values are displayed. The command never changes
stored observations or customer-defined meter settings.
