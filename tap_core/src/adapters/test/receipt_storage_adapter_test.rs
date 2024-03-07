// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod receipt_storage_adapter_unit_test {
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    use std::collections::{HashMap, HashSet};
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};

    use crate::checks::TimestampCheck;
    use crate::{
        adapters::{executor_mock::ExecutorMock, receipt_storage_adapter::ReceiptStore},
        checks::{mock::get_full_list_of_checks, ReceiptCheck},
        eip_712_signed_message::EIP712SignedMessage,
        tap_eip712_domain,
        tap_receipt::{Receipt, ReceivedReceipt},
    };
    use alloy_primitives::Address;
    use alloy_sol_types::Eip712Domain;
    use ethers::signers::coins_bip39::English;
    use ethers::signers::{LocalWallet, MnemonicBuilder};
    use rstest::*;

    #[fixture]
    fn domain_separator() -> Eip712Domain {
        tap_eip712_domain(1, Address::from([0x11u8; 20]))
    }

    struct ExecutorFixture {
        executor: ExecutorMock,
        checks: Vec<ReceiptCheck>,
    }

    #[fixture]
    fn executor_mock(domain_separator: Eip712Domain) -> ExecutorFixture {
        let escrow_storage = Arc::new(RwLock::new(HashMap::new()));
        let rav_storage = Arc::new(RwLock::new(None));
        let query_appraisals = Arc::new(RwLock::new(HashMap::new()));
        let receipt_storage = Arc::new(RwLock::new(HashMap::new()));

        let timestamp_check = Arc::new(TimestampCheck::new(0));
        let executor = ExecutorMock::new(
            rav_storage,
            receipt_storage.clone(),
            escrow_storage.clone(),
            timestamp_check.clone(),
        );
        let mut checks = get_full_list_of_checks(
            domain_separator,
            HashSet::new(),
            Arc::new(RwLock::new(HashSet::new())),
            receipt_storage,
            query_appraisals.clone(),
        );
        checks.push(timestamp_check);

        ExecutorFixture { executor, checks }
    }

    #[rstest]
    #[tokio::test]
    async fn receipt_adapter_test(domain_separator: Eip712Domain, executor_mock: ExecutorFixture) {
        let ExecutorFixture {
            mut executor,
            checks,
        } = executor_mock;

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();

        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();

        // Create receipts
        let query_id = 10u64;
        let value = 100u128;
        let received_receipt = ReceivedReceipt::new(
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(allocation_id, value).unwrap(),
                &wallet,
            )
            .unwrap(),
            query_id,
            &checks,
        );

        let receipt_store_result = executor.store_receipt(received_receipt).await;
        assert!(receipt_store_result.is_ok());
        let receipt_id = receipt_store_result.unwrap();

        // Retreive receipt with id expected to be valid
        assert!(executor.retrieve_receipt_by_id(receipt_id).await.is_ok());
        // Retreive receipt with arbitrary id expected to be invalid
        assert!(executor.retrieve_receipt_by_id(999).await.is_err());

        // Remove receipt with id expected to be valid
        assert!(executor.remove_receipt_by_id(receipt_id).await.is_ok());
        // Remove receipt with arbitrary id expected to be invalid
        assert!(executor.remove_receipt_by_id(999).await.is_err());

        // Retreive receipt that was removed previously
        assert!(executor.retrieve_receipt_by_id(receipt_id).await.is_err());

        // Remove receipt that was removed previously
        assert!(executor.remove_receipt_by_id(receipt_id).await.is_err());
    }

    #[rstest]
    #[tokio::test]
    async fn multi_receipt_adapter_test(
        domain_separator: Eip712Domain,
        executor_mock: ExecutorFixture,
    ) {
        let ExecutorFixture {
            mut executor,
            checks,
        } = executor_mock;

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();

        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();

        // Create receipts
        let mut received_receipts = Vec::new();
        for (query_id, value) in (50..60).enumerate() {
            received_receipts.push(ReceivedReceipt::new(
                EIP712SignedMessage::new(
                    &domain_separator,
                    Receipt::new(allocation_id, value).unwrap(),
                    &wallet,
                )
                .unwrap(),
                query_id as u64,
                &checks,
            ));
        }
        let mut receipt_ids = Vec::new();
        let mut receipt_timestamps = Vec::new();
        for received_receipt in received_receipts {
            receipt_ids.push(
                executor
                    .store_receipt(received_receipt.clone())
                    .await
                    .unwrap(),
            );
            receipt_timestamps.push(received_receipt.signed_receipt().message.timestamp_ns)
        }

        // Retreive receipts with timestamp
        assert!(executor
            .retrieve_receipts_by_timestamp(receipt_timestamps[0])
            .await
            .is_ok());
        assert!(!executor
            .retrieve_receipts_by_timestamp(receipt_timestamps[0])
            .await
            .unwrap()
            .is_empty());

        // Retreive receipts before timestamp
        assert!(executor
            .retrieve_receipts_upto_timestamp(receipt_timestamps[3])
            .await
            .is_ok());
        assert!(
            executor
                .retrieve_receipts_upto_timestamp(receipt_timestamps[3])
                .await
                .unwrap()
                .len()
                >= 4
        );

        // Remove all receipts with one call
        assert!(executor
            .remove_receipts_by_ids(receipt_ids.as_slice())
            .await
            .is_ok());
        // Removal should no longer be valid
        assert!(executor
            .remove_receipts_by_ids(receipt_ids.as_slice())
            .await
            .is_err());
        // Retrieval should be invalid
        for receipt_id in receipt_ids {
            assert!(executor.retrieve_receipt_by_id(receipt_id).await.is_err());
        }
    }

    /// The test code will shuffle the input timestamps prior to calling safe_truncate_receipts.
    #[rstest]
    #[case(vec![1, 2, 3, 4, 5], 3, vec![1, 2, 3])]
    #[case(vec![1, 2, 3, 3, 4, 5], 3, vec![1, 2])]
    #[case(vec![1, 2, 3, 4, 4, 4], 3, vec![1, 2, 3])]
    #[case(vec![1, 1, 1, 1, 2, 3], 3, vec![])]
    #[test]
    fn safe_truncate_receipts_test(
        domain_separator: Eip712Domain,
        executor_mock: ExecutorFixture,
        #[case] input: Vec<u64>,
        #[case] limit: u64,
        #[case] expected: Vec<u64>,
    ) {
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
        let checks = executor_mock.checks;

        // Vec of (id, receipt)
        let mut receipts_orig: Vec<(u64, ReceivedReceipt)> = Vec::new();

        for (i, timestamp) in input.iter().enumerate() {
            // The contents of the receipt only need to be unique for this test (so we can check)
            receipts_orig.push((
                i as u64,
                ReceivedReceipt::new(
                    EIP712SignedMessage::new(
                        &domain_separator,
                        Receipt {
                            allocation_id: Address::ZERO,
                            timestamp_ns: *timestamp,
                            nonce: 0,
                            value: 0,
                        },
                        &wallet,
                    )
                    .unwrap(),
                    i as u64, // Will use that to check the IDs
                    &checks,
                ),
            ));
        }

        let mut receipts_truncated = receipts_orig.clone();

        // shuffle the input receipts
        receipts_truncated.shuffle(&mut thread_rng());

        crate::adapters::receipt_storage_adapter::safe_truncate_receipts(
            &mut receipts_truncated,
            limit,
        );

        assert_eq!(receipts_truncated.len(), expected.len());

        for (elem_trun, expected_timestamp) in receipts_truncated.iter().zip(expected.iter()) {
            // Check timestamps
            assert_eq!(
                elem_trun.1.signed_receipt().message.timestamp_ns,
                *expected_timestamp
            );

            // Check that the IDs are fine
            assert_eq!(elem_trun.0, elem_trun.1.query_id());
        }
    }
}
