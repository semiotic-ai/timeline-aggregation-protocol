// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

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
}
type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use ethereum_types::Address;
    use std::{
        str::FromStr,
        time::{SystemTime, UNIX_EPOCH},
    };

    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rand_core::OsRng;

    use crate::{receipt::Receipt, receipt_aggregate_voucher::ReceiptAggregateVoucher, Error};

    #[test]
    fn test_receipt() {
        let signing_key = SigningKey::random(&mut OsRng);

        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();
        let value = 10u64;
        let timestamp_ns = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .unwrap();
        let test_receipt = Receipt::new(allocation_id, timestamp_ns + 9, value, &signing_key);

        assert!(test_receipt
            .is_valid(
                VerifyingKey::from(signing_key),
                &[allocation_id],
                timestamp_ns - 10,
                timestamp_ns + 10,
                None
            )
            .is_ok())
    }

    #[test]
    fn test_rav() {
        let values = (0u64..2 ^ 20).collect::<Vec<_>>();
        let mut receipts = Vec::new();
        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        for value in values {
            let timestamp_ns = u64::try_from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            )
            .unwrap();

            receipts.push(Receipt::new(
                allocation_id,
                timestamp_ns,
                value,
                &signing_key,
            ))
        }

        let rav = ReceiptAggregateVoucher::aggregate_receipt(
            &receipts,
            verifying_key,
            &signing_key,
            None,
        )
        .unwrap();
        assert!(rav.is_valid(verifying_key, allocation_id).is_ok());
    }

    #[test]
    fn test_incorrect_allocation() {
        let values = (0u64..2 ^ 20).collect::<Vec<_>>();
        let mut receipts = Vec::new();
        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();
        let false_allocation_id =
            Address::from_str("0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd").unwrap();
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        for value in values {
            receipts.push(Receipt::new(allocation_id, value, value, &signing_key));
        }

        // Add receipt with invalid allocation id
        receipts.push(Receipt::new(false_allocation_id, 0u64, 0u64, &signing_key));

        let false_rav = ReceiptAggregateVoucher::aggregate_receipt(
            &receipts,
            verifying_key,
            &signing_key,
            None,
        )
        .unwrap_err();
        assert!(matches!(false_rav, Error::InvalidAllocationID { .. }));
    }
}
