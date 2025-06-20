// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! In-memory context implementation for the TAP manager.
//!
//! This module provides an in-memory implementation of the TAP manager context.
//! It is useful for testing and development purposes.

use std::{
    collections::HashMap,
    ops::RangeBounds,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
#[cfg(feature = "v2")]
use tap_graph::v2::{ReceiptAggregateVoucher, SignedRav, SignedReceipt};
#[cfg(not(feature = "v2"))]
use tap_graph::{ReceiptAggregateVoucher, SignedRav, SignedReceipt};
use thegraph_core::alloy::primitives::Address;

use crate::{
    manager::adapters::*,
    receipt::{checks::StatefulTimestampCheck, state::Checking, ReceiptWithState},
    signed_message::MessageId,
};

pub type EscrowStorage = Arc<RwLock<HashMap<Address, u128>>>;
pub type QueryAppraisals = Arc<RwLock<HashMap<MessageId, u128>>>;
pub type ReceiptStorage = Arc<RwLock<HashMap<u64, ReceiptWithState<Checking, SignedReceipt>>>>;
pub type RAVStorage = Arc<RwLock<Option<SignedRav>>>;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InMemoryError {
    #[error("something went wrong: {error}")]
    AdapterError { error: String },
}

#[derive(Clone)]
pub struct InMemoryContext {
    /// local RAV store with rwlocks to allow sharing with other compenents as needed
    rav_storage: RAVStorage,
    receipt_storage: ReceiptStorage,
    unique_id: Arc<RwLock<u64>>,
    sender_escrow_storage: EscrowStorage,
    timestamp_check: Arc<StatefulTimestampCheck>,
    sender_address: Option<Address>,
}

impl InMemoryContext {
    pub fn new(
        rav_storage: RAVStorage,
        receipt_storage: ReceiptStorage,
        sender_escrow_storage: EscrowStorage,
        timestamp_check: Arc<StatefulTimestampCheck>,
    ) -> Self {
        InMemoryContext {
            rav_storage,
            receipt_storage,
            unique_id: Arc::new(RwLock::new(0)),
            sender_escrow_storage,
            timestamp_check,
            sender_address: None,
        }
    }

    pub fn with_sender_address(mut self, sender_address: Address) -> Self {
        self.sender_address = Some(sender_address);
        self
    }

    pub async fn retrieve_receipt_by_id(
        &self,
        receipt_id: u64,
    ) -> Result<ReceiptWithState<Checking, SignedReceipt>, InMemoryError> {
        let receipt_storage = self.receipt_storage.read().unwrap();

        receipt_storage
            .get(&receipt_id)
            .cloned()
            .ok_or(InMemoryError::AdapterError {
                error: "No receipt found with ID".to_owned(),
            })
    }

    pub async fn retrieve_receipts_by_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceiptWithState<Checking, SignedReceipt>)>, InMemoryError> {
        let receipt_storage = self.receipt_storage.read().unwrap();
        Ok(receipt_storage
            .iter()
            .filter(|(_, rx_receipt)| {
                rx_receipt.signed_receipt().message.timestamp_ns == timestamp_ns
            })
            .map(|(&id, rx_receipt)| (id, rx_receipt.clone()))
            .collect())
    }

    pub async fn retrieve_receipts_upto_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<ReceiptWithState<Checking, SignedReceipt>>, InMemoryError> {
        self.retrieve_receipts_in_timestamp_range(..=timestamp_ns, None)
            .await
    }

    pub async fn remove_receipt_by_id(&mut self, receipt_id: u64) -> Result<(), InMemoryError> {
        let mut receipt_storage = self.receipt_storage.write().unwrap();
        receipt_storage
            .remove(&receipt_id)
            .map(|_| ())
            .ok_or(InMemoryError::AdapterError {
                error: "No receipt found with ID".to_owned(),
            })
    }
    pub async fn remove_receipts_by_ids(
        &mut self,
        receipt_ids: &[u64],
    ) -> Result<(), InMemoryError> {
        for receipt_id in receipt_ids {
            self.remove_receipt_by_id(*receipt_id).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl RavStore<ReceiptAggregateVoucher> for InMemoryContext {
    type AdapterError = InMemoryError;

    async fn update_last_rav(&self, rav: SignedRav) -> Result<(), Self::AdapterError> {
        let mut rav_storage = self.rav_storage.write().unwrap();
        let timestamp = rav.message.timestampNs;
        *rav_storage = Some(rav);
        self.timestamp_check.update_min_timestamp_ns(timestamp);
        Ok(())
    }
}

#[async_trait]
impl RavRead<ReceiptAggregateVoucher> for InMemoryContext {
    type AdapterError = InMemoryError;

    async fn last_rav(&self) -> Result<Option<SignedRav>, Self::AdapterError> {
        Ok(self.rav_storage.read().unwrap().clone())
    }
}

#[async_trait]
impl ReceiptStore<SignedReceipt> for InMemoryContext {
    type AdapterError = InMemoryError;

    async fn store_receipt(
        &self,
        receipt: ReceiptWithState<Checking, SignedReceipt>,
    ) -> Result<u64, Self::AdapterError> {
        let mut id_pointer = self.unique_id.write().unwrap();
        let id_previous = *id_pointer;
        let mut receipt_storage = self.receipt_storage.write().unwrap();
        receipt_storage.insert(*id_pointer, receipt);
        *id_pointer += 1;
        Ok(id_previous)
    }
}

#[async_trait]
impl ReceiptDelete for InMemoryContext {
    type AdapterError = InMemoryError;

    async fn remove_receipts_in_timestamp_range<R: RangeBounds<u64> + std::marker::Send>(
        &self,
        timestamp_ns: R,
    ) -> Result<(), Self::AdapterError> {
        let mut receipt_storage = self.receipt_storage.write().unwrap();
        receipt_storage.retain(|_, rx_receipt| {
            !timestamp_ns.contains(&rx_receipt.signed_receipt().message.timestamp_ns)
        });
        Ok(())
    }
}
#[async_trait]
impl ReceiptRead<SignedReceipt> for InMemoryContext {
    type AdapterError = InMemoryError;
    async fn retrieve_receipts_in_timestamp_range<R: RangeBounds<u64> + std::marker::Send>(
        &self,
        timestamp_range_ns: R,
        limit: Option<u64>,
    ) -> Result<Vec<ReceiptWithState<Checking, SignedReceipt>>, Self::AdapterError> {
        let receipt_storage = self.receipt_storage.read().unwrap();
        let mut receipts_in_range: Vec<ReceiptWithState<Checking, SignedReceipt>> = receipt_storage
            .iter()
            .filter(|(_, rx_receipt)| {
                timestamp_range_ns.contains(&rx_receipt.signed_receipt().message.timestamp_ns)
            })
            .map(|(&_id, rx_receipt)| rx_receipt.clone())
            .collect();

        if limit.is_some_and(|limit| receipts_in_range.len() > limit as usize) {
            safe_truncate_receipts(&mut receipts_in_range, limit.unwrap());
        }
        Ok(receipts_in_range.into_iter().collect())
    }
}

impl InMemoryContext {
    pub fn escrow(&self, sender_id: Address) -> Result<u128, InMemoryError> {
        let sender_escrow_storage = self.sender_escrow_storage.read().unwrap();
        if let Some(escrow) = sender_escrow_storage.get(&sender_id) {
            return Ok(*escrow);
        }
        Err(InMemoryError::AdapterError {
            error: "No escrow exists for provided sender ID.".to_owned(),
        })
    }

    pub fn increase_escrow(&mut self, sender_id: Address, value: u128) {
        let mut sender_escrow_storage = self.sender_escrow_storage.write().unwrap();

        if let Some(current_value) = sender_escrow_storage.get(&sender_id) {
            let mut sender_escrow_storage = self.sender_escrow_storage.write().unwrap();
            sender_escrow_storage.insert(sender_id, current_value + value);
        } else {
            sender_escrow_storage.insert(sender_id, value);
        }
    }

    pub fn reduce_escrow(&self, sender_id: Address, value: u128) -> Result<(), InMemoryError> {
        let mut sender_escrow_storage = self.sender_escrow_storage.write().unwrap();

        if let Some(current_value) = sender_escrow_storage.get(&sender_id) {
            let checked_new_value = current_value.checked_sub(value);
            if let Some(new_value) = checked_new_value {
                sender_escrow_storage.insert(sender_id, new_value);
                return Ok(());
            }
        }
        Err(InMemoryError::AdapterError {
            error: "Provided value is greater than existing escrow.".to_owned(),
        })
    }
}

#[async_trait]
impl SignatureChecker for InMemoryContext {
    type AdapterError = InMemoryError;

    async fn verify_signer(&self, signer_address: Address) -> Result<bool, Self::AdapterError> {
        Ok(self
            .sender_address
            .map(|sender| signer_address == sender)
            .unwrap_or(false))
    }
}

pub mod checks {
    use std::{
        collections::{HashMap, HashSet},
        sync::{Arc, RwLock},
    };

    use tap_graph::v2::SignedReceipt;
    use thegraph_core::alloy::{dyn_abi::Eip712Domain, primitives::Address};

    use crate::{
        receipt::{
            checks::{Check, CheckError, CheckResult, ReceiptCheck},
            state::Checking,
            Context, ReceiptError, ReceiptWithState,
        },
        signed_message::MessageId,
    };

    pub fn get_full_list_of_checks(
        domain_separator: Eip712Domain,
        valid_signers: HashSet<Address>,
        allocation_ids: Arc<RwLock<HashSet<Address>>>,
        _query_appraisals: Arc<RwLock<HashMap<MessageId, u128>>>,
    ) -> Vec<ReceiptCheck<SignedReceipt>> {
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

    struct AllocationIdCheck {
        allocation_ids: Arc<RwLock<HashSet<Address>>>,
    }

    #[async_trait::async_trait]
    impl Check<SignedReceipt> for AllocationIdCheck {
        async fn check(
            &self,
            _: &Context,
            receipt: &ReceiptWithState<Checking, SignedReceipt>,
        ) -> CheckResult {
            let received_allocation_id = receipt.signed_receipt().message.allocation_id;
            if self
                .allocation_ids
                .read()
                .unwrap()
                .contains(&received_allocation_id)
            {
                Ok(())
            } else {
                Err(CheckError::Failed(
                    ReceiptError::InvalidAllocationID {
                        received_allocation_id,
                    }
                    .into(),
                ))
            }
        }
    }

    struct SignatureCheck {
        domain_separator: Eip712Domain,
        valid_signers: HashSet<Address>,
    }

    #[async_trait::async_trait]
    impl Check<SignedReceipt> for SignatureCheck {
        async fn check(
            &self,
            _: &Context,
            receipt: &ReceiptWithState<Checking, SignedReceipt>,
        ) -> CheckResult {
            let recovered_address = receipt
                .signed_receipt()
                .recover_signer(&self.domain_separator)
                .map_err(|e| {
                    CheckError::Failed(
                        ReceiptError::InvalidSignature {
                            source_error_message: e.to_string(),
                        }
                        .into(),
                    )
                })?;

            if !self.valid_signers.contains(&recovered_address) {
                Err(CheckError::Failed(
                    ReceiptError::InvalidSignature {
                        source_error_message: "Invalid signer".to_string(),
                    }
                    .into(),
                ))
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {}
