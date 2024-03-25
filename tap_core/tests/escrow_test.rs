// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
use rstest::*;

use tap_core::{
    manager::{adapters::EscrowHandler, context::memory::InMemoryContext},
    receipt::checks::StatefulTimestampCheck,
};

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
async fn escrow_handler_test(mut context: InMemoryContext) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
    let sender_id: [u8; 20] = wallet.address().into();
    let sender_id = sender_id.into();

    let invalid_wallet: LocalWallet = MnemonicBuilder::<English>::default()
        .phrase("wrong century settle satisfy market forest title connect ten push alley depend")
        .build()
        .unwrap();
    let invalid_sender_id: [u8; 20] = invalid_wallet.address().into();
    let invalid_sender_id = invalid_sender_id.into();

    let initial_value = 500u128;

    context.increase_escrow(sender_id, initial_value);

    // Check that sender exists and has valid value through adapter
    assert!(context.get_available_escrow(sender_id).await.is_ok());
    assert_eq!(
        context.get_available_escrow(sender_id).await.unwrap(),
        initial_value
    );

    // Check that subtracting is valid for valid sender, and results in expected value
    assert!(context
        .subtract_escrow(sender_id, initial_value)
        .await
        .is_ok());
    assert!(context.get_available_escrow(sender_id).await.is_ok());
    assert_eq!(context.get_available_escrow(sender_id).await.unwrap(), 0);

    // Check that subtracting to negative escrow results in err
    assert!(context
        .subtract_escrow(sender_id, initial_value)
        .await
        .is_err());

    // Check that accessing non initialized sender results in err
    assert!(context
        .get_available_escrow(invalid_sender_id)
        .await
        .is_err());
}
