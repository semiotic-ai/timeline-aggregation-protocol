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
//! use tap_receipt::{
//!     checks::{Check, CheckResult, ReceiptCheck},
//!     Context, ReceiptWithState, state::Checking
//! };
//! # use async_trait::async_trait;
//! # struct SignedReceipt;
//!
//! struct MyCheck;
//!
//! #[async_trait]
//! impl<T> Check<T> for MyCheck {
//!    async fn check(&self, ctx: &Context, receipt: &ReceiptWithState<Checking, T>) -> CheckResult {
//!       // Implement your check here
//!      Ok(())
//!   }
//! }
//!
//! let my_check: ReceiptCheck<SignedReceipt> = Arc::new(MyCheck);
//! ```

use std::{
    collections::HashSet,
    ops::Deref,
    sync::{Arc, RwLock},
};

use tap_eip712_message::Eip712Error;

use super::{
    state::{Checking, Failed},
    Context, ReceiptError, ReceiptWithState, WithUniqueId, WithValueAndTimestamp,
};

/// ReceiptCheck is a type alias for an Arc of a struct that implements the `Check` trait.
pub type ReceiptCheck<Rcpt> = Arc<dyn Check<Rcpt> + Sync + Send>;

/// Result of a check operation. It uses the `anyhow` crate to handle errors.
pub type CheckResult = Result<(), CheckError>;

#[derive(thiserror::Error, Debug)]
pub enum CheckError {
    #[error(transparent)]
    Retryable(anyhow::Error),
    #[error(transparent)]
    Failed(anyhow::Error),
}

/// CheckList is a NewType pattern to store a list of checks.
/// It is a wrapper around an Arc of ReceiptCheck[].
pub struct CheckList<Rcpt>(Arc<[ReceiptCheck<Rcpt>]>);

impl<Rcpt> CheckList<Rcpt> {
    pub fn new(checks: Vec<ReceiptCheck<Rcpt>>) -> Self {
        Self(checks.into())
    }

    pub fn empty() -> Self {
        Self(Arc::new([]))
    }
}

impl<Rcpt> Deref for CheckList<Rcpt> {
    type Target = [ReceiptCheck<Rcpt>];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

/// Check trait is implemented by the lib user to validate receipts before they are stored.
#[async_trait::async_trait]
pub trait Check<Rcpt> {
    async fn check(&self, ctx: &Context, receipt: &ReceiptWithState<Checking, Rcpt>)
        -> CheckResult;
}

type CheckBatchResponse<Rcpt> = (
    Vec<ReceiptWithState<Checking, Rcpt>>,
    Vec<ReceiptWithState<Failed, Rcpt>>,
);

/// CheckBatch is mostly used by the lib to implement checks
/// that transition from one state to another.
pub trait CheckBatch<Rcpt> {
    fn check_batch(
        &self,
        receipts: Vec<ReceiptWithState<Checking, Rcpt>>,
    ) -> Result<CheckBatchResponse<Rcpt>, Eip712Error>;
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
impl<Rcpt> Check<Rcpt> for StatefulTimestampCheck
where
    Rcpt: WithValueAndTimestamp + Sync,
{
    async fn check(&self, _: &Context, receipt: &ReceiptWithState<Checking, Rcpt>) -> CheckResult {
        let min_timestamp_ns = *self.min_timestamp_ns.read().unwrap();
        let signed_receipt = receipt.signed_receipt();
        if signed_receipt.timestamp_ns() <= min_timestamp_ns {
            return Err(CheckError::Failed(
                ReceiptError::InvalidTimestamp {
                    received_timestamp: signed_receipt.timestamp_ns(),
                    timestamp_min: min_timestamp_ns,
                }
                .into(),
            ));
        }
        Ok(())
    }
}

/// Timestamp Check verifies if the receipt is **greater or equal** than the
/// minimum timestamp provided.
///
/// Used by the [`crate::manager::Manager`].
pub struct TimestampCheck(pub u64);

impl<Rcpt> CheckBatch<Rcpt> for TimestampCheck
where
    Rcpt: WithValueAndTimestamp,
{
    fn check_batch(
        &self,
        receipts: Vec<ReceiptWithState<Checking, Rcpt>>,
    ) -> Result<CheckBatchResponse<Rcpt>, Eip712Error> {
        let (mut checking, mut failed) = (vec![], vec![]);
        for receipt in receipts.into_iter() {
            let receipt_timestamp_ns = receipt.signed_receipt().timestamp_ns();
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
        Ok((checking, failed))
    }
}

/// UniqueCheck is a batch check that verifies if any given list of receipts
/// has unique signatures.
///
/// Used by the [`crate::manager::Manager`].
pub struct UniqueCheck;

impl<Rcpt> CheckBatch<Rcpt> for UniqueCheck
where
    Rcpt: WithUniqueId,
{
    fn check_batch(
        &self,
        receipts: Vec<ReceiptWithState<Checking, Rcpt>>,
    ) -> Result<CheckBatchResponse<Rcpt>, Eip712Error> {
        let mut signatures = HashSet::new();
        let (mut checking, mut failed) = (vec![], vec![]);

        for received_receipt in receipts.into_iter() {
            let unique_id = received_receipt.receipt.unique_id();
            if signatures.insert(unique_id) {
                checking.push(received_receipt);
            } else {
                failed.push(received_receipt.perform_state_error(ReceiptError::NonUniqueReceipt));
            }
        }
        Ok((checking, failed))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use tap_eip712_message::Eip712SignedMessage;
    use thegraph_core::alloy::{
        dyn_abi::Eip712Domain, primitives::Address, signers::local::PrivateKeySigner, sol,
        sol_types::eip712_domain,
    };

    use super::*;

    sol! {
        struct MyReceipt {
            uint64 timestamp_ns;
            uint128 value;
        }
    }

    impl WithValueAndTimestamp for MyReceipt {
        fn value(&self) -> u128 {
            self.value
        }

        fn timestamp_ns(&self) -> u64 {
            self.timestamp_ns
        }
    }

    fn create_test_domain_separator() -> Eip712Domain {
        eip712_domain! {
            name: "TAP",
            version: "1",
            chain_id: 1,
            verifying_contract: Address:: from([0x11u8; 20]),
        }
    }

    fn create_signed_receipt_with_custom_value(
        value: u128,
    ) -> ReceiptWithState<Checking, Eip712SignedMessage<MyReceipt>> {
        let wallet: PrivateKeySigner = PrivateKeySigner::random();
        let eip712_domain_separator = create_test_domain_separator();

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos()
            + Duration::from_secs(33).as_nanos();
        let timestamp_ns = timestamp as u64;

        let value: u128 = value;
        let receipt = Eip712SignedMessage::new(
            &eip712_domain_separator,
            MyReceipt {
                timestamp_ns,
                value,
            },
            &wallet,
        )
        .unwrap();
        ReceiptWithState::new(receipt)
    }

    #[tokio::test]
    async fn test_receipt_uniqueness_check() {
        let signed_receipt = create_signed_receipt_with_custom_value(10);
        let signed_receipt_2 = create_signed_receipt_with_custom_value(15);
        let signed_receipt_copy = signed_receipt.clone();
        let receipts_batch = vec![signed_receipt, signed_receipt_2, signed_receipt_copy];
        let (valid_receipts, invalid_receipts) = UniqueCheck.check_batch(receipts_batch).unwrap();
        assert_eq!(valid_receipts.len(), 2);
        assert_eq!(invalid_receipts.len(), 1);
    }

    #[tokio::test]
    async fn test_receipt_timestamp_check() {
        let signed_receipt = create_signed_receipt_with_custom_value(10);
        let signed_receipt_2 = create_signed_receipt_with_custom_value(15);

        let receipts_batch = vec![signed_receipt.clone(), signed_receipt_2];
        let min_time_stamp = signed_receipt.receipt.message.timestamp_ns + 1;
        let (valid_receipts, invalid_receipts) = TimestampCheck(min_time_stamp)
            .check_batch(receipts_batch)
            .unwrap();
        assert_eq!(valid_receipts.len(), 1);
        assert_eq!(invalid_receipts.len(), 1);
    }
}
