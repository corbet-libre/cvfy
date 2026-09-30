# cvfy contract

`ProcessVerifier::verify` checks a serializable public statement and proof using a configured worker. Circuit/VK scope, executable, artifacts, concurrency, input size, memory and deadline are host configuration. Requests cannot select them. Failures expose no child stderr or proof data. The same instance shares a concurrency bound across clones; timeout kills and reaps the worker. It stores nothing.

Survey (2026-09-30): retain [Noir](https://github.com/noir-lang/noir) / [Barretenberg](https://github.com/AztecProtocol/aztec-packages/tree/next/barretenberg), via the upstream `@aztec/bb.js` worker migrated into cblc. A native ACVM executor is not a proof verifier; a different proof system would invalidate existing artifacts. The Rust wrapper uses std process APIs, serde and [czkp](https://github.com/corbet-foss/czkp) wire types. No own crypto. The worker must validate every public input and pinned artifact. `verified: true` from an arbitrary worker is not cryptographic evidence; executable configuration is trusted.

Source extraction: maintained adapters from cfrm; substantive accounting policy remains in cblc. This package is not independently a complete private-account relation.
