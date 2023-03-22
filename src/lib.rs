// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The Timeline Aggregation Protocol (TAP) is a micro-trust
//! state channel payment solution allowing one-way payments
//! from a payment sender to be aggregated then cheaply
//! verified on-chain by a payment receiver.

use ethereum_types::Address;
use thiserror::Error;

pub mod eip_712_signed_message;
pub mod receipt;
pub mod receipt_aggregate_voucher;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid allocation ID: {received_allocation_id} (valid {expected_allocation_ids})")]
    InvalidAllocationID {
        received_allocation_id: Address,
        expected_allocation_ids: String,
    },
    #[error(transparent)]
    InvalidSignature(#[from] k256::ecdsa::Error),
    #[error("invalid timestamp: {received_timestamp} (expected range [{timestamp_min}, {timestamp_max}) )")]
    InvalidTimestamp {
        received_timestamp: u64,
        timestamp_min: u64,
        timestamp_max: u64,
    },
    #[error("Invalid Value: {received_value} (expected {expected_value})")]
    InvalidValue {
        received_value: u128,
        expected_value: u128,
    },
    #[error("Aggregating receipt results in overflow")]
    AggregateOverflow,
    #[error("Failed to encode to EIP712 hash:\n{source_error_message}")]
    EIP712EncodeError { source_error_message: String },
}
type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tap_tests {
    use crate::{
        eip_712_signed_message::EIP712SignedMessage, receipt::Receipt,
        receipt_aggregate_voucher::ReceiptAggregateVoucher,
    };
    use ethereum_types::Address;
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rand_core::OsRng;
    use rstest::*;
    use std::str::FromStr;

    #[fixture]
    fn keys() -> (SigningKey, VerifyingKey) {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        (signing_key, verifying_key)
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
    #[case::basic_receipt_test(30, 100)]
    #[case::closest_valid_min_timestamp(30, 100)]
    #[case::closest_valid_max_timestamp(30, 100)]
    #[case::closest_valid_min_max_timestamp(30, 100)]
    fn signed_receipt_is_valid(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] timestamp: u64,
        #[case] value: u128,
    ) {
        let test_receipt = Receipt::new(allocation_ids[0], timestamp, value);
        let signed_message = EIP712SignedMessage::new(test_receipt, &keys.0).unwrap();
        assert!(signed_message.check_signature(keys.1).is_ok())
    }

    #[rstest]
    #[case::basic_rav_test(vec![1,2,3,4], vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![1,2,3,4], vec![0,0,0,0])]
    #[case::rav_with_same_timestamped_receipts(vec![1,1,1,1], vec![45,56,34,23])]
    fn signed_rav_is_valid_with_no_previous_rav(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] timestamps: Vec<u64>,
        #[case] values: Vec<u128>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for (value, timestamp) in values.iter().zip(timestamps) {
            receipts.push(
                EIP712SignedMessage::new(
                    crate::receipt::Receipt::new(allocation_ids[0], timestamp, *value),
                    &keys.0,
                )
                .unwrap(),
            );
        }

        // Skipping receipts validation in this test, aggregate_receipts assumes receipts are valid.

        let rav = ReceiptAggregateVoucher::aggregate_receipts(allocation_ids[0], &receipts, None)
            .unwrap();
        let signed_rav = EIP712SignedMessage::new(rav, &keys.0).unwrap();
        assert!(signed_rav.check_signature(keys.1).is_ok());
    }

    #[rstest]
    #[case::basic_rav_test(vec![1,2,3,4], vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![1,2,3,4], vec![0,0,0,0])]
    #[case::rav_with_same_timestamped_receipts(vec![1,1,2,2], vec![45,56,34,23])]
    fn signed_rav_is_valid_with_previous_rav(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] timestamps: Vec<u64>,
        #[case] values: Vec<u128>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for (value, timestamp) in values.iter().zip(timestamps) {
            receipts.push(
                EIP712SignedMessage::new(
                    crate::receipt::Receipt::new(allocation_ids[0], timestamp, *value),
                    &keys.0,
                )
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
        let signed_prev_rav = EIP712SignedMessage::new(prev_rav, &keys.0).unwrap();

        // Create new RAV from last half of receipts and prev_rav
        let rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[receipts.len() / 2..receipts.len()],
            Some(signed_prev_rav),
        )
        .unwrap();
        let signed_rav = EIP712SignedMessage::new(rav, &keys.0).unwrap();

        assert!(signed_rav.check_signature(keys.1).is_ok());
    }
}
