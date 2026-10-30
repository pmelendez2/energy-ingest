# Private Atom relationship graph verification

**Audience:** fixture privacy maintainers and profile contributors.

**Purpose:** Compare every Atom entry and direct `link` relationship before
and after pseudonymization without exposing customer IDs or resource URLs.

The new `verify_graph(source, candidate, mapping)` Rust API requires an
explicit **private** map of original Atom IDs and original link URLs to
their candidate values. The map must cover *all* source IDs and hrefs,
including unchanged external links. The checker compares multisets of
mapped Atom entries and `(entry ID, rel, href)` edges. Link and document
ordering may change; deleted entries, link swaps, changed relation labels,
and changed multiplicities fail the comparison.

Results include counts and the boolean `exact_relationships_preserved`.
They deliberately contain no identifier, URL, or mapping values, and
`publication_authorized` always remains `false`. Keep the map private;
never commit it or write it to an evidence ZIP.

This **library-only** API is not wired into CLI `verify-set` or candidate
sanitization yet. The caller must ensure that the correspondence comes
from the trusted sanitizer, not a mapping invented to fit a damaged
candidate. It inspects Atom entry IDs and direct entry links only;
other ESPI fields, nested references, feed-level links, namespace-based
semantics, transformations of private text, and privacy clearance remain
separate fail-closed tasks. For SDG&E, the six original shared-self-URL
conflicts must remain six corresponding associations after rewriting;
this is not evidence of a confirmed provider defect.

## Installation note

Add these two lines to `crates/fixture_privacy/src/lib.rs` alongside its
existing module declarations and public exports (do not replace the file):

```rust
mod graph_verification;
pub use graph_verification::{
    GraphVerificationReport,
    IdentityMap,
    verify_graph,
};
```

Then run `cargo fmt --all`, `cargo check -p fixture_privacy --all-targets`,
`cargo test -p fixture_privacy`, and
`cargo clippy -p fixture_privacy --all-targets -- -D warnings`.
