#[cfg(test)]
mod received_receipt_unit_test {
    use crate::{
        eip_712_signed_message::EIP712SignedMessage,
        tap_receipt::{
            get_full_list_of_checks,
            received_receipt::{RAVStatus, ReceiptState},
            Receipt, ReceiptCheck, ReceivedReceipt,
        },
    };
    use ethereum_types::Address;
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rand_core::OsRng;
    use rstest::*;
    use std::str::FromStr;

    #[fixture]
    fn allocation_ids() -> Vec<Address> {
        vec![
            Address::from_str("0xabababababababababababababababababababab").unwrap(),
            Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead").unwrap(),
            Address::from_str("0xbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef").unwrap(),
            Address::from_str("0x1234567890abcdef1234567890abcdef12345678").unwrap(),
        ]
    }

    #[fixture]
    fn keys() -> (SigningKey, VerifyingKey) {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        (signing_key, verifying_key)
    }

    #[rstest]
    fn test_initialization(keys: (SigningKey, VerifyingKey), allocation_ids: Vec<Address>) {
        let signed_receipt = EIP712SignedMessage::new(
            Receipt::new(allocation_ids[0].clone(), 10).unwrap(),
            &keys.0,
        )
        .unwrap();
        let query_id = 1;
        let checks = get_full_list_of_checks();

        let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, checks.clone());

        assert!(received_receipt.completed_checks_with_errors().len() == 0);
        assert!(received_receipt.incomplete_checks().len() == checks.len());
        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);
    }

    #[rstest]
    fn test_initialization_with_some_checks_with_ok(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
    ) {
        let signed_receipt = EIP712SignedMessage::new(
            Receipt::new(allocation_ids[0].clone(), 10).unwrap(),
            &keys.0,
        )
        .unwrap();
        let query_id = 1;
        let mut checks = get_full_list_of_checks();
        // Set a check to passing
        checks.insert(ReceiptCheck::CheckUnique, Some(Ok(())));

        let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, checks.clone());

        assert!(received_receipt.completed_checks_with_errors().len() == 0);
        assert!(received_receipt.incomplete_checks().len() == (checks.len() - 1));
        assert_eq!(received_receipt.state, ReceiptState::Checking);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);
    }

    #[rstest]
    fn test_initialization_with_some_checks_with_err(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
    ) {
        let signed_receipt = EIP712SignedMessage::new(
            Receipt::new(allocation_ids[0].clone(), 10).unwrap(),
            &keys.0,
        )
        .unwrap();
        let query_id = 1;
        let mut checks = get_full_list_of_checks();
        // Set a check to fail
        let check_to_fail = ReceiptCheck::CheckUnique;
        let cause_of_fail = Err(crate::Error::InvalidValue {
            received_value: 10,
            expected_value: 20,
        });
        checks.insert(check_to_fail.clone(), Some(cause_of_fail));

        let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, checks.clone());

        assert!(received_receipt.completed_checks_with_errors().len() == 1);
        assert!(received_receipt
            .completed_checks_with_errors()
            .get(&check_to_fail)
            .is_some());
        assert_eq!(received_receipt.state, ReceiptState::Failed);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);
    }

    #[rstest]
    fn test_initialization_all_checks_complete_with_ok(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
    ) {
        let signed_receipt = EIP712SignedMessage::new(
            Receipt::new(allocation_ids[0].clone(), 10).unwrap(),
            &keys.0,
        )
        .unwrap();
        let query_id = 1;
        let mut checks = get_full_list_of_checks();
        // Set all checks to complete and passing
        for (_, result) in checks.iter_mut() {
            *result = Some(Ok(()));
        }
        let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, checks.clone());

        assert!(received_receipt.completed_checks_with_errors().len() == 0);
        assert!(received_receipt.incomplete_checks().len() == 0);
        assert_eq!(received_receipt.state, ReceiptState::Accepted);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);
    }

    #[rstest]
    fn test_full_lifetime_with_valid_receipt(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
    ) {
        let signed_receipt = EIP712SignedMessage::new(
            Receipt::new(allocation_ids[0].clone(), 10).unwrap(),
            &keys.0,
        )
        .unwrap();
        let query_id = 1;
        let checks = get_full_list_of_checks();
        let mut checks_to_complete: Vec<ReceiptCheck> = get_full_list_of_checks()
            .into_iter()
            .map(|(check, _)| check)
            .collect();

        let mut received_receipt = ReceivedReceipt::new(signed_receipt, query_id, checks.clone());

        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        // Set all checks to complete and passing
        while let Some(check) = checks_to_complete.pop() {
            received_receipt.update_check(check, Some(Ok(()))).unwrap();
            // As each check is added make sure the incomplete_check list matches the checks left to complete
            let incomplete_checks = received_receipt.incomplete_checks();
            assert!(incomplete_checks
                .iter()
                .all(|(check, _)| checks_to_complete.contains(check)));
            assert!(incomplete_checks.iter().all(|(_, result)| result.is_none()));
        }

        assert_eq!(received_receipt.state, ReceiptState::Accepted);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        received_receipt.update_rav_status(RAVStatus::IncludedInRequest);

        assert_eq!(received_receipt.state, ReceiptState::IncludedInRAVRequest);
        assert_eq!(received_receipt.rav_status, RAVStatus::IncludedInRequest);

        received_receipt.update_rav_status(RAVStatus::IncludedInReceived);

        assert_eq!(received_receipt.state, ReceiptState::Complete);
        assert_eq!(received_receipt.rav_status, RAVStatus::IncludedInReceived);
    }
}
