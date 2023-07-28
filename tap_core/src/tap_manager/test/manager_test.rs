// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod manager_unit_test {
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
        sync::Arc,
    };

    use ethereum_types::Address;
    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;
    use tokio::sync::RwLock;

    use super::super::Manager;
    use crate::{
        adapters::{
            collateral_adapter_mock::CollateralAdapterMock,
            rav_storage_adapter_mock::RAVStorageAdapterMock,
            receipt_checks_adapter_mock::ReceiptChecksAdapterMock,
            receipt_storage_adapter::ReceiptStorageAdapter,
            receipt_storage_adapter_mock::ReceiptStorageAdapterMock,
        },
        eip_712_signed_message::EIP712SignedMessage,
        get_current_timestamp_u64_ns,
        tap_receipt::{get_full_list_of_checks, Receipt, ReceiptCheck},
    };

    #[fixture]
    fn keys() -> (LocalWallet, Address) {
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
        let address = wallet.address();
        (wallet, address)
    }

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
    fn gateway_ids() -> Vec<Address> {
        vec![
            Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
            Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
            Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
            keys().1,
        ]
    }

    #[fixture]
    fn rav_storage_adapter() -> RAVStorageAdapterMock {
        let rav_storage = Arc::new(RwLock::new(None));

        RAVStorageAdapterMock::new(rav_storage)
    }

    #[fixture]
    fn collateral_adapters() -> (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>) {
        let gateway_collateral_storage = Arc::new(RwLock::new(HashMap::new()));
        let collateral_adapter =
            CollateralAdapterMock::new(Arc::clone(&gateway_collateral_storage));
        (collateral_adapter, gateway_collateral_storage)
    }

    #[fixture]
    fn receipt_adapters() -> (
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        Arc<RwLock<HashMap<u64, u128>>>,
    ) {
        let receipt_storage = Arc::new(RwLock::new(HashMap::new()));
        let receipt_storage_adapter = ReceiptStorageAdapterMock::new(Arc::clone(&receipt_storage));

        let allocation_ids_set = Arc::new(RwLock::new(HashSet::from_iter(allocation_ids())));
        let gateway_ids_set = Arc::new(RwLock::new(HashSet::from_iter(gateway_ids())));
        let query_appraisal_storage = Arc::new(RwLock::new(HashMap::new()));

        let receipt_checks_adapter = ReceiptChecksAdapterMock::new(
            Arc::clone(&receipt_storage),
            Arc::clone(&query_appraisal_storage),
            Arc::clone(&allocation_ids_set),
            Arc::clone(&gateway_ids_set),
        );

        (
            receipt_storage_adapter,
            receipt_checks_adapter,
            query_appraisal_storage,
        )
    }

    #[rstest]
    #[case::full_checks(get_full_list_of_checks())]
    #[case::partial_checks(vec![ReceiptCheck::CheckSignature])]
    #[case::no_checks(Vec::<ReceiptCheck>::new())]
    #[tokio::test]
    async fn manager_verify_and_store_varying_initial_checks(
        rav_storage_adapter: RAVStorageAdapterMock,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        #[case] initial_checks: Vec<ReceiptCheck>,
    ) {
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let manager = Manager::new(
            collateral_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );

        let query_id = 1;
        let value = 20u128;
        let signed_receipt =
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                .await
                .unwrap();
        query_appraisal_storage
            .write()
            .await
            .insert(query_id, value);
        collateral_storage.write().await.insert(keys.1, 999999);

        assert!(manager
            .verify_and_store_receipt(signed_receipt, query_id, initial_checks)
            .await
            .is_ok());
    }

    #[rstest]
    #[case::full_checks(get_full_list_of_checks())]
    #[case::partial_checks(vec![ReceiptCheck::CheckSignature])]
    #[case::no_checks(Vec::<ReceiptCheck>::new())]
    #[tokio::test]
    async fn manager_create_rav_request_all_valid_receipts(
        rav_storage_adapter: RAVStorageAdapterMock,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        #[case] initial_checks: Vec<ReceiptCheck>,
    ) {
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let manager = Manager::new(
            collateral_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );
        collateral_storage.write().await.insert(keys.1, 999999);

        let mut stored_signed_receipts = Vec::new();
        for query_id in 0..10 {
            let value = 20u128;
            let signed_receipt =
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .await
                    .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.clone())
                .await
                .is_ok());
        }
        let rav_request_result = manager.create_rav_request(0).await;
        assert!(rav_request_result.is_ok());

        let rav_request = rav_request_result.unwrap();
        // all passing
        assert_eq!(
            rav_request.valid_receipts.len(),
            stored_signed_receipts.len()
        );
        // no failing
        assert_eq!(rav_request.invalid_receipts.len(), 0);

        let signed_rav = EIP712SignedMessage::new(rav_request.expected_rav.clone(), &keys.0)
            .await
            .unwrap();
        assert!(manager
            .verify_and_store_rav(rav_request.expected_rav, signed_rav)
            .await
            .is_ok());
    }

    #[rstest]
    #[case::full_checks(get_full_list_of_checks())]
    #[case::partial_checks(vec![ReceiptCheck::CheckSignature])]
    #[case::no_checks(Vec::<ReceiptCheck>::new())]
    #[tokio::test]
    async fn manager_create_multiple_rav_requests_all_valid_receipts(
        rav_storage_adapter: RAVStorageAdapterMock,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        #[case] initial_checks: Vec<ReceiptCheck>,
    ) {
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let manager = Manager::new(
            collateral_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );

        collateral_storage.write().await.insert(keys.1, 999999);

        let mut stored_signed_receipts = Vec::new();
        let mut expected_accumulated_value = 0;
        for query_id in 0..10 {
            let value = 20u128;
            let signed_receipt =
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .await
                    .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.clone())
                .await
                .is_ok());
            expected_accumulated_value += value;
        }
        let rav_request_result = manager.create_rav_request(0).await;
        assert!(rav_request_result.is_ok());

        let rav_request = rav_request_result.unwrap();
        // all receipts passing
        assert_eq!(
            rav_request.valid_receipts.len(),
            stored_signed_receipts.len()
        );
        // no receipts failing
        assert_eq!(rav_request.invalid_receipts.len(), 0);
        // accumulated value is correct
        assert_eq!(
            rav_request.expected_rav.value_aggregate,
            expected_accumulated_value
        );
        // no previous rav
        assert!(rav_request.previous_rav.is_none());

        let signed_rav = EIP712SignedMessage::new(rav_request.expected_rav.clone(), &keys.0)
            .await
            .unwrap();
        assert!(manager
            .verify_and_store_rav(rav_request.expected_rav, signed_rav)
            .await
            .is_ok());

        stored_signed_receipts.clear();
        for query_id in 10..20 {
            let value = 20u128;
            let signed_receipt =
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .await
                    .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.clone())
                .await
                .is_ok());
            expected_accumulated_value += value;
        }
        let rav_request_result = manager.create_rav_request(0).await;
        assert!(rav_request_result.is_ok());

        let rav_request = rav_request_result.unwrap();
        // all receipts passing
        assert_eq!(
            rav_request.valid_receipts.len(),
            stored_signed_receipts.len()
        );
        // no receipts failing
        assert_eq!(rav_request.invalid_receipts.len(), 0);
        // accumulated value is correct
        assert_eq!(
            rav_request.expected_rav.value_aggregate,
            expected_accumulated_value
        );
        // Verify there is a previous rav
        assert!(rav_request.previous_rav.is_some());

        let signed_rav = EIP712SignedMessage::new(rav_request.expected_rav.clone(), &keys.0)
            .await
            .unwrap();
        assert!(manager
            .verify_and_store_rav(rav_request.expected_rav, signed_rav)
            .await
            .is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn manager_create_multiple_rav_requests_all_valid_receipts_consecutive_timestamps(
        rav_storage_adapter: RAVStorageAdapterMock,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        #[values(get_full_list_of_checks(), vec![ReceiptCheck::CheckSignature], Vec::<ReceiptCheck>::new())]
        initial_checks: Vec<ReceiptCheck>,
        #[values(true, false)] remove_old_receipts: bool,
    ) {
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let mut manager = Manager::new(
            collateral_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );

        collateral_storage.write().await.insert(keys.1, 999999);

        let mut stored_signed_receipts = Vec::new();
        let mut expected_accumulated_value = 0;
        for query_id in 0..10 {
            let value = 20u128;
            let mut receipt = Receipt::new(allocation_ids[0], value).unwrap();
            receipt.timestamp_ns = starting_min_timestamp + query_id + 1;
            let signed_receipt = EIP712SignedMessage::new(receipt, &keys.0).await.unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.clone())
                .await
                .is_ok());
            expected_accumulated_value += value;
        }

        // Remove old receipts if requested
        // This shouldn't do anything since there has been no rav created yet
        if remove_old_receipts {
            manager.remove_obsolete_receipts().await.unwrap();
        }

        let rav_request_1_result = manager.create_rav_request(0).await;
        assert!(rav_request_1_result.is_ok());

        let rav_request_1 = rav_request_1_result.unwrap();
        // all receipts passing
        assert_eq!(
            rav_request_1.valid_receipts.len(),
            stored_signed_receipts.len()
        );
        // no receipts failing
        assert_eq!(rav_request_1.invalid_receipts.len(), 0);
        // accumulated value is correct
        assert_eq!(
            rav_request_1.expected_rav.value_aggregate,
            expected_accumulated_value
        );
        // no previous rav
        assert!(rav_request_1.previous_rav.is_none());

        let signed_rav_1 = EIP712SignedMessage::new(rav_request_1.expected_rav.clone(), &keys.0)
            .await
            .unwrap();
        assert!(manager
            .verify_and_store_rav(rav_request_1.expected_rav, signed_rav_1)
            .await
            .is_ok());

        stored_signed_receipts.clear();
        for query_id in 10..20 {
            let value = 20u128;
            let mut receipt = Receipt::new(allocation_ids[0], value).unwrap();
            receipt.timestamp_ns = starting_min_timestamp + query_id + 1;
            let signed_receipt = EIP712SignedMessage::new(receipt, &keys.0).await.unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.clone())
                .await
                .is_ok());
            expected_accumulated_value += value;
        }

        // Remove old receipts if requested
        if remove_old_receipts {
            manager.remove_obsolete_receipts().await.unwrap();
            // We expect to have 10 receipts left in receipt storage
            assert_eq!(
                manager
                    .receipt_storage_adapter
                    .retrieve_receipts_in_timestamp_range(..)
                    .await
                    .unwrap()
                    .len(),
                10
            );
        }

        let rav_request_2_result = manager.create_rav_request(0).await;
        assert!(rav_request_2_result.is_ok());

        let rav_request_2 = rav_request_2_result.unwrap();
        // all receipts passing
        assert_eq!(
            rav_request_2.valid_receipts.len(),
            stored_signed_receipts.len()
        );
        // no receipts failing
        assert_eq!(rav_request_2.invalid_receipts.len(), 0);
        // accumulated value is correct
        assert_eq!(
            rav_request_2.expected_rav.value_aggregate,
            expected_accumulated_value
        );
        // Verify there is a previous rav
        assert!(rav_request_2.previous_rav.is_some());

        let signed_rav_2 = EIP712SignedMessage::new(rav_request_2.expected_rav.clone(), &keys.0)
            .await
            .unwrap();
        assert!(manager
            .verify_and_store_rav(rav_request_2.expected_rav, signed_rav_2)
            .await
            .is_ok());
    }
}
