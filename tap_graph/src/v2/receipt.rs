// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Receipt v2

use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use rand::{rng, Rng};
use serde::{Deserialize, Serialize};
use tap_eip712_message::Eip712SignedMessage;
use tap_receipt::WithValueAndTimestamp;
use thegraph_core::alloy::{primitives::Address, sol};

/// A signed receipt message
pub type SignedReceipt = Eip712SignedMessage<Receipt>;

sol! {
    /// Holds information needed for promise of payment signed with ECDSA
    #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
    struct Receipt {
        /// Unique allocation id this receipt belongs to
        address allocation_id;

        // The address of the payer the RAV was issued by
        address payer;
        // The address of the data service the RAV was issued to
        address data_service;
        // The address of the service provider the RAV was issued to
        address service_provider;

        /// Unix Epoch timestamp in nanoseconds (Truncated to 64-bits)
        uint64 timestamp_ns;
        /// Random value used to avoid collisions from multiple receipts with one timestamp
        uint64 nonce;
        /// GRT value for transaction (truncate to lower bits)
        uint128 value;
    }
}

fn get_current_timestamp_u64_ns() -> Result<u64, SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64)
}
impl Receipt {
    /// Returns a receipt with provided values
    pub fn new(
        allocation_id: Address,
        payer: Address,
        data_service: Address,
        service_provider: Address,
        value: u128,
    ) -> Result<Self, SystemTimeError> {
        let timestamp_ns = get_current_timestamp_u64_ns()?;
        let nonce = rng().random::<u64>();
        Ok(Self {
            allocation_id,
            payer,
            data_service,
            service_provider,
            timestamp_ns,
            nonce,
            value,
        })
    }
}

impl WithValueAndTimestamp for Receipt {
    fn value(&self) -> u128 {
        self.value
    }

    fn timestamp_ns(&self) -> u64 {
        self.timestamp_ns
    }
}

#[cfg(test)]
mod receipt_unit_test {
    use std::time::{SystemTime, UNIX_EPOCH};

    use rstest::*;
    use thegraph_core::alloy::primitives::address;

    use super::*;

    #[fixture]
    fn allocation_id() -> Address {
        address!("1234567890abcdef1234567890abcdef12345678")
    }

    #[fixture]
    fn payer() -> Address {
        address!("abababababababababababababababababababab")
    }

    #[fixture]
    fn data_service() -> Address {
        address!("deaddeaddeaddeaddeaddeaddeaddeaddeaddead")
    }

    #[fixture]
    fn service_provider() -> Address {
        address!("beefbeefbeefbeefbeefbeefbeefbeefbeefbeef")
    }

    #[fixture]
    fn value() -> u128 {
        1234
    }

    #[fixture]
    fn receipt(
        allocation_id: Address,
        payer: Address,
        data_service: Address,
        service_provider: Address,
        value: u128,
    ) -> Receipt {
        Receipt::new(allocation_id, payer, data_service, service_provider, value).unwrap()
    }

    #[rstest]
    fn test_new_receipt(allocation_id: Address, value: u128, receipt: Receipt) {
        assert_eq!(receipt.allocation_id, allocation_id);
        assert_eq!(receipt.value, value);

        // Check that the timestamp is within a reasonable range
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Current system time should be greater than `UNIX_EPOCH`")
            .as_nanos() as u64;
        assert!(receipt.timestamp_ns <= now);
        assert!(receipt.timestamp_ns >= now - 5000000); // 5 second tolerance
    }

    #[rstest]
    fn test_unique_nonce_and_timestamp(
        #[from(receipt)] receipt1: Receipt,
        #[from(receipt)] receipt2: Receipt,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Current system time should be greater than `UNIX_EPOCH`")
            .as_nanos() as u64;

        // Check that nonces are different
        // Note: This test has an *extremely low* (~1/2^64) probability of false failure, if a failure happens
        //       once it is not neccessarily a sign of an issue. If this test fails more than once, especially
        //       in a short period of time (within a ) then there may be an issue with randomness
        //       of the nonce generation.
        assert_ne!(receipt1.nonce, receipt2.nonce);

        assert!(receipt1.timestamp_ns <= now);
        assert!(receipt1.timestamp_ns >= now - 5000000); // 5 second tolerance

        assert!(receipt2.timestamp_ns <= now);
        assert!(receipt2.timestamp_ns >= now - 5000000); // 5 second tolerance
    }
}
