//! Native process ownership; all authenticated artifacts remain in memory.
use czkp::{Error, ProofScope, native as wire};
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
mod transport;

/// Explicit trusted host configuration. Never derive these values from a proof.
#[derive(Clone)]
pub struct NativeVerifierConfig {
    /// Absolute path to the owned `czkp-bb-worker`, not the upstream `bb` CLI.
    pub executable: PathBuf,
    /// Authenticated circuit and verification-key identity.
    pub scope: ProofScope,
    /// Whole-batch monotonic time limit, including encoding and worker startup.
    pub timeout: Duration,
    /// Maximum concurrently admitted batches, shared across clones.
    pub maximum_parallel: usize,
    /// Per-proof byte bound before encoding; batches also have a 4 MiB bound.
    pub max_proof_bytes: usize,
    /// Per-proof number of public field elements.
    pub max_public_inputs: usize,
    /// Explicit native worker thread limit.
    pub threads: usize,
    /// Finite native address-space limit in MiB, applied before parsing.
    pub memory_megabytes: usize,
    /// Caller-authenticated upstream verification-key bytes.
    pub verification_key: Vec<u8>,
    /// Caller-authenticated first compressed G1 chunk (exactly 4 MiB).
    pub srs_g1: Vec<u8>,
    /// Caller-authenticated G2 point; upstream validates its pinned identity.
    pub srs_g2: [u8; 128],
}

/// One anonymous admission pool, reusable across independently scoped verifiers.
#[derive(Clone)]
pub struct NativeCapacity {
    limit: usize,
    active: Arc<AtomicUsize>,
}
impl NativeCapacity {
    /// Create a pool allowing between one and 64 concurrent batches.
    pub fn new(maximum_parallel: usize) -> Result<Self, Error> {
        if !(1..=64).contains(&maximum_parallel) {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            limit: maximum_parallel,
            active: Arc::new(AtomicUsize::new(0)),
        })
    }
    fn reserve(&self) -> Result<Permit, Error> {
        self.active
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.limit).then_some(n + 1)
            })
            .or(Err(Error::Capacity))?;
        Ok(Permit(self.active.clone()))
    }
}
struct Permit(Arc<AtomicUsize>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// One-way host cancellation. Clones only withdraw execution authority; this
/// cannot reset a verifier, change its inputs, or produce an accepting verdict.
#[derive(Clone, Default)]
pub struct NativeCancellation(Arc<AtomicBool>);
impl NativeCancellation {
    /// Permanently refuse new work and stop the current owned process at its
    /// next bounded poll, killing and reaping it before returning.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    fn check(&self) -> Result<(), Error> {
        if self.0.load(Ordering::Acquire) {
            Err(Error::CryptoProvider)
        } else {
            Ok(())
        }
    }
}

/// Stateless verification with one isolated process and admission per batch.
#[derive(Clone)]
pub struct NativeVerifier {
    config: Arc<NativeVerifierConfig>,
    capacity: NativeCapacity,
    cancellation: NativeCancellation,
}
impl NativeVerifier {
    /// Validate the host's immutable RAM snapshot and create its admission pool.
    pub fn new(config: NativeVerifierConfig) -> Result<Self, Error> {
        let capacity = NativeCapacity::new(config.maximum_parallel)?;
        Self::with_capacity(config, capacity)
    }
    /// Reuse an existing anonymous pool across different authenticated keys.
    /// The configured parallel bound must equal the pool's bound.
    pub fn with_capacity(
        config: NativeVerifierConfig,
        capacity: NativeCapacity,
    ) -> Result<Self, Error> {
        let key_digest: [u8; 32] = Sha256::digest(&config.verification_key).into();
        if !config.executable.is_absolute()
            || config.scope.circuit_digest == [0; 32]
            || config.scope.verifying_key_digest != key_digest
            || config.verification_key.is_empty()
            || config.verification_key.len() > wire::MAX_KEY_BYTES
            || config.srs_g1.len() != wire::SRS_G1_BYTES
            || config.timeout.is_zero()
            || config.timeout > Duration::from_secs(300)
            || config.maximum_parallel != capacity.limit
            || !(1..=wire::MAX_PROOF_BYTES).contains(&config.max_proof_bytes)
            || !(1..=wire::MAX_PUBLIC_INPUTS).contains(&config.max_public_inputs)
            || !(1..=64).contains(&config.threads)
            || !(256..=65_536).contains(&config.memory_megabytes)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            config: Arc::new(config),
            capacity,
            cancellation: NativeCancellation::default(),
        })
    }
    /// The host retains this handle when transferring the verifier to its actor.
    pub fn cancellation(&self) -> NativeCancellation {
        self.cancellation.clone()
    }
    /// Exact authenticated identity, independent of a caller-supplied statement.
    pub fn scope(&self) -> ProofScope {
        self.config.scope.clone()
    }

    /// Verify one proof under the configured whole-call timeout.
    pub fn verify(&self, inputs: &[[u8; 32]], proof: &[u8]) -> Result<(), Error> {
        self.verify_before(inputs, proof, Instant::now() + self.config.timeout)
    }
    /// Verify before an earlier caller deadline, including domain preparation time.
    pub fn verify_before(
        &self,
        inputs: &[[u8; 32]],
        proof: &[u8],
        deadline: Instant,
    ) -> Result<(), Error> {
        self.verify_batch_before(&[(inputs, proof)], deadline)
    }
    /// Verify every ordered proof with one admission, setup initialization and
    /// absolute deadline. At most 65 proofs, 4 MiB proof bytes and 32,768 fields
    /// are accepted. A refusal stops the batch; partial success never escapes.
    pub fn verify_batch_before(
        &self,
        requests: &[(&[[u8; 32]], &[u8])],
        deadline: Instant,
    ) -> Result<(), Error> {
        self.cancellation.check()?;
        let now = Instant::now();
        let deadline = deadline.min(now + self.config.timeout);
        transport::check_deadline(now, deadline)?;
        if requests.is_empty() || requests.len() > wire::MAX_PROOFS {
            return Err(Error::InvalidInput);
        }
        let mut total_bytes = 0;
        let mut total_fields = 0;
        for (inputs, proof) in requests {
            if inputs.is_empty()
                || inputs.len() > self.config.max_public_inputs
                || proof.is_empty()
                || proof.len() > self.config.max_proof_bytes
                || proof.len() % 32 != 0
            {
                return Err(Error::InvalidInput);
            }
            total_bytes += proof.len();
            total_fields += inputs.len();
            if total_bytes > wire::MAX_PROOF_BYTES || total_fields > wire::MAX_TOTAL_PUBLIC_INPUTS {
                return Err(Error::InvalidInput);
            }
            for input in *inputs {
                czkp::field(input, false)?;
            }
        }
        let _permit = self.capacity.reserve()?;
        let hex: Vec<_> = requests
            .iter()
            .map(|(_, proof)| HEXLOWER.encode(proof))
            .collect();
        let proofs: Vec<_> = requests
            .iter()
            .zip(&hex)
            .map(|((inputs, _), proof)| wire::Proof {
                public_inputs: inputs,
                proof,
            })
            .collect();
        let key = HEXLOWER.encode(&self.config.verification_key);
        let g1 = HEXLOWER.encode(&self.config.srs_g1);
        let g2 = HEXLOWER.encode(&self.config.srs_g2);
        let request = wire::Request {
            scope: &self.config.scope,
            verification_key: &key,
            srs_g1: &g1,
            srs_g2: &g2,
            proofs: &proofs,
        };
        // A slice writer bounds serialized allocation independently of serde.
        let mut bytes = vec![0; wire::MAX_REQUEST_BYTES];
        let mut remaining = bytes.as_mut_slice();
        serde_json::to_writer(&mut remaining, &request).or(Err(Error::InvalidInput))?;
        let written = wire::MAX_REQUEST_BYTES - remaining.len();
        bytes.truncate(written);
        transport::verify(&self.config, bytes, deadline, &self.cancellation)
    }
}
