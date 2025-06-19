// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
};

use rstest::*;
use tap_core::{
    manager::{
        adapters::{RavRead, RavStore},
        context::memory::InMemoryContext,
    },
    receipt::checks::StatefulTimestampCheck,
    signed_message::Eip712SignedMessage,
    tap_eip712_domain,
};
use tap_graph::v2::{Receipt, ReceiptAggregateVoucher};
#[allow(deprecated)]
use thegraph_core::alloy::primitives::{Address, Signature};
use thegraph_core::alloy::{dyn_abi::Eip712Domain, signers::local::PrivateKeySigner};

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
fn check_for_rav_serialization(domain_separator: Eip712Domain) {
    let allocation_id = Address::from_str("0xabababababababababababababababababababab").unwrap();
    let wallet = PrivateKeySigner::from_slice(&[1u8; 32]).unwrap();
    let mut receipts = Vec::new();

    for value in 50..60 {
        let mut receipt = Receipt::new(
            allocation_id,
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            value,
        )
        .unwrap();

        receipt.timestamp_ns = 1000000000 + value as u64;
        receipt.nonce = value as u64;

        receipts.push(Eip712SignedMessage::new(&domain_separator, receipt, &wallet).unwrap());
    }

    let signed_rav = Eip712SignedMessage::new(
        &domain_separator,
        ReceiptAggregateVoucher::aggregate_receipts(
            allocation_id,
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            &receipts,
            None,
        )
        .unwrap(),
        &wallet,
    )
    .unwrap();

    insta::assert_json_snapshot!(receipts);
    insta::assert_json_snapshot!(signed_rav);

    let raw_sig = r#"{
      "r": "0x1596dd0d380ede7aa5dec5ed09ea7d1fa8e4bc8dfdb43a4e965bb4f16906e321",
      "s": "0x788b69625a031fbd2e769928b63505387df16e7c51f19ff67c782bfec101a387",
      "yParity": "0x1"
    }"#;

    serde_json::from_str::<Signature>(raw_sig).unwrap();
    #[allow(deprecated)]
    serde_json::from_str::<Signature>(raw_sig).unwrap();
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
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(
                    allocation_id,
                    Address::ZERO,
                    Address::ZERO,
                    Address::ZERO,
                    value,
                )
                .unwrap(),
                &wallet,
            )
            .unwrap(),
        );
    }

    let signed_rav = Eip712SignedMessage::new(
        &domain_separator,
        ReceiptAggregateVoucher::aggregate_receipts(
            allocation_id,
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            &receipts,
            None,
        )
        .unwrap(),
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
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(
                    allocation_id,
                    Address::ZERO,
                    Address::ZERO,
                    Address::ZERO,
                    value,
                )
                .unwrap(),
                &wallet,
            )
            .unwrap(),
        );
    }

    let signed_rav = Eip712SignedMessage::new(
        &domain_separator,
        ReceiptAggregateVoucher::aggregate_receipts(
            allocation_id,
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            &receipts,
            None,
        )
        .unwrap(),
        &wallet,
    )
    .unwrap();

    // Update the last rav
    context.update_last_rav(signed_rav.clone()).await.unwrap();

    // Retreive rav
    let retrieved_rav = context.last_rav().await;
    assert!(retrieved_rav.unwrap().unwrap() == signed_rav);
}
