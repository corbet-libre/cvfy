# Agent instructions

Write all code comments and documentation in English.

## Product boundary

- Verifier for zero-knowledge accounting: checks a proof against a policy, pure and without trace.
- Checks proofs against published policies (circuits, keys, formats).
- Never writes state or records anything (that is `cssr`).
- No own proof system: the upstream system stays a dependency via `czkp`.
- This crate is LGPL-3.0-only WITH LGPL-3.0-linking-exception. Do not add implementation code available only under the full GPL or AGPL.
- Called only through `cblc`; `cvld` never talks to `cvfy` directly.

## Quality boundary

- `cargo fmt --check`, `cargo clippy --all-targets` (no warnings),
  `cargo test` — all green before every commit. No local
  builds; use Crow with the canonical Forgejo repository.
