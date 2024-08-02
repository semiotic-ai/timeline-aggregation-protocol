// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::RwLock;
use std::{str::FromStr, sync::Arc};

use alloy::dyn_abi::Eip712Domain;
use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use rstest::*;

use tap_core::manager::context::memory::InMemoryContext;
use tap_core::{
    manager::adapters::{RAVRead, RAVStore},
    rav::ReceiptAggregateVoucher,
    receipt::{checks::StatefulTimestampCheck, Receipt},
    signed_message::EIP712SignedMessage,
    tap_eip712_domain,
};

#[fixture]
fn domain_separator() -> Eip712Domain {
    tap_eip712_domain(1, Address::from([0x11u8; 20]))
}

#[fixture]
fn context() -> InMemoryContext {
    let escrow_storage = Arc::new(RwLock::new(HashMap::new()));
    let rav_storage = Arc::new(RwLock::new(None));
    let receipt_storage = Arc::new(RwLock::new(HashMap::new()));

    let timestamp_check = Arc::new(StatefulTimestampCheck::new(0));
    InMemoryContext::new(
        rav_storage,
        receipt_storage.clone(),
        escrow_storage.clone(),
        timestamp_check,
    )
}

#[rstest]
#[tokio::test]
async fn rav_storage_adapter_test(domain_separator: Eip712Domain, context: InMemoryContext) {
    let wallet = PrivateKeySigner::random();

    let allocation_id = Address::from_str("0xabababababababababababababababababababab").unwrap();

    // Create receipts
    let mut receipts = Vec::new();
    for value in 50..60 {
        receipts.push(
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(allocation_id, value).unwrap(),
                &wallet,
            )
            .unwrap(),
        );
    }

    let signed_rav = EIP712SignedMessage::new(
        &domain_separator,
        ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None).unwrap(),
        &wallet,
    )
    .unwrap();

    context.update_last_rav(signed_rav.clone()).await.unwrap();

    // Retreive rav
    let retrieved_rav = context.last_rav().await;
    assert!(retrieved_rav.unwrap().unwrap() == signed_rav);

    // Testing the last rav update...

    // Create more receipts
    let mut receipts = Vec::new();
    for value in 60..70 {
        receipts.push(
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(allocation_id, value).unwrap(),
                &wallet,
            )
            .unwrap(),
        );
    }

    let signed_rav = EIP712SignedMessage::new(
        &domain_separator,
        ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None).unwrap(),
        &wallet,
    )
    .unwrap();

    // Update the last rav
    context.update_last_rav(signed_rav.clone()).await.unwrap();

    // Retreive rav
    let retrieved_rav = context.last_rav().await;
    assert!(retrieved_rav.unwrap().unwrap() == signed_rav);
}
