// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::tap_receipt::{Checking, ReceiptError, ReceiptResult, ReceiptWithState};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub type ReceiptCheck = Arc<dyn Check>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CheckingChecks {
    Pending(ReceiptCheck),
    Executed(ReceiptResult<()>),
}

impl CheckingChecks {
    pub fn new(check: ReceiptCheck) -> Self {
        Self::Pending(check)
    }

    pub async fn execute(self, receipt: &ReceiptWithState<Checking>) -> Self {
        match self {
            Self::Pending(check) => {
                let result = check.check(receipt).await;
                Self::Executed(result)
            }
            Self::Executed(_) => self,
        }
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Executed(Err(_)))
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Executed(_))
    }
}

#[async_trait::async_trait]
#[typetag::serde(tag = "type")]
pub trait Check: std::fmt::Debug + Send + Sync {
    async fn check(&self, receipt: &ReceiptWithState<Checking>) -> ReceiptResult<()>;

    async fn check_batch(&self, receipts: &[ReceiptWithState<Checking>]) -> Vec<ReceiptResult<()>> {
        let mut results = Vec::new();
        for receipt in receipts {
            let result = self.check(receipt).await;
            results.push(result);
        }
        results
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimestampCheck {
    #[serde(skip)]
    min_timestamp_ns: RwLock<u64>,
}

impl TimestampCheck {
    pub fn new(min_timestamp_ns: u64) -> Self {
        Self {
            min_timestamp_ns: RwLock::new(min_timestamp_ns),
        }
    }
    /// Updates the minimum timestamp that will be accepted for a receipt (exclusive).
    pub async fn update_min_timestamp_ns(&self, min_timestamp_ns: u64) {
        *self.min_timestamp_ns.write().await = min_timestamp_ns;
    }
}

#[async_trait::async_trait]
#[typetag::serde]
impl Check for TimestampCheck {
    async fn check(&self, receipt: &ReceiptWithState<Checking>) -> ReceiptResult<()> {
        let min_timestamp_ns = *self.min_timestamp_ns.read().await;
        let signed_receipt = receipt.signed_receipt();
        if signed_receipt.message.timestamp_ns <= min_timestamp_ns {
            return Err(ReceiptError::InvalidTimestamp {
                received_timestamp: signed_receipt.message.timestamp_ns,
                timestamp_min: min_timestamp_ns,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use crate::tap_receipt::ReceivedReceipt;
    use alloy_primitives::Address;
    use alloy_sol_types::Eip712Domain;
    use std::{
        collections::{HashMap, HashSet},
        fmt::Debug,
    };

    pub fn get_full_list_of_checks(
        domain_separator: Eip712Domain,
        valid_signers: HashSet<Address>,
        allocation_ids: Arc<RwLock<HashSet<Address>>>,
        receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
        query_appraisals: Arc<RwLock<HashMap<u64, u128>>>,
    ) -> Vec<ReceiptCheck> {
        vec![
            Arc::new(UniqueCheck { receipt_storage }),
            Arc::new(ValueCheck { query_appraisals }),
            Arc::new(AllocationIdCheck { allocation_ids }),
            Arc::new(SignatureCheck {
                domain_separator,
                valid_signers,
            }),
            Arc::new(TimestampCheck::new(0)),
        ]
    }

    #[derive(Serialize, Deserialize)]
    struct UniqueCheck {
        #[serde(skip)]
        receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
    }
    impl Debug for UniqueCheck {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "UniqueCheck")
        }
    }

    #[async_trait::async_trait]
    #[typetag::serde]
    impl Check for UniqueCheck {
        async fn check(&self, receipt: &ReceiptWithState<Checking>) -> ReceiptResult<()> {
            let receipt_storage = self.receipt_storage.read().await;
            // let receipt_id = receipt.
            receipt_storage
                .iter()
                .all(|(_stored_receipt_id, stored_receipt)| {
                    stored_receipt.signed_receipt().message != receipt.signed_receipt().message
                })
                .then_some(())
                .ok_or(ReceiptError::NonUniqueReceipt)
        }

        async fn check_batch(
            &self,
            receipts: &[ReceiptWithState<Checking>],
        ) -> Vec<ReceiptResult<()>> {
            let mut signatures: HashSet<ethers::types::Signature> = HashSet::new();
            let mut results = Vec::new();

            for received_receipt in receipts {
                let signature = received_receipt.signed_receipt.signature;
                if signatures.insert(signature) {
                    results.push(Ok(()));
                } else {
                    results.push(Err(ReceiptError::NonUniqueReceipt));
                }
            }
            results
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ValueCheck {
        #[serde(skip)]
        query_appraisals: Arc<RwLock<HashMap<u64, u128>>>,
    }

    #[async_trait::async_trait]
    #[typetag::serde]
    impl Check for ValueCheck {
        async fn check(&self, receipt: &ReceiptWithState<Checking>) -> ReceiptResult<()> {
            let query_id = receipt.query_id;
            let value = receipt.signed_receipt().message.value;
            let query_appraisals = self.query_appraisals.read().await;
            let appraised_value =
                query_appraisals
                    .get(&query_id)
                    .ok_or(ReceiptError::CheckFailedToComplete {
                        source_error_message: "Could not find query_appraisals".into(),
                    })?;

            if value != *appraised_value {
                Err(ReceiptError::InvalidValue {
                    received_value: value,
                })
            } else {
                Ok(())
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct AllocationIdCheck {
        #[serde(skip)]
        allocation_ids: Arc<RwLock<HashSet<Address>>>,
    }

    #[async_trait::async_trait]
    #[typetag::serde]
    impl Check for AllocationIdCheck {
        async fn check(&self, receipt: &ReceiptWithState<Checking>) -> ReceiptResult<()> {
            let received_allocation_id = receipt.signed_receipt().message.allocation_id;
            if self
                .allocation_ids
                .read()
                .await
                .contains(&received_allocation_id)
            {
                Ok(())
            } else {
                Err(ReceiptError::InvalidAllocationID {
                    received_allocation_id,
                })
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct SignatureCheck {
        domain_separator: Eip712Domain,
        valid_signers: HashSet<Address>,
    }

    #[async_trait::async_trait]
    #[typetag::serde]
    impl Check for SignatureCheck {
        async fn check(&self, receipt: &ReceiptWithState<Checking>) -> ReceiptResult<()> {
            let recovered_address = receipt
                .signed_receipt()
                .recover_signer(&self.domain_separator)
                .map_err(|e| ReceiptError::InvalidSignature {
                    source_error_message: e.to_string(),
                })?;
            if !self.valid_signers.contains(&recovered_address) {
                Err(ReceiptError::InvalidSignature {
                    source_error_message: "Invalid signer".to_string(),
                })
            } else {
                Ok(())
            }
        }
    }
}
