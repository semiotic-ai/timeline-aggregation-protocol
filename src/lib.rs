// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The Timeline Aggregation Protocol (TAP) is a micro-trust
//! state channel payment solution allowing one-way payments
//! from a payment sender to be aggregated then cheaply
//! verified on-chain by a payment receiver.

use ethereum_types::Address;
use thiserror::Error;

pub mod receipt;
pub mod receipt_aggregate_voucher;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid allocation ID: {received_allocation_id} (valid  {expected_allocation_ids})")]
    InvalidAllocationID {
        received_allocation_id: Address,
        expected_allocation_ids: String,
    },
    #[error("invalid signature on Receipt Aggregate Voucher")]
    InvalidSignature(#[from] k256::ecdsa::Error),
    #[error("invalid timestamp: {received_timestamp} (expected range [{timestamp_min}, {timestamp_max}) )")]
    InvalidTimestamp {
        received_timestamp: u64,
        timestamp_min: u64,
        timestamp_max: u64,
    },
    #[error("Invalid Value: {received_value} (expected {expected_value})")]
    InvalidValue {
        received_value: u64,
        expected_value: u64,
    },
    #[error("Aggregating receipt results in overflow")]
    AggregateOverflow,
}
type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use crate::{receipt::Receipt, receipt_aggregate_voucher::ReceiptAggregateVoucher, Error};
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
    #[case::basic_receipt_test(30, 20, 40, 100)]
    #[case::closest_valid_min_timestamp(30, 30, 40, 100)]
    #[case::closest_valid_max_timestamp(30, 20, 31, 100)]
    #[case::closest_valid_min_max_timestamp(30, 30, 31, 100)]
    #[case::timestamp_at_min(u64::MIN, u64::MIN, 31, 100)]
    #[case::timestamp_at_max(u64::MAX-1, u64::MAX-1, u64::MAX, 100)]
    #[case::min_value_amount(30, 20, 40, u64::MIN)]
    #[case::min_value_amount(30, 20, 40, u64::MAX)]

    fn receipt_is_valid(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] timestamp: u64,
        #[case] timestamp_min: u64,
        #[case] timestamp_max: u64,
        #[case] value: u64,
    ) {
        let test_receipt = Receipt::new(allocation_ids[0], timestamp, value, &keys.0);

        assert!(test_receipt
            .is_valid(
                keys.1,
                &[allocation_ids[0]],
                timestamp_min,
                timestamp_max,
                None
            )
            .is_ok())
    }

    #[rstest]
    #[case::basic_rav_test(vec![1,2,3,4], vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![1,2,3,4], vec![0,0,0,0])]
    #[case::rav_with_same_timestamped_receipts(vec![1,1,1,1], vec![45,56,34,23])]
    fn rav_is_valid_with_no_previous_rav(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] timestamps: Vec<u64>,
        #[case] values: Vec<u64>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for (value, timestamp) in values.iter().zip(timestamps) {
            receipts.push(Receipt::new(allocation_ids[0], timestamp, *value, &keys.0))
        }

        let rav =
            ReceiptAggregateVoucher::aggregate_receipt(&receipts, keys.1, &keys.0, None).unwrap();
        assert!(rav.is_valid(keys.1, allocation_ids[0]).is_ok());
    }

    #[rstest]
    #[case::basic_rav_test(vec![1,2,3,4], vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![1,2,3,4], vec![0,0,0,0])]
    #[case::rav_with_same_timestamped_receipts(vec![1,1,2,2], vec![45,56,34,23])]
    fn rav_is_valid_with_previous_rav(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] timestamps: Vec<u64>,
        #[case] values: Vec<u64>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for (value, timestamp) in values.iter().zip(timestamps) {
            receipts.push(Receipt::new(allocation_ids[0], timestamp, *value, &keys.0))
        }

        // Create previous RAV from first half of receipts
        let prev_rav = ReceiptAggregateVoucher::aggregate_receipt(
            &receipts[0..receipts.len() / 2],
            keys.1,
            &keys.0,
            None,
        )
        .unwrap();

        // Create new RAV from last half of receipts and prev_rav
        let rav = ReceiptAggregateVoucher::aggregate_receipt(
            &receipts[receipts.len() / 2..receipts.len()],
            keys.1,
            &keys.0,
            Some(prev_rav),
        )
        .unwrap();
        assert!(rav.is_valid(keys.1, allocation_ids[0]).is_ok());
    }

    #[rstest]
    #[case::basic_rav_test(vec![1,2,3,4], vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![1,2,3,4], vec![0,0,0,0])]
    #[case::rav_with_same_timestamped_receipts(vec![1,1,1,1], vec![45,56,34,23])]
    fn rav_with_invalid_receipt_allocation_id_submitted_errors(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] timestamps: Vec<u64>,
        #[case] values: Vec<u64>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for (value, timestamp) in values.iter().zip(timestamps) {
            receipts.push(Receipt::new(allocation_ids[0], timestamp, *value, &keys.0))
        }

        // Inject receipt with invalid allocation id
        receipts.last_mut().unwrap().allocation_id = allocation_ids[1];

        let false_rav =
            ReceiptAggregateVoucher::aggregate_receipt(&receipts, keys.1, &keys.0, None)
                .unwrap_err();
        assert!(matches!(false_rav, Error::InvalidAllocationID { .. }));
    }
}
