//! Isolated native upstream worker for the actual proof integration tests.
#![forbid(unsafe_code)]
fn main() {
    if czkp::native::run_worker().is_err() {
        std::process::exit(1);
    }
}
