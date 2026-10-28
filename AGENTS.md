# cvfy agent instructions

Write comments and documentation in English. Owned code forbids unsafe Rust
(`#![forbid(unsafe_code)]`, `src/lib.rs:2`).

## Build and test (lane: `~/agents/scratch/cbuild <lane> ...`)

- `fmt -p cvfy -- --check` (exit 0)
- `clippy -p cvfy --locked --all-targets --features test-worker -- -D warnings`
  (exit 0)
- `test -p cvfy --locked --features test-worker --test native` (exit 0:
  6 refusal/config/bound tests; no crypto acceptance claimed)
- CI-only (GHA `.ci/verify.sh`): the full suite needs `CVFY_FIXTURE_SOURCE`
  (real upstream proofs) and `BB_NATIVE_RUSTFLAGS`; genuine acceptance
  (`tests/real_account.rs`), `cargo deny`, coverage (`docs/COVERAGE.md`).

## Hard invariants (each with its enforcing test)

- At most 65 pairs per batch under one absolute `Instant` deadline; success
  observed at or after expiry is refused (`docs/CONTRACT.md:3`):
  `one_batch_holds_shared_capacity_until_the_absolute_deadline_then_releases_it`,
  `a_genuine_proof_observed_after_its_deadline_is_refused` (lib, needs fixture).
- The constructor authenticates the key SHA-256 against its scope and checks
  every resource bound (`docs/CONTRACT.md:5`):
  `configuration_authenticates_identity_and_every_resource_bound`.
- Empty, oversized, noncanonical or misaligned inputs are refused before
  dispatch and never retain capacity (`docs/CONTRACT.md:7`):
  `input_limits_refuse_before_dispatch_and_leave_capacity_available`.
- Worker failures are bounded and redacted; cancellation kills, reaps and
  permanently refuses new work (`docs/CONTRACT.md:9-11`):
  `real_process_refusals_are_bounded_and_diagnostics_are_redacted`,
  `blocked_input_is_killed_and_reaped_within_the_whole_call_timeout`,
  `host_cancellation_kills_owned_process_and_permanently_refuses_new_work`.
- Only retained upstream proofs accept; tampered proofs or inputs are refused
  (`tests/real_account.rs`, CI fixtures):
  `maintained_account_proofs_bind_all_inputs_and_refuse_tampering`.
- A substituted executable is not evidence: scope and binary are trust
  configuration (`docs/CONTRACT.md:15`).

Pointers: `docs/CONTRACT.md`, `docs/COVERAGE.md`.
