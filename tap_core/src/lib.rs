// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The Timeline Aggregation Protocol (TAP) is a micro-trust
//! state channel payment solution allowing one-way payments
//! from a payment sender to be aggregated then cheaply
//! verified on-chain by a payment receiver.

use ethereum_types::Address;
use ethers::{signers::WalletError, types::SignatureError};
use thiserror::Error;

pub mod adapters;
pub mod eip_712_signed_message;
pub mod receipt_aggregate_voucher;
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
}
type Result<T> = std::result::Result<T, Error>;

// use k256::{ecdsa::VerifyingKey, PublicKey as K256PublicKey};
use k256::{ecdsa::VerifyingKey, elliptic_curve::sec1::ToEncodedPoint, PublicKey as K256PublicKey};
use tiny_keccak::{Hasher, Keccak};

// TODO: https://github.com/semiotic-ai/timeline_aggregation_protocol/issues/37
//       Remove this function when issue is resolved (library should use ether-rs directly)

/// Translates from K256 ECDSA VerifyingKey to Ether Address
pub fn verifying_key_to_address(verifying_key: &VerifyingKey) -> Address {
    let public_key = K256PublicKey::from(verifying_key).to_encoded_point(false);
    let public_key_bytes = public_key.as_bytes();

    // Take the Keccak-256 hash of the serialized public key
    let mut keccak = Keccak::v256();
    let mut hash_output = [0u8; 32];
    keccak.update(&public_key_bytes[1..]);
    keccak.finalize(&mut hash_output);

    Address::from_slice(&hash_output[12..])
}

#[cfg(test)]
mod tap_tests {
    use crate::{
        eip_712_signed_message::EIP712SignedMessage,
        receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
        verifying_key_to_address,
    };
    use ethereum_types::Address;
    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rstest::*;
    use std::str::FromStr;

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
    fn verifying_key_to_address_test() {
        // Randomly generated key with expected address
        let signing_key_bytes = [
            131u8, 5, 83, 10, 48, 91, 169, 43, 233, 200, 145, 129, 226, 44, 204, 71, 173, 186, 163,
            54, 158, 165, 161, 61, 170, 144, 138, 40, 166, 213, 139, 142,
        ];
        let expected_address = [
            82u8, 114, 93, 165, 3, 152, 20, 223, 240, 150, 135, 235, 90, 222, 107, 21, 180, 227,
            60, 12,
        ];
        let signing_key = SigningKey::from_bytes(&signing_key_bytes.into()).unwrap();
        let verifying_key = VerifyingKey::from(&signing_key);
        let address = verifying_key_to_address(&verifying_key);
        assert_eq!(expected_address.as_slice(), address.as_bytes())
    }
}
