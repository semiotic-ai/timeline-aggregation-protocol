// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use alloy_primitives::Address;
use alloy_sol_types::Eip712Domain;
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
use rstest::*;

fn get_current_timestamp_u64_ns() -> anyhow::Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64)
}

use tap_core::{
    manager::{
        adapters::ReceiptRead,
        context::memory::{
            checks::get_full_list_of_checks, EscrowStorage, InMemoryContext, QueryAppraisals,
        },
        Manager,
    },
    rav::ReceiptAggregateVoucher,
    receipt::{
        checks::{CheckList, StatefulTimestampCheck},
        Receipt,
    },
    signed_message::EIP712SignedMessage,
    tap_eip712_domain,
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
    tap_eip712_domain(1, Address::from([0x11u8; 20]))
}

struct ContextFixture {
    context: InMemoryContext,
    escrow_storage: EscrowStorage,
    query_appraisals: QueryAppraisals,
    checks: CheckList,
}

#[fixture]
fn context(
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
    sender_ids: Vec<Address>,
    keys: (LocalWallet, Address),
) -> ContextFixture {
    let escrow_storage = Arc::new(RwLock::new(HashMap::new()));
    let rav_storage = Arc::new(RwLock::new(None));
    let query_appraisals = Arc::new(RwLock::new(HashMap::new()));
    let receipt_storage = Arc::new(RwLock::new(HashMap::new()));
    let timestamp_check = Arc::new(StatefulTimestampCheck::new(0));
    let context = InMemoryContext::new(
        rav_storage,
        receipt_storage.clone(),
        escrow_storage.clone(),
        timestamp_check.clone(),
    )
    .with_sender_address(keys.1);

    let mut checks = get_full_list_of_checks(
        domain_separator,
        sender_ids.iter().cloned().collect(),
        Arc::new(RwLock::new(allocation_ids.iter().cloned().collect())),
        query_appraisals.clone(),
    );
    checks.push(timestamp_check);
    let checks = CheckList::new(checks);

    ContextFixture {
        context,
        escrow_storage,
        query_appraisals,
        checks,
    }
}

#[rstest]
#[tokio::test]
async fn manager_verify_and_store_varying_initial_checks(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        context,
        checks,
        query_appraisals,
        escrow_storage,
        ..
    } = context;
    let manager = Manager::new(domain_separator.clone(), context, checks);

    let value = 20u128;
    let signed_receipt = EIP712SignedMessage::new(
        &domain_separator,
        Receipt::new(allocation_ids[0], value).unwrap(),
        &keys.0,
    )
    .unwrap();
    let query_id = signed_receipt.unique_hash();
    query_appraisals.write().unwrap().insert(query_id, value);
    escrow_storage.write().unwrap().insert(keys.1, 999999);

    assert!(manager
        .verify_and_store_receipt(signed_receipt)
        .await
        .is_ok());
}

#[rstest]
#[tokio::test]
async fn manager_create_rav_request_all_valid_receipts(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        context,
        checks,
        query_appraisals,
        escrow_storage,
        ..
    } = context;
    let manager = Manager::new(domain_separator.clone(), context, checks);
    escrow_storage.write().unwrap().insert(keys.1, 999999);

    let mut stored_signed_receipts = Vec::new();
    for _ in 0..10 {
        let value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], value).unwrap(),
            &keys.0,
        )
        .unwrap();
        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(signed_receipt)
            .await
            .is_ok());
    }
    let rav_request_result = manager.create_rav_request(0, None).await;
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
            .unwrap();
    assert!(manager
        .verify_and_store_rav(rav_request.expected_rav, signed_rav)
        .await
        .is_ok());
}

#[rstest]
#[tokio::test]
async fn deny_rav_due_to_wrong_value(
    keys: (LocalWallet, Address),
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        context, checks, ..
    } = context;
    let manager = Manager::new(domain_separator.clone(), context, checks);

    let rav = ReceiptAggregateVoucher {
        allocationId: Address::from_str("0xabababababababababababababababababababab").unwrap(),
        timestampNs: 1232442,
        valueAggregate: 20u128,
    };

    let rav_wrong_value = ReceiptAggregateVoucher {
        allocationId: Address::from_str("0xabababababababababababababababababababab").unwrap(),
        timestampNs: 1232442,
        valueAggregate: 10u128,
    };

    let signed_rav_with_wrong_aggregate =
        EIP712SignedMessage::new(&domain_separator, rav_wrong_value, &keys.0).unwrap();

    assert!(manager
        .verify_and_store_rav(rav, signed_rav_with_wrong_aggregate)
        .await
        .is_err());
}

#[rstest]
#[tokio::test]
async fn manager_create_multiple_rav_requests_all_valid_receipts(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        context,
        checks,
        query_appraisals,
        escrow_storage,
        ..
    } = context;

    let manager = Manager::new(domain_separator.clone(), context, checks);

    escrow_storage.write().unwrap().insert(keys.1, 999999);

    let mut stored_signed_receipts = Vec::new();
    let mut expected_accumulated_value = 0;
    for _ in 0..10 {
        let value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], value).unwrap(),
            &keys.0,
        )
        .unwrap();
        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(signed_receipt)
            .await
            .is_ok());
        expected_accumulated_value += value;
    }
    let rav_request_result = manager.create_rav_request(0, None).await;
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
        rav_request.expected_rav.valueAggregate,
        expected_accumulated_value
    );
    // no previous rav
    assert!(rav_request.previous_rav.is_none());

    let signed_rav =
        EIP712SignedMessage::new(&domain_separator, rav_request.expected_rav.clone(), &keys.0)
            .unwrap();
    assert!(manager
        .verify_and_store_rav(rav_request.expected_rav, signed_rav)
        .await
        .is_ok());

    stored_signed_receipts.clear();
    for _ in 10..20 {
        let value = 20u128;
        let signed_receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], value).unwrap(),
            &keys.0,
        )
        .unwrap();

        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(signed_receipt)
            .await
            .is_ok());
        expected_accumulated_value += value;
    }
    let rav_request_result = manager.create_rav_request(0, None).await;
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
        rav_request.expected_rav.valueAggregate,
        expected_accumulated_value
    );
    // Verify there is a previous rav
    assert!(rav_request.previous_rav.is_some());

    let signed_rav =
        EIP712SignedMessage::new(&domain_separator, rav_request.expected_rav.clone(), &keys.0)
            .unwrap();
    assert!(manager
        .verify_and_store_rav(rav_request.expected_rav, signed_rav)
        .await
        .is_ok());
}

#[rstest]
#[tokio::test]
async fn manager_create_multiple_rav_requests_all_valid_receipts_consecutive_timestamps(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    #[values(true, false)] remove_old_receipts: bool,
    context: ContextFixture,
) {
    let ContextFixture {
        context,
        checks,
        query_appraisals,
        escrow_storage,
        ..
    } = context;
    let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

    let manager = Manager::new(domain_separator.clone(), context.clone(), checks);

    escrow_storage.write().unwrap().insert(keys.1, 999999);

    let mut stored_signed_receipts = Vec::new();
    let mut expected_accumulated_value = 0;
    for query_id in 0..10 {
        let value = 20u128;
        let mut receipt = Receipt::new(allocation_ids[0], value).unwrap();
        receipt.timestamp_ns = starting_min_timestamp + query_id + 1;
        let signed_receipt = EIP712SignedMessage::new(&domain_separator, receipt, &keys.0).unwrap();

        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(signed_receipt)
            .await
            .is_ok());
        expected_accumulated_value += value;
    }

    // Remove old receipts if requested
    // This shouldn't do anything since there has been no rav created yet
    if remove_old_receipts {
        manager.remove_obsolete_receipts().await.unwrap();
    }

    let rav_request_1_result = manager.create_rav_request(0, None).await;
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
        rav_request_1.expected_rav.valueAggregate,
        expected_accumulated_value
    );
    // no previous rav
    assert!(rav_request_1.previous_rav.is_none());

    let signed_rav_1 = EIP712SignedMessage::new(
        &domain_separator,
        rav_request_1.expected_rav.clone(),
        &keys.0,
    )
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
        let signed_receipt = EIP712SignedMessage::new(&domain_separator, receipt, &keys.0).unwrap();
        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(signed_receipt)
            .await
            .is_ok());
        expected_accumulated_value += value;
    }

    // Remove old receipts if requested
    if remove_old_receipts {
        manager.remove_obsolete_receipts().await.unwrap();
        // We expect to have 10 receipts left in receipt storage
        assert_eq!(
            context
                .retrieve_receipts_in_timestamp_range(.., None)
                .await
                .unwrap()
                .len(),
            10
        );
    }

    let rav_request_2_result = manager.create_rav_request(0, None).await;
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
        rav_request_2.expected_rav.valueAggregate,
        expected_accumulated_value
    );
    // Verify there is a previous rav
    assert!(rav_request_2.previous_rav.is_some());

    let signed_rav_2 = EIP712SignedMessage::new(
        &domain_separator,
        rav_request_2.expected_rav.clone(),
        &keys.0,
    )
    .unwrap();
    assert!(manager
        .verify_and_store_rav(rav_request_2.expected_rav, signed_rav_2)
        .await
        .is_ok());
}

#[rstest]
#[tokio::test]
async fn manager_create_rav_and_ignore_invalid_receipts(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        context,
        checks,
        escrow_storage,
        ..
    } = context;

    let manager = Manager::new(domain_separator.clone(), context.clone(), checks);

    escrow_storage.write().unwrap().insert(keys.1, 999999);

    let mut stored_signed_receipts = Vec::new();
    //Forcing all receipts but one to be invalid by making all the same
    for _ in 0..10 {
        let receipt = Receipt {
            allocation_id: allocation_ids[0],
            timestamp_ns: 1,
            nonce: 1,
            value: 20u128,
        };
        let signed_receipt = EIP712SignedMessage::new(&domain_separator, receipt, &keys.0).unwrap();
        stored_signed_receipts.push(signed_receipt.clone());
        manager
            .verify_and_store_receipt(signed_receipt)
            .await
            .unwrap();
    }

    let rav_request = manager.create_rav_request(0, None).await.unwrap();

    assert_eq!(rav_request.valid_receipts.len(), 1);
    // All receipts but one being invalid
    assert_eq!(rav_request.invalid_receipts.len(), 9);
    //Rav Value corresponds only to value of one receipt
    assert_eq!(rav_request.expected_rav.valueAggregate, 20);
}
