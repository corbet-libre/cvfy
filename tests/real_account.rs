//! Real retained account proofs qualify the maintained native SDK and public inputs.
use cvfy::{NativeVerifier, NativeVerifierConfig};
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
#[path = "support/account_fixture.rs"]
mod account_fixture;

#[test]
fn maintained_account_proofs_bind_all_inputs_and_refuse_tampering() {
    let (config, records) = account_fixture::fixture();
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
    // The host-authenticated byte identity is checked before native dispatch.
    assert!(NativeVerifier::new(wrong_key.clone()).is_err());
    // An explicitly trusted but truncated key must still fail upstream parsing.
    wrong_key.verification_key.pop();
    wrong_key.scope.verifying_key_digest = Sha256::digest(&wrong_key.verification_key).into();
    assert!(
        NativeVerifier::new(wrong_key)
            .unwrap()
            .verify(batch[0].0, batch[0].1)
            .is_err()
    );
}
