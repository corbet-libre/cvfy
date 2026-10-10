# cvfy

`NativeVerifier` checks ordered public field elements and upstream proofs through an isolated Rust worker. It reserves one shared admission slot and one absolute deadline for a whole batch. Circuit/key identity, executable, authenticated setup bytes, threads, memory and size limits come from the host. Requests cannot select them. All request and artifact data stays in RAM; errors expose no native diagnostics.

Cryptography stays in maintained [Barretenberg's official Rust API](https://github.com/AztecProtocol/aztec-packages/tree/v5.2.1/barretenberg/rust/barretenberg-rs), through [czkp](https://git.corbet.ch/corbet-foss/czkp). Owned crates forbid unsafe code. Upstream FFI and native cryptography run in a separate bounded process. No Node runtime or owned cryptographic implementation is involved.

The host must supply the trusted `czkp-bb-worker` executable, not an arbitrary program or the upstream `bb` CLI. The latter initializes filesystem/network CRS handling. The owned worker initializes only the supplied authenticated memory CRS through the official API. Real account and extension proof replays qualify SDK upgrades against the existing proof format; a dependency version match alone does not establish compatibility.

See [the contract](docs/CONTRACT.md) and [coverage evidence](docs/COVERAGE.md). Build and verification run in CI, with current compatible manifest versions and a retained tested lockfile.

Copyright 2026 Julian Y. Richard Corbet. LGPL-3.0-only WITH LGPL-3.0-linking-exception; see [LICENSE.md](LICENSE.md).
