// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::tap_receipt::{Checking, ReceiptError, ReceiptWithState};
use std::sync::{Arc, RwLock};

pub type ReceiptCheck = Arc<dyn Check + Sync + Send>;

pub type CheckResult = anyhow::Result<()>;

#[async_trait::async_trait]
pub trait Check {
    async fn check(&self, receipt: &ReceiptWithState<Checking>) -> CheckResult;
}

#[async_trait::async_trait]
pub trait CheckBatch {
    async fn check_batch(&self, receipts: &[ReceiptWithState<Checking>]) -> Vec<CheckResult>;
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
        if signed_receipt.message.timestamp_ns < min_timestamp_ns {
            return Err(ReceiptError::InvalidTimestamp {
                received_timestamp: signed_receipt.message.timestamp_ns,
                timestamp_min: min_timestamp_ns,
            }
            .into());
        }
        Ok(())
    }
}

#[cfg(feature = "mock")]
pub mod mock {

    use super::*;
    use crate::eip_712_signed_message::MessageId;
    use alloy_primitives::Address;
    use alloy_sol_types::Eip712Domain;
    use std::collections::{HashMap, HashSet};

    pub fn get_full_list_of_checks(
        domain_separator: Eip712Domain,
        valid_signers: HashSet<Address>,
        allocation_ids: Arc<RwLock<HashSet<Address>>>,
        _query_appraisals: Arc<RwLock<HashMap<MessageId, u128>>>,
    ) -> Vec<ReceiptCheck> {
        vec![
            // Arc::new(UniqueCheck ),
            // Arc::new(ValueCheck { query_appraisals }),
            Arc::new(AllocationIdCheck { allocation_ids }),
            Arc::new(SignatureCheck {
                domain_separator,
                valid_signers,
            }),
        ]
    }

    struct UniqueCheck;

    #[async_trait::async_trait]
    impl CheckBatch for UniqueCheck {
        async fn check_batch(&self, receipts: &[ReceiptWithState<Checking>]) -> Vec<CheckResult> {
            let mut signatures: HashSet<ethers::types::Signature> = HashSet::new();
            let mut results = Vec::new();

            for received_receipt in receipts {
                let signature = received_receipt.signed_receipt.signature;
                if signatures.insert(signature) {
                    results.push(Ok(()));
                } else {
                    results.push(Err(ReceiptError::NonUniqueReceipt.into()));
                }
            }
            results
        }
    }

    struct ValueCheck {
        query_appraisals: Arc<RwLock<HashMap<MessageId, u128>>>,
    }

    #[async_trait::async_trait]
    impl Check for ValueCheck {
        async fn check(&self, receipt: &ReceiptWithState<Checking>) -> CheckResult {
            let value = receipt.signed_receipt().message.value;
            let query_appraisals = self.query_appraisals.read().unwrap();
            let hash = receipt.signed_receipt().unique_hash();
            let appraised_value =
                query_appraisals
                    .get(&hash)
                    .ok_or(ReceiptError::CheckFailedToComplete(
                        "Could not find query_appraisals".into(),
                    ))?;

            if value != *appraised_value {
                Err(ReceiptError::InvalidValue {
                    received_value: value,
                }
                .into())
            } else {
                Ok(())
            }
        }
    }

    struct AllocationIdCheck {
        allocation_ids: Arc<RwLock<HashSet<Address>>>,
    }

    #[async_trait::async_trait]
    impl Check for AllocationIdCheck {
        async fn check(&self, receipt: &ReceiptWithState<Checking>) -> CheckResult {
            let received_allocation_id = receipt.signed_receipt().message.allocation_id;
            if self
                .allocation_ids
                .read()
                .unwrap()
                .contains(&received_allocation_id)
            {
                Ok(())
            } else {
                Err(ReceiptError::InvalidAllocationID {
                    received_allocation_id,
                }
                .into())
            }
        }
    }

    struct SignatureCheck {
        domain_separator: Eip712Domain,
        valid_signers: HashSet<Address>,
    }

    #[async_trait::async_trait]
    impl Check for SignatureCheck {
        async fn check(&self, receipt: &ReceiptWithState<Checking>) -> CheckResult {
            let recovered_address = receipt
                .signed_receipt()
                .recover_signer(&self.domain_separator)
                .map_err(|e| ReceiptError::InvalidSignature {
                    source_error_message: e.to_string(),
                })?;
            println!("{:?}, {:?}", self.valid_signers, recovered_address);
            if !self.valid_signers.contains(&recovered_address) {
                Err(ReceiptError::InvalidSignature {
                    source_error_message: "Invalid signer".to_string(),
                }
                .into())
            } else {
                Ok(())
            }
        }
    }
}
