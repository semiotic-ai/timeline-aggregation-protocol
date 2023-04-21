#[cfg(test)]
mod receipt_unit_test {
    use crate::tap_receipt::Receipt;
    use ethereum_types::Address;
    use rstest::*;
    use std::str::FromStr;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[fixture]
    fn allocation_ids() -> Vec<Address> {
        vec![
            Address::from_str("0xabababababababababababababababababababab").unwrap(),
            Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead").unwrap(),
            Address::from_str("0xbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef").unwrap(),
            Address::from_str("0x1234567890abcdef1234567890abcdef12345678").unwrap(),
        ]
    }

    #[rstest]
    fn test_new_receipt(allocation_ids: Vec<Address>) {
        let value = 1234;

        let receipt = Receipt::new(allocation_ids[0].clone(), value).unwrap();

        assert_eq!(receipt.allocation_id, allocation_ids[0]);
        assert_eq!(receipt.value, value);

        // Check that the timestamp is within a reasonable range
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Current system time should be greater than `UNIX_EPOCH`")
            .as_millis() as u64;
        assert!(receipt.timestamp_ns <= now);
        assert!(receipt.timestamp_ns >= now - 5000); // 5 second tolerance
    }

    #[rstest]
    fn test_unique_nonce_and_timestamp(allocation_ids: Vec<Address>) {
        let value = 1234;

        let receipt1 = Receipt::new(allocation_ids[0].clone(), value).unwrap();
        let receipt2 = Receipt::new(allocation_ids[0].clone(), value).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Current system time should be greater than `UNIX_EPOCH`")
            .as_millis() as u64;

        // Check that nonces are different
        // Note: This test has an *extremely low* (~1/2^64) probability of false failure, if a failure happens
        //       once it is not neccessarily a sign of an issue. If this test fails more than once, especially
        //       in a short period of time (within a ) then there may be an issue with randomness
        //       of the nonce generation.
        assert_ne!(receipt1.nonce, receipt2.nonce);

        assert!(receipt1.timestamp_ns <= now);
        assert!(receipt1.timestamp_ns >= now - 5000); // 5 second tolerance

        assert!(receipt2.timestamp_ns <= now);
        assert!(receipt2.timestamp_ns >= now - 5000); // 5 second tolerance
    }
}
