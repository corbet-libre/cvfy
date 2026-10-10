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
