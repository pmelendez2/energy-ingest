<!-- SPDX-License-Identifier: MIT -->

# Analyze imported Energy Usage data

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

For command help:

```bash
bash analyze-prod-data.sh --help
```

No meter numbers, external Green Button identifiers, credentials, or
individual reading values are displayed. The command never changes
stored observations or customer-defined meter settings.
