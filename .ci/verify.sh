#!/usr/bin/env bash
set -euo pipefail
exec 2>&1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
CVFY_LAST_GATE='init'
trap 'rc=$?; echo "cvfy:last_gate=$CVFY_LAST_GATE exit=$rc"' EXIT

CVFY_LAST_GATE='native-sdk'
python3 -B .ci/test-native-sdk.py
python3 -B .ci/check-native-sdk.py
CVFY_LAST_GATE='first-party'
python3 .github/check-first-party.py
CVFY_LAST_GATE='rustc-version'
rustc --version
CVFY_LAST_GATE='fmt'
cargo fmt --check || { cargo fmt; git diff > formatting.patch; exit 1; }
CVFY_LAST_GATE='clippy'
cargo clippy --locked --all-targets --features test-worker -- -D warnings

CVFY_LAST_GATE='fixture-unpack'
# Consumes the fresh fixture prepared by run-cached.sh (fixture-resolve
# stage): fixture-source.tar + fixture-package-lock.json are regenerated
# every job and pinned into the verify cache key via cache_env, so the
# cached check can unpack them unchanged here.
mkdir -p fixture-source
tar -xf fixture-source.tar -C fixture-source
cp fixture-package-lock.json fixture-source/package-lock.json
CVFY_LAST_GATE='fixture-npm-ci'
npm ci --ignore-scripts --no-audit --no-fund --prefix fixture-source
CVFY_LAST_GATE='fixture-real-proof'
(cd fixture-source && node tests/real-proof.mjs && node .ci/native-fixture-inputs.ts)
CVFY_LAST_GATE='fixture-config'
node --input-type=module -e 'import {readFileSync,writeFileSync} from "node:fs"; import {resolve} from "node:path"; const p=resolve("fixture-source/.proof-test/config.json"); const c=JSON.parse(readFileSync(p)); c.directory=resolve("fixture-source/.proof-test"); writeFileSync(p,JSON.stringify(c));'
export CVFY_FIXTURE_SOURCE="$PWD/fixture-source"
CVFY_LAST_GATE='cargo-test'
cargo build --locked --example czkp_bb_worker
cvfy_target_dir="${CARGO_TARGET_DIR:-target}"
export CVFY_BB="$(realpath "$cvfy_target_dir/debug/examples/czkp_bb_worker")"
cargo test --locked --features test-worker

CVFY_LAST_GATE='nightly-versions'
rustup run nightly rustc --version
NIGHTLY_RUSTC="$(rustup which --toolchain nightly rustc)"
NIGHTLY_CARGO="$(rustup which --toolchain nightly cargo)"
NIGHTLY_BIN="$(dirname "$NIGHTLY_RUSTC")"
export RUSTC="$NIGHTLY_RUSTC"
export CARGO="$NIGHTLY_CARGO"
export PATH="$NIGHTLY_BIN:$PATH"
export RUSTUP_TOOLCHAIN=nightly
"$NIGHTLY_CARGO" llvm-cov --version
CVFY_LAST_GATE='coverage-json'
# Bug 2 isolation: coverage data must not reuse the shared cargo target
# across jobs (stale checkout paths fail the next gate); scope the
# coverage target to this job only. Native gates keep the shared warm
# target; outer traps and services are untouched.
(
coverage_target="$(mktemp -d)"
case "$coverage_target" in ""|"$PWD"/*) echo 'unsafe coverage target' >&2; exit 2;; esac
trap 'rm -rf -- "$coverage_target"' EXIT
export CARGO_TARGET_DIR="$coverage_target"
"$NIGHTLY_CARGO" build --locked --example czkp_bb_worker
export CVFY_BB="$CARGO_TARGET_DIR/debug/examples/czkp_bb_worker"
"$NIGHTLY_CARGO" llvm-cov --locked --workspace --all-features --branch --no-cfg-coverage --no-cfg-coverage-nightly --ignore-filename-regex '/tests/' --json --output-path coverage.json
CVFY_LAST_GATE='coverage-lcov'
"$NIGHTLY_CARGO" llvm-cov report --branch --ignore-filename-regex '/tests/' --lcov --output-path coverage.lcov
CVFY_LAST_GATE='coverage-text'
"$NIGHTLY_CARGO" llvm-cov report --branch --ignore-filename-regex '/tests/' --text --output-path coverage.txt
)
CVFY_LAST_GATE='test-source-coverage'
python3 .github/test-source-coverage.py
CVFY_LAST_GATE='check-source-coverage'
python3 .github/check-source-coverage.py coverage.lcov coverage.json coverage.txt
