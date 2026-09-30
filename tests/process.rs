//! Integration tests for the public contract.
use cvfy::{ProcessVerifier, ProcessVerifierConfig};
use czkp::{Error, ProofScope};
use std::{fs, time::Duration};
fn node() -> std::path::PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|path| path.join("node"))
        .find(|path| path.is_absolute() && path.is_file())
        .expect("CI must install Node for real process tests")
}
#[test]
fn reject_bad_configuration_and_failed_worker() {
    let dir = tempfile::tempdir().unwrap();
    let config = ProcessVerifierConfig {
        node: node(),
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

#[test]
fn transport_delivers_a_bounded_multi_proof_bundle_without_accepting_it() {
    let dir = tempfile::tempdir().unwrap();
    let receipt = dir.path().join("size.json");
    let config = ProcessVerifierConfig {
        node: node(),
        script: dir.path().join("worker.mjs"),
        artifact_config: receipt.clone(),
        scope: ProofScope {
            circuit_digest: [1; 32],
            verifying_key_digest: [2; 32],
        },
        timeout: Duration::from_secs(5),
        maximum_parallel: 1,
        max_proof_bytes: 4 * 1024 * 1024,
        node_heap_megabytes: 64,
    };
    // This worker records only the received byte count and always refuses proof
    // acceptance. The test checks pipe framing/capacity, not cryptography.
    fs::write(&config.script, "import {writeFileSync} from 'node:fs'; const chunks=[]; for await(const c of process.stdin) chunks.push(c); const r=JSON.parse(Buffer.concat(chunks)); writeFileSync(process.argv[2], String(r.proof.length/2)); process.exitCode=1;").unwrap();
    let verifier = ProcessVerifier::new(config.clone()).unwrap();
    assert_eq!(
        verifier.verify(&1, &vec![5; 2 * 1024 * 1024]),
        Err(Error::CryptoProvider)
    );
    assert_eq!(
        fs::read_to_string(&receipt).unwrap(),
        (2 * 1024 * 1024).to_string()
    );
    assert_eq!(
        verifier.verify(&1, &vec![5; 4 * 1024 * 1024 + 1]),
        Err(Error::InvalidInput)
    );
    let mut oversized = config;
    oversized.max_proof_bytes += 1;
    assert!(ProcessVerifier::new(oversized).is_err());
}
