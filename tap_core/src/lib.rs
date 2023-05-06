// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The Timeline Aggregation Protocol (TAP) is a micro-trust
//! state channel payment solution allowing one-way payments
//! from a payment sender to be aggregated then cheaply
//! verified on-chain by a payment receiver.

use std::time::{SystemTime, UNIX_EPOCH};

use ethereum_types::Address;
use ethers::{signers::WalletError, types::SignatureError};
use receipt_aggregate_voucher::ReceiptAggregateVoucher;
use thiserror::Error;

pub mod adapters;
pub mod eip_712_signed_message;
pub mod receipt_aggregate_voucher;
pub mod tap_manager;
pub mod tap_receipt;

#[derive(Error, Debug)]

pub enum Error {
    #[error("Aggregating receipt results in overflow")]
    AggregateOverflow,
    #[error("Failed to encode to EIP712 hash:\n{source_error_message}")]
    EIP712EncodeError { source_error_message: String },
    #[error(
        "Unexpected check: {check_string}, only checks provided in initial checklist are valid"
    )]
    InvalidCheckError { check_string: String },
    #[error("The requested action is invalid for current receipt state: {state}")]
    InvalidStateForRequestedAction { state: String },
    #[error("Failed to get current system time: {source_error_message} ")]
    InvalidSystemTime { source_error_message: String },
    #[error(transparent)]
    WalletError(#[from] WalletError),
    #[error(transparent)]
    SignatureError(#[from] SignatureError),
    #[error("Recovered gateway address invalid{address}")]
    InvalidRecoveredSigner { address: Address },
    #[error("Received RAV does not match expexted RAV")]
    InvalidReceivedRAV {
        received_rav: ReceiptAggregateVoucher,
        expected_rav: ReceiptAggregateVoucher,
    },
    #[error("Error from adapter: {source_error_message}")]
    AdapterError { source_error_message: String },
    #[error("Failed to produce rav request, no valid receipts")]
    NoValidReceiptsForRAVRequest,
}
type Result<T> = std::result::Result<T, Error>;

pub(crate) fn get_current_timestamp_u64_ns() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::InvalidSystemTime {
            source_error_message: err.to_string(),
        })?
        .as_nanos() as u64)
}

#[cfg(test)]
mod tap_tests {
    use std::str::FromStr;

    use ethereum_types::Address;
    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;

    use crate::{
        eip_712_signed_message::EIP712SignedMessage,
        receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
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

    #[rstest]
    #[case::basic_rav_test (vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts (vec![0,0,0,0])]
    async fn signed_rav_is_valid_with_no_previous_rav(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        #[case] values: Vec<u128>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .await
                    .unwrap(),
            );
        }

        // Skipping receipts validation in this test, aggregate_receipts assumes receipts are valid.

        let rav = ReceiptAggregateVoucher::aggregate_receipts(allocation_ids[0], &receipts, None)
            .unwrap();
        let signed_rav = EIP712SignedMessage::new(rav, &keys.0).await.unwrap();
        assert!(signed_rav.recover_signer().unwrap() == keys.1);
    }

    #[rstest]
    #[case::basic_rav_test(vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![0,0,0,0])]
    async fn signed_rav_is_valid_with_previous_rav(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        #[case] values: Vec<u128>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .await
                    .unwrap(),
            );
        }

        // Create previous RAV from first half of receipts
        let prev_rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[0..receipts.len() / 2],
            None,
        )
        .unwrap();
        let signed_prev_rav = EIP712SignedMessage::new(prev_rav, &keys.0).await.unwrap();

        // Create new RAV from last half of receipts and prev_rav
        let rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[receipts.len() / 2..receipts.len()],
            Some(signed_prev_rav),
        )
        .unwrap();
        let signed_rav = EIP712SignedMessage::new(rav, &keys.0).await.unwrap();

        assert!(signed_rav.recover_signer().unwrap() == keys.1);
    }

    #[rstest]
    async fn verify_signature(keys: (LocalWallet, Address), allocation_ids: Vec<Address>) {
        let signed_message =
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .await
                .unwrap();

        assert!(signed_message.verify(keys.1).is_ok());
        assert!(signed_message
            .verify(Address::from_str("0x76f4eeD9fE41262669D0250b2A97db79712aD855").unwrap())
            .is_err());
    }
}
