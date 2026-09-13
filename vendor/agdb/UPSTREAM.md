# Temporary agdb dependency

Source: [agnesoft/agdb](https://github.com/agnesoft/agdb/commit/170b956be92591cb8ce922be180bb93aff0151d7), revision `170b956be92591cb8ce922be180bb93aff0151d7`.

The engine and derive crate are copied from this revision, including their upstream tests and Apache 2.0 license. The engine carries the three-line transaction-boundary and reverse-WAL correction in `engine-only.patch`. The added upstream regression test is included. No other engine changes are intended.

This explicit vendored dependency enables the authorized native experiment with `SyncMode::Commit`. It is temporary support debt, not a commitment to maintaining an engine fork. Replace it with an upstream-reviewed corrected revision before production release qualification. Engine source remains upstream-owned; domain behavior belongs outside this directory.

Upstream preparation passed all-feature tests and doctests, Clippy with warnings rejected, formatting, and coverage. Process interruption has regression coverage. Power loss, storage failures, interrupted recovery and caught-panic reuse remain unqualified. Detailed native integration evidence belongs in the Graph delivery record.
