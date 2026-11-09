//! The crate's integration tests in one binary.
//!
//! Every former `tests/<name>.rs` is a module here, so a warm edit loop links one test
//! executable instead of one per file. The test names keep their module prefix
//! (`<name>::<test>`). `native` keeps its own feature gate (`test-worker`); the
//! `real_account` test is the CI-fixture `bot` tier of `cmeet/ci/land-tests.txt`.
mod native;
mod real_account;
