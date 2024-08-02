// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # Receipt Checks
//!
//! Checks are implemented by the lib user to validate receipts before they are stored.
//! To create a check, you need to implement the `Check` trait into a struct.
//!
//! ## Example
//!
//! ```rust
//! # use std::sync::Arc;
//! use tap_core::{
//!     receipt::checks::{Check, CheckResult, ReceiptCheck},
//!     receipt::{ReceiptWithState, state::Checking}
//! };
//! # use async_trait::async_trait;
//!
//! struct MyCheck;
//!
//! #[async_trait]
//! impl Check for MyCheck {
//!    async fn check(&self, receipt: &ReceiptWithState<Checking>) -> CheckResult {
//!       // Implement your check here
//!      Ok(())
//!   }
//! }
//!
//! let my_check: ReceiptCheck = Arc::new(MyCheck);
//! ```

use super::{
    state::{Checking, Failed},
    ReceiptError, ReceiptWithState,
};
use std::{
    collections::HashSet,
    ops::Deref,
    sync::{Arc, RwLock},
};

/// ReceiptCheck is a type alias for an Arc of a struct that implements the `Check` trait.
pub type ReceiptCheck = Arc<dyn Check + Sync + Send>;

/// Result of a check operation. It uses the `anyhow` crate to handle errors.
pub type CheckResult = anyhow::Result<()>;

/// CheckList is a NewType pattern to store a list of checks.
/// It is a wrapper around an Arc of ReceiptCheck[].
pub struct CheckList(Arc<[ReceiptCheck]>);

impl CheckList {
    pub fn new(checks: Vec<ReceiptCheck>) -> Self {
        Self(checks.into())
    }

    pub fn empty() -> Self {
        Self(Arc::new([]))
    }
}

impl Deref for CheckList {
    type Target = [ReceiptCheck];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

/// Check trait is implemented by the lib user to validate receipts before they are stored.
#[async_trait::async_trait]
pub trait Check {
    async fn check(&self, receipt: &ReceiptWithState<Checking>) -> CheckResult;
}

/// CheckBatch is mostly used by the lib to implement checks
/// that transition from one state to another.
pub trait CheckBatch {
    fn check_batch(
        &self,
        receipts: Vec<ReceiptWithState<Checking>>,
    ) -> (
        Vec<ReceiptWithState<Checking>>,
        Vec<ReceiptWithState<Failed>>,
    );
}

/// Provides a built-in check to verify that the timestamp of a receipt
/// is greater than a given value.
///
/// This check is stateful, meaning that it can be updated with a new minimum
/// timestamp.
#[derive(Debug)]
pub struct StatefulTimestampCheck {
    min_timestamp_ns: RwLock<u64>,
}

impl StatefulTimestampCheck {
    pub fn new(min_timestamp_ns: u64) -> Self {
        Self {
            min_timestamp_ns: RwLock::new(min_timestamp_ns),
        }
    }
    /// Updates the minimum timestamp that will be accepted for a receipt (exclusive).
    pub fn update_min_timestamp_ns(&self, min_timestamp_ns: u64) {
        *self.min_timestamp_ns.write().unwrap() = min_timestamp_ns;
    }
}

#[async_trait::async_trait]
impl Check for StatefulTimestampCheck {
    async fn check(&self, receipt: &ReceiptWithState<Checking>) -> CheckResult {
        let min_timestamp_ns = *self.min_timestamp_ns.read().unwrap();
        let signed_receipt = receipt.signed_receipt();
        if signed_receipt.message.timestamp_ns <= min_timestamp_ns {
            return Err(ReceiptError::InvalidTimestamp {
                received_timestamp: signed_receipt.message.timestamp_ns,
                timestamp_min: min_timestamp_ns,
            }
            .into());
        }
        Ok(())
    }
}

/// Timestamp Check verifies if the receipt is **greater or equal** than the
/// minimum timestamp provided.
///
/// Used by the [`crate::manager::Manager`].
pub struct TimestampCheck(pub u64);

impl CheckBatch for TimestampCheck {
    fn check_batch(
        &self,
        receipts: Vec<ReceiptWithState<Checking>>,
    ) -> (
        Vec<ReceiptWithState<Checking>>,
        Vec<ReceiptWithState<Failed>>,
    ) {
        let (mut checking, mut failed) = (vec![], vec![]);
        for receipt in receipts.into_iter() {
            let receipt_timestamp_ns = receipt.signed_receipt().message.timestamp_ns;
            let min_timestamp_ns = self.0;
            if receipt_timestamp_ns >= min_timestamp_ns {
                checking.push(receipt);
            } else {
                failed.push(receipt.perform_state_error(ReceiptError::InvalidTimestamp {
                    received_timestamp: receipt_timestamp_ns,
                    timestamp_min: min_timestamp_ns,
                }));
            }
        }
        (checking, failed)
    }
}

/// UniqueCheck is a batch check that verifies if any given list of receipts
/// has unique signatures.
///
/// Used by the [`crate::manager::Manager`].
pub struct UniqueCheck;

impl CheckBatch for UniqueCheck {
    fn check_batch(
        &self,
        receipts: Vec<ReceiptWithState<Checking>>,
    ) -> (
        Vec<ReceiptWithState<Checking>>,
        Vec<ReceiptWithState<Failed>>,
    ) {
        let mut signatures: HashSet<[u8; 65]> = HashSet::new();
        let (mut checking, mut failed) = (vec![], vec![]);

        for received_receipt in receipts.into_iter() {
            let signature = received_receipt.signed_receipt.signature.as_bytes();
            if signatures.insert(signature) {
                checking.push(received_receipt);
            } else {
                failed.push(received_receipt.perform_state_error(ReceiptError::NonUniqueReceipt));
            }
        }
        (checking, failed)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::time::Duration;
    use std::time::SystemTime;

    use alloy::dyn_abi::Eip712Domain;
    use alloy::primitives::Address;
    use alloy::signers::local::PrivateKeySigner;
    use alloy::sol_types::eip712_domain;

    use crate::receipt::Receipt;
    use crate::signed_message::EIP712SignedMessage;

    use super::*;

    fn create_signed_receipt_with_custom_value(value: u128) -> ReceiptWithState<Checking> {
        let wallet: PrivateKeySigner = PrivateKeySigner::random();
        let eip712_domain_separator: Eip712Domain = eip712_domain! {
            name: "TAP",
            version: "1",
            chain_id: 1,
            verifying_contract: Address:: from([0x11u8; 20]),
        };

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos()
            + Duration::from_secs(33).as_nanos();
        let timestamp_ns = timestamp as u64;

        let value: u128 = value;
        let nonce: u64 = 10;
        let receipt = EIP712SignedMessage::new(
            &eip712_domain_separator,
            Receipt {
                allocation_id: Address::from_str("0xabababababababababababababababababababab")
                    .unwrap(),
                nonce,
                timestamp_ns,
                value,
            },
            &wallet,
        )
        .unwrap();
        ReceiptWithState::<Checking>::new(receipt)
    }

    #[tokio::test]
    async fn test_receipt_uniqueness_check() {
        let signed_receipt = create_signed_receipt_with_custom_value(10);
        let signed_receipt_2 = create_signed_receipt_with_custom_value(15);
        let signed_receipt_copy = signed_receipt.clone();
        let receipts_batch = vec![signed_receipt, signed_receipt_2, signed_receipt_copy];
        let (valid_receipts, invalid_receipts) = UniqueCheck.check_batch(receipts_batch);
        assert_eq!(valid_receipts.len(), 2);
        assert_eq!(invalid_receipts.len(), 1);
    }

    #[tokio::test]
    async fn test_receipt_timestamp_check() {
        let signed_receipt = create_signed_receipt_with_custom_value(10);
        let signed_receipt_2 = create_signed_receipt_with_custom_value(15);
        let receipts_batch = vec![signed_receipt.clone(), signed_receipt_2];
        let min_time_stamp = signed_receipt.signed_receipt.message.timestamp_ns + 1;
        let (valid_receipts, invalid_receipts) =
            TimestampCheck(min_time_stamp).check_batch(receipts_batch);
        assert_eq!(valid_receipts.len(), 1);
        assert_eq!(invalid_receipts.len(), 1);
    }
}
