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

/// CheckBatch is mostly used by the lib to implement checks that transition from one state to another.
pub trait CheckBatch {
    fn check_batch(
        &self,
        receipts: Vec<ReceiptWithState<Checking>>,
    ) -> (
        Vec<ReceiptWithState<Checking>>,
        Vec<ReceiptWithState<Failed>>,
    );
}

/// Provides a built-in check to verify that the timestamp of a receipt is greater than a given value.
///
/// This check is stateful, meaning that it can be updated with a new minimum timestamp.
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

/// Timestamp Check verifies if the receipt is **greater or equal** than the minimum timestamp provided.
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

/// UniqueCheck is a batch check that verifies if any given list of receipts has unique signatures.
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
        let mut signatures: HashSet<ethers::types::Signature> = HashSet::new();
        let (mut checking, mut failed) = (vec![], vec![]);

        for received_receipt in receipts.into_iter() {
            let signature = received_receipt.signed_receipt.signature;
            if signatures.insert(signature) {
                checking.push(received_receipt);
            } else {
                failed.push(received_receipt.perform_state_error(ReceiptError::NonUniqueReceipt));
            }
        }
        (checking, failed)
    }
}
