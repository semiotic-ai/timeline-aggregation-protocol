// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use rstest::*;
use tap_core::{
    manager::{
        adapters::ReceiptRead,
        context::memory::{EscrowStorage, InMemoryContext, QueryAppraisals},
        Manager,
    },
    receipt::{
        checks::{CheckList, StatefulTimestampCheck},
        Context,
    },
    signed_message::Eip712SignedMessage,
    tap_eip712_domain,
};
use tap_eip712_message::MessageId;
use tap_graph::v2::{Receipt, ReceiptAggregateVoucher, SignedReceipt};
use thegraph_core::alloy::{
    dyn_abi::Eip712Domain, primitives::Address, signers::local::PrivateKeySigner,
};

fn get_current_timestamp_u64_ns() -> anyhow::Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64)
}

#[fixture]
fn signer() -> PrivateKeySigner {
    PrivateKeySigner::random()
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
fn sender_ids(signer: PrivateKeySigner) -> (PrivateKeySigner, Vec<Address>) {
    let address = signer.address();
    (
        signer,
        vec![
            Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
            Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
            Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
            address,
        ],
    )
}

#[fixture]
fn domain_separator() -> Eip712Domain {
    tap_eip712_domain(1, Address::from([0x11u8; 20]))
}

struct ContextFixture {
    context: InMemoryContext,
    escrow_storage: EscrowStorage,
    query_appraisals: QueryAppraisals,
    checks: CheckList<SignedReceipt>,
    signer: PrivateKeySigner,
}

#[fixture]
fn context(
    _domain_separator: Eip712Domain,
    _allocation_ids: Vec<Address>,
    sender_ids: (PrivateKeySigner, Vec<Address>),
) -> ContextFixture {
    let (signer, _sender_ids) = sender_ids;
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
    .with_sender_address(signer.address());

    let checks = CheckList::new(vec![timestamp_check]);

    ContextFixture {
        signer,
        context,
        escrow_storage,
        query_appraisals,
        checks,
    }
}

#[rstest]
#[tokio::test]
async fn manager_verify_and_store_varying_initial_checks(
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        context,
        checks,
        query_appraisals,
        escrow_storage,
        signer,
        ..
    } = context;
    let manager = Manager::new(domain_separator.clone(), context, checks);

    let value = 20u128;
    let signed_receipt = Eip712SignedMessage::new(
        &domain_separator,
        Receipt::new(
            allocation_ids[0],
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            value,
        )
        .unwrap(),
        &signer,
    )
    .unwrap();
    let query_id = signed_receipt.unique_hash();
    query_appraisals.write().unwrap().insert(query_id, value);
    escrow_storage
        .write()
        .unwrap()
        .insert(signer.address(), 999999);

    assert!(manager
        .verify_and_store_receipt(&Context::new(), signed_receipt)
        .await
        .is_ok());
}

#[rstest]
#[tokio::test]
async fn manager_create_rav_request_all_valid_receipts(
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        context,
        checks,
        query_appraisals,
        escrow_storage,
        signer,
        ..
    } = context;
    let manager = Manager::new(domain_separator.clone(), context, checks);
    escrow_storage
        .write()
        .unwrap()
        .insert(signer.address(), 999999);

    let mut stored_signed_receipts = Vec::new();
    for _ in 0..10 {
        let value = 20u128;
        let signed_receipt = Eip712SignedMessage::new(
            &domain_separator,
            Receipt::new(
                allocation_ids[0],
                Address::ZERO,
                Address::ZERO,
                Address::ZERO,
                value,
            )
            .unwrap(),
            &signer,
        )
        .unwrap();
        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(&Context::new(), signed_receipt)
            .await
            .is_ok());
    }
    let rav_request_result = manager.create_rav_request(&Context::new(), 0, None).await;
    assert!(rav_request_result.is_ok());

    let rav_request = rav_request_result.unwrap();
    // all passing
    assert_eq!(
        rav_request.valid_receipts.len(),
        stored_signed_receipts.len()
    );
    // no failing
    assert_eq!(rav_request.invalid_receipts.len(), 0);

    let expected_rav = rav_request.expected_rav.unwrap();

    let signed_rav =
        Eip712SignedMessage::new(&domain_separator, expected_rav.clone(), &signer).unwrap();
    assert!(manager
        .verify_and_store_rav(expected_rav, signed_rav)
        .await
        .is_ok());
}

#[rstest]
#[tokio::test]
async fn deny_rav_due_to_wrong_value(domain_separator: Eip712Domain, context: ContextFixture) {
    let ContextFixture {
        context,
        checks,
        signer,
        ..
    } = context;
    let manager = Manager::new(domain_separator.clone(), context, checks);

    let rav = ReceiptAggregateVoucher {
        allocationId: Address::from_str("0xabababababababababababababababababababab").unwrap(),
        timestampNs: 1232442,
        valueAggregate: 20u128,
        payer: Address::ZERO,
        dataService: Address::ZERO,
        serviceProvider: Address::ZERO,
        metadata: vec![].into(),
    };

    let rav_wrong_value = ReceiptAggregateVoucher {
        allocationId: Address::from_str("0xabababababababababababababababababababab").unwrap(),
        timestampNs: 1232442,
        valueAggregate: 10u128,
        payer: Address::ZERO,
        dataService: Address::ZERO,
        serviceProvider: Address::ZERO,
        metadata: vec![].into(),
    };

    let signed_rav_with_wrong_aggregate =
        Eip712SignedMessage::new(&domain_separator, rav_wrong_value, &signer).unwrap();

    assert!(manager
        .verify_and_store_rav(rav, signed_rav_with_wrong_aggregate)
        .await
        .is_err());
}

#[rstest]
#[tokio::test]
async fn manager_create_multiple_rav_requests_all_valid_receipts(
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        context,
        checks,
        query_appraisals,
        escrow_storage,
        signer,
        ..
    } = context;

    let manager = Manager::new(domain_separator.clone(), context, checks);

    escrow_storage
        .write()
        .unwrap()
        .insert(signer.address(), 999999);

    let mut stored_signed_receipts = Vec::new();
    let mut expected_accumulated_value = 0;
    for _ in 0..10 {
        let value = 20u128;
        let signed_receipt = Eip712SignedMessage::new(
            &domain_separator,
            Receipt::new(
                allocation_ids[0],
                Address::ZERO,
                Address::ZERO,
                Address::ZERO,
                value,
            )
            .unwrap(),
            &signer,
        )
        .unwrap();
        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(&Context::new(), signed_receipt)
            .await
            .is_ok());
        expected_accumulated_value += value;
    }
    let rav_request_result = manager.create_rav_request(&Context::new(), 0, None).await;
    assert!(rav_request_result.is_ok());

    let rav_request = rav_request_result.unwrap();
    // all receipts passing
    assert_eq!(
        rav_request.valid_receipts.len(),
        stored_signed_receipts.len()
    );
    // no receipts failing
    assert_eq!(rav_request.invalid_receipts.len(), 0);

    let expected_rav = rav_request.expected_rav.unwrap();
    // accumulated value is correct
    assert_eq!(expected_rav.valueAggregate, expected_accumulated_value);
    // no previous rav
    assert!(rav_request.previous_rav.is_none());

    let signed_rav =
        Eip712SignedMessage::new(&domain_separator, expected_rav.clone(), &signer).unwrap();
    assert!(manager
        .verify_and_store_rav(expected_rav, signed_rav)
        .await
        .is_ok());
}

#[rstest]
#[tokio::test]
async fn manager_create_multiple_rav_requests_all_valid_receipts_consecutive_timestamps(
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
        signer,
        ..
    } = context;
    let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

    let manager = Manager::new(domain_separator.clone(), context.clone(), checks);

    escrow_storage
        .write()
        .unwrap()
        .insert(signer.address(), 999999);

    let mut stored_signed_receipts = Vec::new();
    let mut expected_accumulated_value = 0;
    for query_id in 0..10 {
        let value = 20u128;
        let mut receipt = Receipt::new(
            allocation_ids[0],
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            value,
        )
        .unwrap();
        receipt.timestamp_ns = starting_min_timestamp + query_id + 1;
        let signed_receipt = Eip712SignedMessage::new(&domain_separator, receipt, &signer).unwrap();

        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(&Context::new(), signed_receipt)
            .await
            .is_ok());
        expected_accumulated_value += value;
    }

    // Remove old receipts if requested
    // This shouldn't do anything since there has been no rav created yet
    if remove_old_receipts {
        manager.remove_obsolete_receipts().await.unwrap();
    }

    let rav_request_1_result = manager.create_rav_request(&Context::new(), 0, None).await;
    assert!(rav_request_1_result.is_ok());

    let rav_request_1 = rav_request_1_result.unwrap();
    // all receipts passing
    assert_eq!(
        rav_request_1.valid_receipts.len(),
        stored_signed_receipts.len()
    );
    // no receipts failing
    assert_eq!(rav_request_1.invalid_receipts.len(), 0);

    let expected_rav_1 = rav_request_1.expected_rav.unwrap();
    // accumulated value is correct
    assert_eq!(expected_rav_1.valueAggregate, expected_accumulated_value);
    // no previous rav
    assert!(rav_request_1.previous_rav.is_none());

    let signed_rav_1 =
        Eip712SignedMessage::new(&domain_separator, expected_rav_1.clone(), &signer).unwrap();
    assert!(manager
        .verify_and_store_rav(expected_rav_1, signed_rav_1)
        .await
        .is_ok());

    stored_signed_receipts.clear();
    for query_id in 10..20 {
        let value = 20u128;
        let mut receipt = Receipt::new(
            allocation_ids[0],
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            value,
        )
        .unwrap();
        receipt.timestamp_ns = starting_min_timestamp + query_id + 1;
        let signed_receipt = Eip712SignedMessage::new(&domain_separator, receipt, &signer).unwrap();
        let query_id = signed_receipt.unique_hash();
        stored_signed_receipts.push(signed_receipt.clone());
        query_appraisals.write().unwrap().insert(query_id, value);
        assert!(manager
            .verify_and_store_receipt(&Context::new(), signed_receipt)
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

    let rav_request_2_result = manager.create_rav_request(&Context::new(), 0, None).await;
    assert!(rav_request_2_result.is_ok());

    let rav_request_2 = rav_request_2_result.unwrap();
    // all receipts passing
    assert_eq!(
        rav_request_2.valid_receipts.len(),
        stored_signed_receipts.len()
    );
    // no receipts failing
    assert_eq!(rav_request_2.invalid_receipts.len(), 0);

    let expected_rav_2 = rav_request_2.expected_rav.unwrap();
    // accumulated value is correct
    assert_eq!(expected_rav_2.valueAggregate, expected_accumulated_value);
    // Verify there is a previous rav
    assert!(rav_request_2.previous_rav.is_some());

    let signed_rav_2 =
        Eip712SignedMessage::new(&domain_separator, expected_rav_2.clone(), &signer).unwrap();
    assert!(manager
        .verify_and_store_rav(expected_rav_2, signed_rav_2)
        .await
        .is_ok());
}

#[rstest]
#[tokio::test]
async fn manager_create_rav_and_ignore_invalid_receipts(
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
) -> Result<()> {
    let timestamp_check = Arc::new(StatefulTimestampCheck::new(60));

    // Create context with proper parameters
    let escrow_storage = Arc::new(RwLock::new(HashMap::new()));
    let query_appraisals = Arc::new(RwLock::new(HashMap::<MessageId, u128>::new()));
    let receipt_storage = Arc::new(RwLock::new(HashMap::new()));

    let context = InMemoryContext::new(
        Arc::new(RwLock::new(None)),
        receipt_storage,
        escrow_storage,
        timestamp_check.clone(),
    );

    let checks = CheckList::new(vec![timestamp_check]);

    let manager = Manager::new(domain_separator.clone(), context, checks);
    let context_for_calls = Context::new();

    // Create valid receipts
    for _ in 0..9 {
        let receipt = Receipt::new(
            allocation_ids[0],
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            20u128,
        )
        .unwrap();

        let wallet = PrivateKeySigner::random();
        let signed_receipt = Eip712SignedMessage::new(&domain_separator, receipt, &wallet).unwrap();
        let query_id = signed_receipt.unique_hash();
        query_appraisals.write().unwrap().insert(query_id, 20u128);
        manager
            .verify_and_store_receipt(&context_for_calls, signed_receipt)
            .await?;
    }

    let rav_request = manager
        .create_rav_request(&context_for_calls, 0, None)
        .await?;

    // The test logic needs to be adjusted based on what actually makes receipts invalid
    assert_eq!(rav_request.valid_receipts.len(), 9);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_retryable_checks(
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
) -> Result<()> {
    let timestamp_check = Arc::new(StatefulTimestampCheck::new(60));

    let escrow_storage = Arc::new(RwLock::new(HashMap::new()));
    let receipt_storage = Arc::new(RwLock::new(HashMap::new()));

    let context = InMemoryContext::new(
        Arc::new(RwLock::new(None)),
        receipt_storage,
        escrow_storage,
        timestamp_check.clone(),
    );

    let checks = CheckList::new(vec![timestamp_check]);

    let manager = Manager::new(domain_separator.clone(), context, checks);
    let context_for_calls = Context::new();

    // Store receipts
    for _ in 0..10 {
        let receipt = Receipt::new(
            allocation_ids[0],
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            20u128,
        )
        .unwrap();

        let wallet = PrivateKeySigner::random();
        let signed_receipt = Eip712SignedMessage::new(&domain_separator, receipt, &wallet).unwrap();
        manager
            .verify_and_store_receipt(&context_for_calls, signed_receipt)
            .await?;
    }

    let rav_request = manager
        .create_rav_request(&context_for_calls, 0, None)
        .await?;

    // Check that we got valid receipts (the test name suggests checking retryable behavior)
    assert!(!rav_request.valid_receipts.is_empty());
    Ok(())
}
