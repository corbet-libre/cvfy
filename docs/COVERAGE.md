# Coverage contract

CI targets 100% of reachable production lines and branches. Stable Rust runs the native process adapter against actual public Noir account proofs. Nightly Rust is used only for LLVM branch
instrumentation, which currently requires it. Both jobs use the same resolved
Cargo.lock snapshot; all build and test commands after resolution use --locked.

The coverage job executes real tests with cargo-llvm-cov and retains the raw JSON
even when the gate fails. The checker compares integer covered/total counts for
both metrics; rounded percentages cannot pass. An empty report cannot pass.
The report excludes integration-test harness files under tests/, not production
code. No production source exclusions are currently approved.

A failing gate is missing evidence, not permission to lower the threshold or
change domain behavior. Add meaningful failure and round-trip tests. Document
any genuinely unreachable defensive branch precisely before excluding it. Native
coverage does not establish browser execution; keep the actual wasm vectors.

First-party dependencies follow main. Their resolved full revisions remain in
Cargo.lock, with exactly one source per first-party crate. Verification consumes
the retained, committed lock snapshot with `--locked`; it does not refresh
dependencies. An explicit dependency refresh is a separate operation: retain
and commit the resolved snapshot, then validate that exact snapshot. A passing
verification run establishes compatibility of the recorded revisions, not a
new dependency-resolution or freshness claim.

## Emitted source counters

The same execution exports raw LLVM JSON, LCOV and annotated source. The strict
gate requires every emitted production line and both outcomes at every branch
location, with no exclusions. It cross-checks exact file and line inventories,
branch coordinates, summaries and hit status across the reports, rejecting missing
or duplicate input. Generic instantiation summaries remain evidence, not source
line counts: df40d10 measured89/90 raw lines while all85 emitted source lines and
36/36 branches executed. These figures are kept distinct. No successful test or
compiler check is treated as coverage.
