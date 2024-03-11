// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::receipt::{Checking, ReceiptError, ReceiptWithState};
use std::{
    collections::HashSet,
    ops::Deref,
    sync::{Arc, RwLock},
};

use super::Failed;

pub type ReceiptCheck = Arc<dyn Check + Sync + Send>;

pub type CheckResult = anyhow::Result<()>;

pub struct Checks(Arc<[ReceiptCheck]>);

impl Checks {
    pub fn new(checks: Vec<ReceiptCheck>) -> Self {
        Self(checks.into())
    }

    pub fn empty() -> Self {
        Self(Arc::new([]))
    }
}

impl Deref for Checks {
    type Target = [ReceiptCheck];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

#[async_trait::async_trait]
pub trait Check {
    async fn check(&self, receipt: &ReceiptWithState<Checking>) -> CheckResult;
}

pub trait CheckBatch {
    fn check_batch(
        &self,
        receipts: Vec<ReceiptWithState<Checking>>,
    ) -> (
        Vec<ReceiptWithState<Checking>>,
        Vec<ReceiptWithState<Failed>>,
    );
}

#[derive(Debug)]
pub struct TimestampCheck {
    min_timestamp_ns: RwLock<u64>,
}

impl TimestampCheck {
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
impl Check for TimestampCheck {
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
pub struct BatchTimestampCheck(pub u64);

impl CheckBatch for BatchTimestampCheck {
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
