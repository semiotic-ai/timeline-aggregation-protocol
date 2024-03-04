use crate::tap_receipt::{Checking, ReceiptError, ReceiptResult, ReceiptWithState};
use ethers::types::Signature;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
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
                let result = check.check(&receipt).await;
                // *self = Self::Executed(result);
                // self
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
}

#[async_trait::async_trait]
trait CheckBatch {
    async fn check_batch(receipt: &[ReceiptWithState<Checking>]) -> Vec<ReceiptResult<()>>;
}

// #[async_trait::async_trait]
// impl<T> CheckBatch for T
// where
//     T: Check,
// {
//     async fn check_batch(receipt: &[ReceiptWithState<Checking>]) -> ReceiptResult<()> {
//         todo!()
//     }
// }

#[derive(Debug, Serialize, Deserialize)]
struct UniqueCheck;

#[async_trait::async_trait]
#[typetag::serde]
impl Check for UniqueCheck {
    async fn check(&self, _receipt: &ReceiptWithState<Checking>) -> ReceiptResult<()> {
        println!("UniqueCheck");
        Ok(())
    }
}

#[async_trait::async_trait]
impl CheckBatch for UniqueCheck {
    async fn check_batch(receipts: &[ReceiptWithState<Checking>]) -> Vec<ReceiptResult<()>> {
        let mut signatures: HashSet<Signature> = HashSet::new();
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

#[derive(Debug, Serialize, Deserialize)]
struct AllocationId;

#[async_trait::async_trait]
#[typetag::serde]
impl Check for AllocationId {
    async fn check(&self, _receipt: &ReceiptWithState<Checking>) -> ReceiptResult<()> {
        println!("AllocationId");
        Ok(())
    }
}
