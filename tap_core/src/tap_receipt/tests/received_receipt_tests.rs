// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod received_receipt_unit_test {
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
        sync::{Arc, RwLock},
    };

    use ethereum_types::Address;
    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;

    use crate::{
        adapters::{
            receipt_checks_adapter_mock::ReceiptChecksAdapterMock,
            receipt_storage_adapter_mock::ReceiptStorageAdapterMock, collateral_adapter_mock::CollateralAdapterMock,
        },
        eip_712_signed_message::EIP712SignedMessage,
        tap_receipt::{
            received_receipt::{RAVStatus, ReceiptState},
            Receipt, ReceiptCheck, ReceivedReceipt, ReceiptAuditor, get_full_list_of_checks,
        }, get_current_timestamp_u64_ns,
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

    #[fixture]
    fn collateral_adapters() -> (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>) {
        let gateway_collateral_storage = Arc::new(RwLock::new(HashMap::new()));
        let collateral_adapter = CollateralAdapterMock::new(Arc::clone(&gateway_collateral_storage));
        (collateral_adapter, gateway_collateral_storage)
    }

    #[rstest]
    async fn initialization_valid_receipt(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
    ) {
        let signed_receipt = EIP712SignedMessage::new(
            Receipt::new(allocation_ids[0].clone(), 10).unwrap(),
            &keys.0,
        )
        .await
        .unwrap();
        let query_id = 1;
        let checks = get_full_list_of_checks();

        let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);

        assert!(received_receipt.completed_checks_with_errors().len() == 0);
        assert!(received_receipt.incomplete_checks().len() == checks.len());
        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);
    }

    #[rstest]
    async fn partial_then_full_check_valid_receipt(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
    ) {
        let (_, receipt_checks_adapter, query_appraisal_storage) = receipt_adapters;
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let now = get_current_timestamp_u64_ns().unwrap();
        let mut receipt_auditor = ReceiptAuditor::new(collateral_adapter, receipt_checks_adapter, now);

        let query_value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
                Receipt::new(allocation_ids[0].clone(), query_value).unwrap(),
                &keys.0,
            )
            .await
            .unwrap();

        let query_id = 1;

        // prepare adapters and storage to correctly validate receipt

        // add collateral for gateway
        collateral_storage.write().unwrap().insert(keys.1, query_value+500);
        // appraise query
        query_appraisal_storage.write().unwrap().insert(query_id, query_value);

        let checks = get_full_list_of_checks();
        let mut received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);

        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        // perform single arbitrary check
        let arbitrary_check_to_perform = ReceiptCheck::CheckUnique;
        assert!(received_receipt.perform_check(&arbitrary_check_to_perform, &mut receipt_auditor).is_ok());

        assert!(received_receipt.perform_checks(&checks, &mut receipt_auditor).is_ok());

        assert_eq!(received_receipt.state, ReceiptState::Accepted);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);
    }

    #[rstest]
    async fn partial_then_finalize_valid_receipt(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
    ) {
        let (_, receipt_checks_adapter, query_appraisal_storage) = receipt_adapters;
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let now = get_current_timestamp_u64_ns().unwrap();
        let mut receipt_auditor = ReceiptAuditor::new(collateral_adapter, receipt_checks_adapter, now);

        let query_value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
                Receipt::new(allocation_ids[0].clone(), query_value).unwrap(),
                &keys.0,
            )
            .await
            .unwrap();

        let query_id = 1;

        // prepare adapters and storage to correctly validate receipt

        // add collateral for gateway
        collateral_storage.write().unwrap().insert(keys.1, query_value+500);
        // appraise query
        query_appraisal_storage.write().unwrap().insert(query_id, query_value);

        let checks = get_full_list_of_checks();
        let mut received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);

        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        // perform single arbitrary check
        let arbitrary_check_to_perform = ReceiptCheck::CheckUnique;
        assert!(received_receipt.perform_check(&arbitrary_check_to_perform, &mut receipt_auditor).is_ok());

        assert!(received_receipt.finalize_receipt_checks(&mut receipt_auditor).is_ok());

        assert_eq!(received_receipt.state, ReceiptState::Accepted);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        received_receipt.update_rav_status(RAVStatus::IncludedInRequest);

        assert_eq!(received_receipt.state, ReceiptState::IncludedInRAVRequest);
        assert_eq!(received_receipt.rav_status, RAVStatus::IncludedInRequest);

        received_receipt.update_rav_status(RAVStatus::IncludedInReceived);

        assert_eq!(received_receipt.state, ReceiptState::Complete);
        assert_eq!(received_receipt.rav_status, RAVStatus::IncludedInReceived);
    }

        #[rstest]
    async fn standard_lifetime_valid_receipt(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
    ) {
        let (_, receipt_checks_adapter, query_appraisal_storage) = receipt_adapters;
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let now = get_current_timestamp_u64_ns().unwrap();
        let mut receipt_auditor = ReceiptAuditor::new(collateral_adapter, receipt_checks_adapter, now);

        let query_value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
                Receipt::new(allocation_ids[0].clone(), query_value).unwrap(),
                &keys.0,
            )
            .await
            .unwrap();

        let query_id = 1;

        // prepare adapters and storage to correctly validate receipt

        // add collateral for gateway
        collateral_storage.write().unwrap().insert(keys.1, query_value+500);
        // appraise query
        query_appraisal_storage.write().unwrap().insert(query_id, query_value);

        let checks = get_full_list_of_checks();
        let mut received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);

        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        assert!(received_receipt.finalize_receipt_checks(&mut receipt_auditor).is_ok());

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
