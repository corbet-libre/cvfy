//! Real retained account proofs qualify the maintained native SDK and public inputs.
use cvfy::{NativeVerifier, NativeVerifierConfig};
use czkp::{ProofScope, native::SRS_G1_BYTES};
use data_encoding::HEXLOWER;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    public_inputs: Vec<[u8; 32]>,
    proof: String,
    proof_scope: ProofScope,
}

#[test]
fn maintained_account_proofs_bind_all_inputs_and_refuse_tampering() {
    let source = PathBuf::from(
        std::env::var_os("CVFY_FIXTURE_SOURCE").expect("CI must provide real proof fixtures"),
    );
    let directory = source.join(".proof-test");
    let records: Vec<Record> =
        serde_json::from_slice(&fs::read(directory.join("native-records.json")).unwrap()).unwrap();
    assert_eq!(records.len(), 2);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    let key = fs::read(directory.join("vk.bin")).unwrap();
    let mut g1 = fs::read(directory.join("setup/g1.dat")).unwrap();
    let g2 = fs::read(directory.join("setup/g2.dat")).unwrap();
    for (name, bytes) in [("g1.dat", &g1), ("g2.dat", &g2)] {
        let item = manifest["setup"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == name)
            .unwrap();
        assert_eq!(item["bytes"].as_u64().unwrap() as usize, bytes.len());
        assert_eq!(
            item["sha256"].as_str().unwrap(),
            HEXLOWER.encode(&Sha256::digest(bytes))
        );
    }
    g1.truncate(SRS_G1_BYTES);
    let config = NativeVerifierConfig {
        executable: PathBuf::from(
            std::env::var_os("CVFY_BB").expect("CI must build the native worker"),
        ),
        scope: records[0].proof_scope.clone(),
        timeout: Duration::from_secs(180),
        maximum_parallel: 1,
        max_proof_bytes: 1024 * 1024,
        max_public_inputs: 243,
        threads: 1,
        memory_megabytes: 4096,
        verification_key: key,
        srs_g1: g1,
        srs_g2: g2.try_into().unwrap(),
    };
    let verifier = NativeVerifier::new(config.clone()).unwrap();
    let proofs: Vec<_> = records
        .iter()
        .map(|record| HEXLOWER.decode(record.proof.as_bytes()).unwrap())
        .collect();
    let batch: Vec<_> = records
        .iter()
        .zip(&proofs)
        .map(|(record, proof)| {
            assert_eq!(record.proof_scope, verifier.scope());
            assert_eq!(record.public_inputs.len(), 243);
            (record.public_inputs.as_slice(), proof.as_slice())
        })
        .collect();
    assert!(
        verifier
            .verify_batch_before(&batch, Instant::now() + Duration::from_secs(180))
            .is_ok()
    );
    for (record, proof) in records.iter().zip(&proofs) {
        assert!(verifier.verify(&record.public_inputs, proof).is_ok());
        let mut corrupt = proof.clone();
        corrupt[0] ^= 1;
        assert!(verifier.verify(&record.public_inputs, &corrupt).is_err());
        let mut changed = record.public_inputs.clone();
        changed[0][31] ^= 1;
        assert!(verifier.verify(&changed, proof).is_err());
    }
    // An invalid later step cannot be hidden by an earlier genuine success.
    let mut corrupt = proofs[1].clone();
    corrupt[0] ^= 1;
    assert!(
        verifier
            .verify_batch_before(
                &[(batch[0].0, batch[0].1), (batch[1].0, &corrupt)],
                Instant::now() + Duration::from_secs(180)
            )
            .is_err()
    );
    let mut wrong_key = config;
    wrong_key.verification_key[0] ^= 1;
    wrong_key.scope.verifying_key_digest = Sha256::digest(&wrong_key.verification_key).into();
    assert!(
        NativeVerifier::new(wrong_key)
            .unwrap()
            .verify(batch[0].0, batch[0].1)
            .is_err()
    );
}
