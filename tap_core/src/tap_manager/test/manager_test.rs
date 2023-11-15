// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
mod manager_unit_test {
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
        sync::Arc,
    };

    use alloy_primitives::Address;
    use alloy_sol_types::{eip712_domain, Eip712Domain};
    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;
    use tokio::sync::RwLock;

    use super::super::Manager;
    use crate::{
        adapters::{
            escrow_adapter_mock::EscrowAdapterMock,
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
        // Alloy library does not have feature parity with ethers library (yet) This workaround is needed to get the address
        // to convert to an alloy Address. This will not be needed when the alloy library has wallet support.
        let address: [u8; 20] = wallet.address().into();

        (wallet, address.into())
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
    fn sender_ids() -> Vec<Address> {
        vec![
            Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
            Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
            Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
            keys().1,
        ]
    }

    #[fixture]
    fn domain_separator() -> Eip712Domain {
        eip712_domain! {
            name: "TAP",
            version: "1",
            chain_id: 1,
            verifying_contract: Address::from([0x11u8; 20]),
        }
    }

    #[fixture]
    fn rav_storage_adapter() -> RAVStorageAdapterMock {
        let rav_storage = Arc::new(RwLock::new(None));

        RAVStorageAdapterMock::new(rav_storage)
    }

    #[fixture]
    fn escrow_adapters() -> (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>) {
        let sender_escrow_storage = Arc::new(RwLock::new(HashMap::new()));
        let escrow_adapter = EscrowAdapterMock::new(Arc::clone(&sender_escrow_storage));
        (escrow_adapter, sender_escrow_storage)
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
        let sender_ids_set = Arc::new(RwLock::new(HashSet::from_iter(sender_ids())));
        let query_appraisal_storage = Arc::new(RwLock::new(HashMap::new()));

        let receipt_checks_adapter = ReceiptChecksAdapterMock::new(
            Arc::clone(&receipt_storage),
            Arc::clone(&query_appraisal_storage),
            Arc::clone(&allocation_ids_set),
            Arc::clone(&sender_ids_set),
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
        escrow_adapters: (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        #[case] initial_checks: Vec<ReceiptCheck>,
    ) {
        let (escrow_adapter, escrow_storage) = escrow_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let manager = Manager::new(
            domain_separator.clone(),
            escrow_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );

        let query_id = 1;
        let value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], value).unwrap(),
            &keys.0,
        )
        .await
        .unwrap();
        query_appraisal_storage
            .write()
            .await
            .insert(query_id, value);
        escrow_storage.write().await.insert(keys.1, 999999);

        assert!(manager
            .verify_and_store_receipt(signed_receipt, query_id, initial_checks.as_slice())
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
        escrow_adapters: (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        #[case] initial_checks: Vec<ReceiptCheck>,
    ) {
        let (escrow_adapter, escrow_storage) = escrow_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let manager = Manager::new(
            domain_separator.clone(),
            escrow_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );
        escrow_storage.write().await.insert(keys.1, 999999);

        let mut stored_signed_receipts = Vec::new();
        for query_id in 0..10 {
            let value = 20u128;
            let signed_receipt = EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(allocation_ids[0], value).unwrap(),
                &keys.0,
            )
            .await
            .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.as_slice())
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

        let signed_rav =
            EIP712SignedMessage::new(&domain_separator, rav_request.expected_rav.clone(), &keys.0)
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
        escrow_adapters: (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        #[case] initial_checks: Vec<ReceiptCheck>,
    ) {
        let (escrow_adapter, escrow_storage) = escrow_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let manager = Manager::new(
            domain_separator.clone(),
            escrow_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );

        escrow_storage.write().await.insert(keys.1, 999999);

        let mut stored_signed_receipts = Vec::new();
        let mut expected_accumulated_value = 0;
        for query_id in 0..10 {
            let value = 20u128;
            let signed_receipt = EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(allocation_ids[0], value).unwrap(),
                &keys.0,
            )
            .await
            .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.as_slice())
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

        let signed_rav =
            EIP712SignedMessage::new(&domain_separator, rav_request.expected_rav.clone(), &keys.0)
                .await
                .unwrap();
        assert!(manager
            .verify_and_store_rav(rav_request.expected_rav, signed_rav)
            .await
            .is_ok());

        stored_signed_receipts.clear();
        for query_id in 10..20 {
            let value = 20u128;
            let signed_receipt = EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(allocation_ids[0], value).unwrap(),
                &keys.0,
            )
            .await
            .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.as_slice())
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

        let signed_rav =
            EIP712SignedMessage::new(&domain_separator, rav_request.expected_rav.clone(), &keys.0)
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
        escrow_adapters: (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        #[values(get_full_list_of_checks(), vec![ReceiptCheck::CheckSignature], Vec::<ReceiptCheck>::new())]
        initial_checks: Vec<ReceiptCheck>,
        #[values(true, false)] remove_old_receipts: bool,
    ) {
        let (escrow_adapter, escrow_storage) = escrow_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let manager = Manager::new(
            domain_separator.clone(),
            escrow_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );

        escrow_storage.write().await.insert(keys.1, 999999);

        let mut stored_signed_receipts = Vec::new();
        let mut expected_accumulated_value = 0;
        for query_id in 0..10 {
            let value = 20u128;
            let mut receipt = Receipt::new(allocation_ids[0], value).unwrap();
            receipt.timestamp_ns = starting_min_timestamp + query_id + 1;
            let signed_receipt = EIP712SignedMessage::new(&domain_separator, receipt, &keys.0)
                .await
                .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.as_slice())
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

        let signed_rav_1 = EIP712SignedMessage::new(
            &domain_separator,
            rav_request_1.expected_rav.clone(),
            &keys.0,
        )
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
            let signed_receipt = EIP712SignedMessage::new(&domain_separator, receipt, &keys.0)
                .await
                .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .await
                .insert(query_id, value);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.as_slice())
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

        let signed_rav_2 = EIP712SignedMessage::new(
            &domain_separator,
            rav_request_2.expected_rav.clone(),
            &keys.0,
        )
        .await
        .unwrap();
        assert!(manager
            .verify_and_store_rav(rav_request_2.expected_rav, signed_rav_2)
            .await
            .is_ok());
    }
}
