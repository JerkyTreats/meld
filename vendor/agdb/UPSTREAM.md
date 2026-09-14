# Temporary agdb dependency

Source: [agnesoft/agdb](https://github.com/agnesoft/agdb/commit/170b956be92591cb8ce922be180bb93aff0151d7), revision `170b956be92591cb8ce922be180bb93aff0151d7`.

The engine and derive crate are copied from this revision, including their upstream tests and Apache 2.0 license. The engine carries the three-line transaction-boundary and reverse-WAL correction in `engine-only.patch`. The added upstream regression test is included. No other engine changes are intended.

This explicit vendored dependency enables the authorized native experiment with `SyncMode::Commit`. It is temporary support debt, not a commitment to maintaining an engine fork. Replace it with an upstream-reviewed corrected revision before production release qualification. Engine source remains upstream-owned; domain behavior belongs outside this directory.

Upstream preparation passed all-feature tests and doctests, Clippy with warnings rejected, formatting, and coverage. Process interruption has regression coverage. Power loss, storage failures, interrupted recovery and caught-panic reuse remain unqualified. Detailed native integration evidence belongs in the Graph delivery record.

## Upstream submission

The correction is submitted as [issue 1930](https://github.com/agnesoft/agdb/issues/1930) and [PR 1931](https://github.com/agnesoft/agdb/pull/1931). The temporary holding fork is [JerkyTreats/agdb](https://github.com/JerkyTreats/agdb/tree/fix/transaction-recovery), fixed commit `67c264f256852574496f5a4e5886f54802eec3cf`, based on upstream `830ea5d2ab091204864838ed6d1b1acef846f28e`.

All 142 Rust source and test files across the vendored engine and derive crate match that fork commit byte for byte. Cargo continues using the explicit local vendor paths; no moving fork branch is introduced into dependency resolution. The fork holds the existing correction while upstream reviews it. Submission is not upstream acceptance and does not close the release-review bracket above.

The submitted revision passes 1,371 package tests and doctests, strict all-feature Clippy, and formatting. Earlier negative controls and coverage remain recorded against the original preparation revision; no new coverage run is claimed. The standalone issue and PR contain only the defect, correction, reproduction and validation.
