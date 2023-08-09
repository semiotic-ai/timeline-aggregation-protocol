// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod received_receipt_unit_test {
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

    use crate::{
        adapters::{
            escrow_adapter_mock::EscrowAdapterMock,
            receipt_checks_adapter_mock::ReceiptChecksAdapterMock,
            receipt_storage_adapter_mock::ReceiptStorageAdapterMock,
        },
        eip_712_signed_message::EIP712SignedMessage,
        get_current_timestamp_u64_ns,
        tap_receipt::{
            get_full_list_of_checks,
            received_receipt::{RAVStatus, ReceiptState},
            Receipt, ReceiptAuditor, ReceiptCheck, ReceivedReceipt,
        },
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
    fn escrow_adapters() -> (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>) {
        let gateway_escrow_storage = Arc::new(RwLock::new(HashMap::new()));
        let escrow_adapter = EscrowAdapterMock::new(Arc::clone(&gateway_escrow_storage));
        (escrow_adapter, gateway_escrow_storage)
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

    #[rstest]
    #[tokio::test]
    async fn initialization_valid_receipt(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
    ) {
        let signed_receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], 10).unwrap(),
            &keys.0,
        )
        .await
        .unwrap();
        let query_id = 1;
        let checks = get_full_list_of_checks();

        let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);

        assert!(received_receipt.completed_checks_with_errors().is_empty());
        assert!(received_receipt.incomplete_checks().len() == checks.len());
        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);
    }

    #[rstest]
    #[tokio::test]
    async fn partial_then_full_check_valid_receipt(
        keys: (LocalWallet, Address),
        domain_separator: Eip712Domain,
        allocation_ids: Vec<Address>,
        escrow_adapters: (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
    ) {
        let (_, receipt_checks_adapter, query_appraisal_storage) = receipt_adapters;
        let (escrow_adapter, escrow_storage) = escrow_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;
        let receipt_auditor = ReceiptAuditor::new(
            domain_separator.clone(),
            escrow_adapter,
            receipt_checks_adapter,
            starting_min_timestamp,
        );

        let query_value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], query_value).unwrap(),
            &keys.0,
        )
        .await
        .unwrap();

        let query_id = 1;

        // prepare adapters and storage to correctly validate receipt

        // add escrow for gateway
        escrow_storage
            .write()
            .await
            .insert(keys.1, query_value + 500);
        // appraise query
        query_appraisal_storage
            .write()
            .await
            .insert(query_id, query_value);

        let checks = get_full_list_of_checks();
        let mut received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);
        let receipt_id = 0u64;

        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        // perform single arbitrary check
        let arbitrary_check_to_perform = ReceiptCheck::CheckUnique;
        assert!(received_receipt
            .perform_check(&arbitrary_check_to_perform, receipt_id, &receipt_auditor)
            .await
            .is_ok());

        assert!(received_receipt
            .perform_checks(&checks, receipt_id, &receipt_auditor)
            .await
            .is_ok());

        assert_eq!(received_receipt.state, ReceiptState::Accepted);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);
    }

    #[rstest]
    #[tokio::test]
    async fn partial_then_finalize_valid_receipt(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        escrow_adapters: (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
    ) {
        let (_, receipt_checks_adapter, query_appraisal_storage) = receipt_adapters;
        let (escrow_adapter, escrow_storage) = escrow_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;
        let receipt_auditor = ReceiptAuditor::new(
            domain_separator.clone(),
            escrow_adapter,
            receipt_checks_adapter,
            starting_min_timestamp,
        );

        let query_value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], query_value).unwrap(),
            &keys.0,
        )
        .await
        .unwrap();

        let query_id = 1;

        // prepare adapters and storage to correctly validate receipt

        // add escrow for gateway
        escrow_storage
            .write()
            .await
            .insert(keys.1, query_value + 500);
        // appraise query
        query_appraisal_storage
            .write()
            .await
            .insert(query_id, query_value);

        let checks = get_full_list_of_checks();
        let mut received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);
        let receipt_id = 0u64;

        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        // perform single arbitrary check
        let arbitrary_check_to_perform = ReceiptCheck::CheckUnique;
        assert!(received_receipt
            .perform_check(&arbitrary_check_to_perform, receipt_id, &receipt_auditor)
            .await
            .is_ok());

        assert!(received_receipt
            .finalize_receipt_checks(receipt_id, &receipt_auditor)
            .await
            .is_ok());

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
    #[tokio::test]
    async fn standard_lifetime_valid_receipt(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        escrow_adapters: (EscrowAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
    ) {
        let (_, receipt_checks_adapter, query_appraisal_storage) = receipt_adapters;
        let (escrow_adapter, escrow_storage) = escrow_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;
        let receipt_auditor = ReceiptAuditor::new(
            domain_separator.clone(),
            escrow_adapter,
            receipt_checks_adapter,
            starting_min_timestamp,
        );

        let query_value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], query_value).unwrap(),
            &keys.0,
        )
        .await
        .unwrap();

        let query_id = 1;

        // prepare adapters and storage to correctly validate receipt

        // add escrow for gateway
        escrow_storage
            .write()
            .await
            .insert(keys.1, query_value + 500);
        // appraise query
        query_appraisal_storage
            .write()
            .await
            .insert(query_id, query_value);

        let checks = get_full_list_of_checks();
        let mut received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);
        let receipt_id = 0u64;

        assert_eq!(received_receipt.state, ReceiptState::Received);
        assert_eq!(received_receipt.rav_status, RAVStatus::NotIncluded);

        assert!(received_receipt
            .finalize_receipt_checks(receipt_id, &receipt_auditor)
            .await
            .is_ok());

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
