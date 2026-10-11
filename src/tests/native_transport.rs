use super::*;
#[test]
fn observing_success_at_the_deadline_cannot_extend_it() {
    let start = Instant::now();
    let deadline = start + Duration::from_secs(1);
    assert_eq!(check_deadline(start, deadline), Ok(()));
    assert_eq!(
        check_deadline(deadline, deadline),
        Err(Error::CryptoProvider)
    );
    assert_eq!(
        check_deadline(deadline + Duration::from_nanos(1), deadline),
        Err(Error::CryptoProvider)
    );
}

// The fixture helper is test-only in both the unit and integration harnesses.
#[path = "../../tests/support/account_fixture.rs"]
mod account_fixture;

#[test]
fn a_genuine_proof_observed_after_its_deadline_is_refused() {
    use czkp::native::{Proof, Request};
    use data_encoding::HEXLOWER;
    let (config, records) = account_fixture::fixture();
    assert_eq!(config.scope, records[0].proof_scope);
    let proofs = [Proof {
        public_inputs: &records[0].public_inputs,
        proof: &records[0].proof,
    }];
    let key = HEXLOWER.encode(&config.verification_key);
    let g1 = HEXLOWER.encode(&config.srs_g1);
    let g2 = HEXLOWER.encode(&config.srs_g2);
    let bytes = serde_json::to_vec(&Request {
        scope: &config.scope,
        verification_key: &key,
        srs_g1: &g1,
        srs_g2: &g2,
        proofs: &proofs,
    })
    .unwrap();
    let deadline = Instant::now() + config.timeout;
    assert_eq!(
        verify_with_completion_clock(
            &config,
            bytes.clone(),
            deadline,
            &NativeCancellation::default(),
            Instant::now
        ),
        Ok(())
    );
    // Only the observation clock advances; the same real native verifier must
    // finish successfully. A failed proof cannot make this refusal test pass.
    let deadline = Instant::now() + config.timeout;
    assert_eq!(
        verify_with_completion_clock(
            &config,
            bytes,
            deadline,
            &NativeCancellation::default(),
            || deadline
        ),
        Err(Error::CryptoProvider),
        "late completion must not authorize a valid proof"
    );
}
