// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{escrow_adapter_mock::AdpaterErrorMock, receipt_checks_adapter_mock::AdapterErrorMock};
use crate::adapters::escrow_adapter::EscrowAdapter;
use crate::adapters::receipt_checks_adapter::ReceiptChecksAdapter;
use crate::eip_712_signed_message::EIP712SignedMessage;
use crate::tap_receipt::{Receipt, ReceivedReceipt};
use alloy_primitives::Address;
use async_trait::async_trait;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

/// `RAVStorageAdapterMock` is a mock implementation of the `RAVStorageAdapter` trait.
///
/// It serves two main purposes:
///
/// 1. **Unit Testing**: The `RAVStorageAdapterMock` is primarily intended to be used for unit tests,
///    providing a way to simulate the behavior of a real `RAVStorageAdapter` without requiring a real
///    implementation. By using a mock implementation, you can create predictable behaviors and
///    responses, enabling isolated and focused testing of the logic that depends on the `RAVStorageAdapter` trait.
///
/// 2. **Example Implementation**: New users of the `RAVStorageAdapter` trait can look to
///    `RAVStorageAdapterMock` as a basic example of how to implement the trait.
///
/// Note: This mock implementation is not suitable for production use. Its methods simply manipulate a
/// local `RwLock<Option<SignedRAV>>`, and it provides no real error handling.
///
/// # Usage
///
/// To use `RAVStorageAdapterMock`, first create an `Arc<RwLock<Option<SignedRAV>>>`, then pass it to
/// `RAVStorageAdapterMock::new()`. Now, it can be used anywhere a `RAVStorageAdapter` is required.
///
/// ```rust
/// use std::sync::{Arc};
/// use tokio::sync::RwLock;
/// use tap_core::{tap_manager::SignedRAV, adapters::rav_storage_adapter_mock::RAVStorageAdapterMock};
///
/// let rav_storage: Arc<RwLock<Option<SignedRAV>>> = Arc::new(RwLock::new(None));
/// let adapter = RAVStorageAdapterMock::new(rav_storage);
/// ```
#[derive(Clone)]
pub struct AuditorExecutorMock {
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,

    sender_escrow_storage: Arc<RwLock<HashMap<Address, u128>>>,

    query_appraisals: Arc<RwLock<HashMap<u64, u128>>>,
    allocation_ids: Arc<RwLock<HashSet<Address>>>,
    sender_ids: Arc<RwLock<HashSet<Address>>>,
}

impl AuditorExecutorMock {
    pub fn new(
        receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
        sender_escrow_storage: Arc<RwLock<HashMap<Address, u128>>>,
        query_appraisals: Arc<RwLock<HashMap<u64, u128>>>,
        allocation_ids: Arc<RwLock<HashSet<Address>>>,
        sender_ids: Arc<RwLock<HashSet<Address>>>,
    ) -> Self {
        AuditorExecutorMock {
            receipt_storage,
            sender_escrow_storage,
            allocation_ids,
            sender_ids,
            query_appraisals,
        }
    }
}

impl AuditorExecutorMock {
    pub async fn escrow(&self, sender_id: Address) -> Result<u128, AdpaterErrorMock> {
        let sender_escrow_storage = self.sender_escrow_storage.read().await;
        if let Some(escrow) = sender_escrow_storage.get(&sender_id) {
            return Ok(*escrow);
        }
        Err(AdpaterErrorMock::AdapterError {
            error: "No escrow exists for provided sender ID.".to_owned(),
        })
    }

    pub async fn increase_escrow(&mut self, sender_id: Address, value: u128) {
        let mut sender_escrow_storage = self.sender_escrow_storage.write().await;

        if let Some(current_value) = sender_escrow_storage.get(&sender_id) {
            let mut sender_escrow_storage = self.sender_escrow_storage.write().await;
            sender_escrow_storage.insert(sender_id, current_value + value);
        } else {
            sender_escrow_storage.insert(sender_id, value);
        }
    }

    pub async fn reduce_escrow(
        &self,
        sender_id: Address,
        value: u128,
    ) -> Result<(), AdpaterErrorMock> {
        let mut sender_escrow_storage = self.sender_escrow_storage.write().await;

        if let Some(current_value) = sender_escrow_storage.get(&sender_id) {
            let checked_new_value = current_value.checked_sub(value);
            if let Some(new_value) = checked_new_value {
                sender_escrow_storage.insert(sender_id, new_value);
                return Ok(());
            }
        }
        Err(AdpaterErrorMock::AdapterError {
            error: "Provided value is greater than existing escrow.".to_owned(),
        })
    }
}

#[async_trait]
impl EscrowAdapter for AuditorExecutorMock {
    type AdapterError = AdpaterErrorMock;
    async fn get_available_escrow(&self, sender_id: Address) -> Result<u128, Self::AdapterError> {
        self.escrow(sender_id).await
    }
    async fn subtract_escrow(
        &self,
        sender_id: Address,
        value: u128,
    ) -> Result<(), Self::AdapterError> {
        self.reduce_escrow(sender_id, value).await
    }
}

#[async_trait]
impl ReceiptChecksAdapter for AuditorExecutorMock {
    type AdapterError = AdapterErrorMock;

    async fn is_unique(
        &self,
        receipt: &EIP712SignedMessage<Receipt>,
        receipt_id: u64,
    ) -> Result<bool, Self::AdapterError> {
        let receipt_storage = self.receipt_storage.read().await;
        Ok(receipt_storage
            .iter()
            .all(|(stored_receipt_id, stored_receipt)| {
                (stored_receipt.signed_receipt().message != receipt.message)
                    || *stored_receipt_id == receipt_id
            }))
    }

    async fn is_valid_allocation_id(
        &self,
        allocation_id: Address,
    ) -> Result<bool, Self::AdapterError> {
        let allocation_ids = self.allocation_ids.read().await;
        Ok(allocation_ids.contains(&allocation_id))
    }

    async fn is_valid_value(&self, value: u128, query_id: u64) -> Result<bool, Self::AdapterError> {
        let query_appraisals = self.query_appraisals.read().await;
        let appraised_value = query_appraisals.get(&query_id).unwrap();

        if value != *appraised_value {
            return Ok(false);
        }
        Ok(true)
    }

    async fn is_valid_sender_id(&self, sender_id: Address) -> Result<bool, Self::AdapterError> {
        let sender_ids = self.sender_ids.read().await;
        Ok(sender_ids.contains(&sender_id))
    }
}
