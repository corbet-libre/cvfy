//! Integration tests for the public contract.
use cvfy::{ProcessVerifier, ProcessVerifierConfig};
use czkp::{Error, ProofScope};
use std::{fs, time::Duration};
#[test]
fn reject_bad_configuration_and_failed_worker() {
    let dir = tempfile::tempdir().unwrap();
    let config = ProcessVerifierConfig {
        node: "/usr/bin/node".into(),
        script: dir.path().join("worker.mjs"),
        artifact_config: dir.path().join("manifest.json"),
        scope: ProofScope {
            circuit_digest: [1; 32],
            verifying_key_digest: [2; 32],
        },
        timeout: Duration::from_millis(500),
        maximum_parallel: 1,
        max_proof_bytes: 100,
        node_heap_megabytes: 64,
    };
    fs::write(
        &config.script,
        "process.stdin.resume(); setInterval(()=>{},1000)",
    )
    .unwrap();
    let verifier = ProcessVerifier::new(config.clone()).unwrap();
    assert_eq!(verifier.verify(&1, &[1]), Err(Error::CryptoProvider));
    fs::write(
        &config.script,
        "process.stdin.resume(); process.stdout.write('x'.repeat(5000))",
    )
    .unwrap();
    assert_eq!(verifier.verify(&1, &[1]), Err(Error::CryptoProvider));
    assert_eq!(verifier.verify(&1, &[]), Err(Error::InvalidInput));
    let mut bad = config;
    bad.node = "node".into();
    assert!(ProcessVerifier::new(bad).is_err());
}
