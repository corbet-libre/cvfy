# cvfy

**Zero-knowledge accounting verifier.**

`cvfy` is the verifier in the issuer-verifier-holder schema: it checks a proof against a policy. Checking is pure and local and leaves no trace. `cblc` is its caller.

Status: name reserved, no implementation yet.

## Boundaries

What it does:

- Checks proofs against published policies (circuits, keys, formats).

What it never does:

- Never writes state or records anything (that is `cssr`).
- No own proof system: the upstream system stays a dependency via `czkp`.

## License

Copyright 2026 Julian Y. Richard Corbet. Licensed under
[LGPL-3.0-only](LICENSES/LGPL-3.0-only.txt)
[WITH LGPL-3.0-linking-exception](LICENSES/LGPL-3.0-linking-exception.txt).
See [LICENSE.md](LICENSE.md).
