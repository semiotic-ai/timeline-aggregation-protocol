// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rstest::*;
use tap_core::{
    manager::context::memory::{checks::get_full_list_of_checks, EscrowStorage, QueryAppraisals},
    receipt::{
        checks::{ReceiptCheck, StatefulTimestampCheck},
        Context, ReceiptWithState,
    },
    signed_message::Eip712SignedMessage,
    tap_eip712_domain,
};
use tap_graph::v2::{Receipt, SignedReceipt};
use thegraph_core::alloy::{
    dyn_abi::Eip712Domain,
    primitives::{address, Address, U256},
    signers::local::PrivateKeySigner,
};

#[fixture]
fn signer() -> PrivateKeySigner {
    PrivateKeySigner::random()
}

#[fixture]
fn allocation_ids() -> Vec<Address> {
    vec![
        address!("0xabababababababababababababababababababab"),
        address!("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead"),
        address!("0xbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef"),
        address!("0x1234567890abcdef1234567890abcdef12345678"),
    ]
}

#[fixture]
fn sender_ids(signer: PrivateKeySigner) -> (PrivateKeySigner, Vec<Address>) {
    let address = signer.address();
    (
        signer,
        vec![
            address!("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb"),
            address!("0xfafafafafafafafafafafafafafafafafafafafa"),
            address!("0xadadadadadadadadadadadadadadadadadadadad"),
            address,
        ],
    )
}

#[fixture]
fn domain_separator() -> Eip712Domain {
    tap_eip712_domain(1, Address::from([0x11u8; 20]))
}

struct ContextFixture {
    escrow_storage: EscrowStorage,
    query_appraisals: QueryAppraisals,
    checks: Vec<ReceiptCheck<SignedReceipt>>,
    signer: PrivateKeySigner,
}

#[fixture]
fn context(
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
    sender_ids: (PrivateKeySigner, Vec<Address>),
) -> ContextFixture {
    let (signer, sender_ids) = sender_ids;
    let escrow_storage = Arc::new(RwLock::new(HashMap::new()));
    let query_appraisals = Arc::new(RwLock::new(HashMap::new()));

    let timestamp_check = Arc::new(StatefulTimestampCheck::new(0));
    let mut checks = get_full_list_of_checks(
        domain_separator,
        sender_ids.iter().cloned().collect(),
        Arc::new(RwLock::new(allocation_ids.iter().cloned().collect())),
        query_appraisals.clone(),
    );
    checks.push(timestamp_check);

    ContextFixture {
        signer,
        escrow_storage,
        query_appraisals,
        checks,
    }
}

#[rstest]
#[tokio::test]
async fn partial_then_full_check_valid_receipt(
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
    context: ContextFixture,
) {
    let ContextFixture {
        checks,
        escrow_storage,
        query_appraisals,
        signer,
        ..
    } = context;

    let query_value = U256::from(20u128);
    let signed_receipt = Eip712SignedMessage::new(
        &domain_separator,
        Receipt::new(
            allocation_ids[0],
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            query_value,
        )
        .unwrap(),
        &signer,
    )
    .unwrap();

    let query_id = signed_receipt.unique_hash();

    // add escrow for sender
    escrow_storage
        .write()
        .unwrap()
        .insert(signer.address(), query_value + U256::from(500u128));
    // appraise query
    query_appraisals
        .write()
        .unwrap()
        .insert(query_id, query_value);

    let mut received_receipt = ReceiptWithState::new(signed_receipt);

    let result = received_receipt
        .perform_checks(&Context::new(), &checks)
        .await;
    assert!(result.is_ok());
}

#[rstest]
#[tokio::test]
async fn partial_then_finalize_valid_receipt(
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        checks,
        escrow_storage,
        query_appraisals,
        signer,
        ..
    } = context;

    let query_value = U256::from(20u128);
    let signed_receipt = Eip712SignedMessage::new(
        &domain_separator,
        Receipt::new(
            allocation_ids[0],
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            query_value,
        )
        .unwrap(),
        &signer,
    )
    .unwrap();
    let query_id = signed_receipt.unique_hash();

    // add escrow for sender
    escrow_storage
        .write()
        .unwrap()
        .insert(signer.address(), query_value + U256::from(500u128));
    // appraise query
    query_appraisals
        .write()
        .unwrap()
        .insert(query_id, query_value);

    let received_receipt = ReceiptWithState::new(signed_receipt);

    let awaiting_escrow_receipt = received_receipt
        .finalize_receipt_checks(&Context::new(), &checks)
        .await;
    assert!(awaiting_escrow_receipt.is_ok());

    let checked_receipt = awaiting_escrow_receipt.unwrap();
    assert!(checked_receipt.is_ok());
}

#[rstest]
#[tokio::test]
async fn standard_lifetime_valid_receipt(
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    context: ContextFixture,
) {
    let ContextFixture {
        checks,
        escrow_storage,
        query_appraisals,
        signer,
        ..
    } = context;

    let query_value = U256::from(20u128);
    let signed_receipt = Eip712SignedMessage::new(
        &domain_separator,
        Receipt::new(
            allocation_ids[0],
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            query_value,
        )
        .unwrap(),
        &signer,
    )
    .unwrap();

    let query_id = signed_receipt.unique_hash();

    // add escrow for sender
    escrow_storage
        .write()
        .unwrap()
        .insert(signer.address(), query_value + U256::from(500u128));
    // appraise query
    query_appraisals
        .write()
        .unwrap()
        .insert(query_id, query_value);

    let received_receipt = ReceiptWithState::new(signed_receipt);

    let awaiting_escrow_receipt = received_receipt
        .finalize_receipt_checks(&Context::new(), &checks)
        .await;
    assert!(awaiting_escrow_receipt.is_ok());

    let checked_receipt = awaiting_escrow_receipt.unwrap();
    assert!(checked_receipt.is_ok());
}
