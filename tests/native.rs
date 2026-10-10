//! Actual native process refusal tests. None claims cryptographic acceptance.
#![cfg(feature = "test-worker")]
use cvfy::{NativeCapacity, NativeVerifier, NativeVerifierConfig};
use czkp::{Error, ProofScope, native as wire};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

fn config(directory: &Path, mode: &str) -> NativeVerifierConfig {
    let executable = directory.join(mode);
    fs::copy(env!("CARGO_BIN_EXE_cvfy-refusal-worker"), &executable).unwrap();
    let key = vec![5; 32];
    NativeVerifierConfig {
        executable,
        scope: ProofScope {
            circuit_digest: [1; 32],
            verifying_key_digest: Sha256::digest(&key).into(),
        },
        timeout: Duration::from_secs(5),
        maximum_parallel: 1,
        max_proof_bytes: 1024 * 1024,
        max_public_inputs: 4096,
        threads: 1,
        memory_megabytes: 256,
        verification_key: key,
        srs_g1: vec![0; wire::SRS_G1_BYTES],
        srs_g2: [0; 128],
    }
}
fn input() -> [[u8; 32]; 1] {
    [[0; 32]]
}

#[test]
fn configuration_authenticates_identity_and_every_resource_bound() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path(), "refuse");
    assert_eq!(
        NativeVerifier::new(config.clone()).unwrap().scope(),
        config.scope
    );
    for case in 0..19 {
        let mut bad = config.clone();
        match case {
            0 => bad.executable = "worker".into(),
            1 => bad.scope.circuit_digest = [0; 32],
            2 => bad.scope.verifying_key_digest[0] ^= 1,
            3 => bad.verification_key.clear(),
            4 => bad.verification_key = vec![0; wire::MAX_KEY_BYTES + 1],
            5 => {
                bad.srs_g1.pop();
            }
            6 => bad.timeout = Duration::ZERO,
            7 => bad.timeout = Duration::from_secs(301),
            8 => bad.maximum_parallel = 0,
            9 => bad.maximum_parallel = 65,
            10 => bad.max_proof_bytes = 0,
            11 => bad.max_proof_bytes = wire::MAX_PROOF_BYTES + 1,
            12 => bad.max_public_inputs = 0,
            13 => bad.max_public_inputs = wire::MAX_PUBLIC_INPUTS + 1,
            14 => bad.threads = 0,
            15 => bad.threads = 65,
            16 => bad.memory_megabytes = 255,
            17 => bad.memory_megabytes = 65_537,
            _ => {
                bad.verification_key[0] ^= 1;
            }
        }
        assert!(
            matches!(NativeVerifier::new(bad), Err(Error::InvalidInput)),
            "case {case}"
        );
    }
    assert!(matches!(
        NativeVerifier::with_capacity(config, NativeCapacity::new(2).unwrap()),
        Err(Error::InvalidInput)
    ));
}

#[test]
fn input_limits_refuse_before_dispatch_and_leave_capacity_available() {
    let directory = tempfile::tempdir().unwrap();
    let mut config = config(directory.path(), "refuse");
    config.executable = directory.path().join("absent-worker");
    let verifier = NativeVerifier::new(config).unwrap();
    for (fields, proof) in [
        (vec![], vec![0; 32]),
        (vec![[0; 32]; 4097], vec![0; 32]),
        (vec![[255; 32]], vec![0; 32]),
        (vec![[0; 32]], vec![]),
        (vec![[0; 32]], vec![0; 31]),
        (vec![[0; 32]], vec![0; 1024 * 1024 + 32]),
    ] {
        assert_eq!(verifier.verify(&fields, &proof), Err(Error::InvalidInput));
    }
    let fields = input();
    let proof = [0; 32];
    let deadline = Instant::now() + Duration::from_secs(20);
    assert_eq!(
        verifier.verify_batch_before(&[], deadline),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        verifier.verify_batch_before(&vec![(&fields[..], &proof[..]); 66], deadline),
        Err(Error::InvalidInput)
    );
    let many = vec![[0; 32]; 4096];
    assert_eq!(
        verifier.verify_batch_before(&vec![(&many[..], &proof[..]); 9], deadline),
        Err(Error::InvalidInput)
    );
    let large = vec![0; 1024 * 1024];
    assert_eq!(
        verifier.verify_batch_before(&vec![(&fields[..], &large[..]); 5], deadline),
        Err(Error::InvalidInput)
    );
    assert_eq!(
        verifier.verify_before(&fields, &proof, Instant::now()),
        Err(Error::CryptoProvider)
    );
    // The missing executable is reached twice: no invalid request/spawn failure
    // may retain the sole capacity permit.
    assert_eq!(verifier.verify(&fields, &proof), Err(Error::CryptoProvider));
    assert_eq!(verifier.verify(&fields, &proof), Err(Error::CryptoProvider));
}

#[test]
fn real_process_refusals_are_bounded_and_diagnostics_are_redacted() {
    let directory = tempfile::tempdir().unwrap();
    for mode in [
        "oversize",
        "partial",
        "empty",
        "wrong_scope",
        "extra",
        "refuse",
        "environment",
    ] {
        let config = config(directory.path(), mode);
        let verifier = NativeVerifier::new(config.clone()).unwrap();
        let error = verifier.verify(&input(), &[0; 32]).unwrap_err();
        assert_eq!(error, Error::CryptoProvider, "{mode}");
        assert!(!format!("{error:?}").contains("diagnostic-private-marker"));
        if mode == "environment" {
            assert_eq!(
                fs::read_to_string(config.executable.with_extension("receipt")).unwrap(),
                "controlled"
            );
        }
    }
}

#[test]
fn one_batch_holds_shared_capacity_until_the_absolute_deadline_then_releases_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut batch = config(directory.path(), "batch");
    batch.timeout = Duration::from_secs(10);
    let receipt = batch.executable.with_extension("receipt");
    let capacity = NativeCapacity::new(1).unwrap();
    let verifier = NativeVerifier::with_capacity(batch, capacity.clone()).unwrap();
    let second =
        NativeVerifier::with_capacity(config(directory.path(), "refuse"), capacity).unwrap();
    let started = Instant::now();
    let deadline = started + Duration::from_secs(3);
    let handle = std::thread::spawn(move || {
        let fields = input();
        let proof = [0; 32];
        verifier.verify_batch_before(&vec![(&fields[..], &proof[..]); 65], deadline)
    });
    while !receipt.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(fs::read_to_string(receipt).unwrap(), "65");
    assert_eq!(
        second.clone().verify(&input(), &[0; 32]),
        Err(Error::Capacity)
    );
    assert_eq!(handle.join().unwrap(), Err(Error::CryptoProvider));
    assert!(started.elapsed() < Duration::from_secs(6));
    assert_eq!(
        second.verify(&input(), &[0; 32]),
        Err(Error::CryptoProvider)
    );
}

#[test]
fn blocked_input_is_killed_and_reaped_within_the_whole_call_timeout() {
    let directory = tempfile::tempdir().unwrap();
    let mut config = config(directory.path(), "hang");
    config.timeout = Duration::from_secs(2);
    let receipt = config.executable.with_extension("receipt");
    let verifier = NativeVerifier::new(config).unwrap();
    let start = Instant::now();
    assert_eq!(
        verifier.verify(&input(), &[0; 32]),
        Err(Error::CryptoProvider)
    );
    assert!(start.elapsed() < Duration::from_secs(5));
    assert_eq!(fs::read_to_string(receipt).unwrap(), "started");
    assert_eq!(
        verifier.verify(&input(), &[0; 32]),
        Err(Error::CryptoProvider)
    );
}
