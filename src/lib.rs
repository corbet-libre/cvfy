//! Pure, bounded proof verification through the maintained upstream native worker.
#![forbid(unsafe_code)]

mod native;
pub use native::{NativeCapacity, NativeVerifier, NativeVerifierConfig};
