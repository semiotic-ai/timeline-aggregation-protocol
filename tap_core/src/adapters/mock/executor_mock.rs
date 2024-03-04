// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::adapters::escrow_adapter::EscrowAdapter;
use crate::adapters::receipt_storage_adapter::{
    safe_truncate_receipts, ReceiptRead, ReceiptStore, StoredReceipt,
};
use crate::tap_receipt::ReceivedReceipt;
use crate::{
    adapters::rav_storage_adapter::{RAVRead, RAVStore},
    tap_manager::SignedRAV,
};
use alloy_primitives::Address;
use async_trait::async_trait;
use std::ops::RangeBounds;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub type EscrowStorage = Arc<RwLock<HashMap<Address, u128>>>;
pub type QueryAppraisals = Arc<RwLock<HashMap<u64, u128>>>;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterErrorMock {
    #[error("something went wrong: {error}")]
    AdapterError { error: String },
}

#[derive(Clone)]
pub struct ExecutorMock {
    /// local RAV store with rwlocks to allow sharing with other compenents as needed
    rav_storage: Arc<RwLock<Option<SignedRAV>>>,
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
    unique_id: Arc<RwLock<u64>>,

    sender_escrow_storage: EscrowStorage,
}

impl ExecutorMock {
    pub fn new(
        rav_storage: Arc<RwLock<Option<SignedRAV>>>,
        receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
        sender_escrow_storage: Arc<RwLock<HashMap<Address, u128>>>,
    ) -> Self {
        ExecutorMock {
            rav_storage,
            receipt_storage,
            unique_id: Arc::new(RwLock::new(0)),
            sender_escrow_storage,
        }
    }
}

#[async_trait]
impl RAVStore for ExecutorMock {
    type AdapterError = AdapterErrorMock;

    async fn update_last_rav(&self, rav: SignedRAV) -> Result<(), Self::AdapterError> {
        let mut rav_storage = self.rav_storage.write().await;
        *rav_storage = Some(rav);
        Ok(())
    }
}

#[async_trait]
impl RAVRead for ExecutorMock {
    type AdapterError = AdapterErrorMock;

    async fn last_rav(&self) -> Result<Option<SignedRAV>, Self::AdapterError> {
        Ok(self.rav_storage.read().await.clone())
    }
}

#[async_trait]
impl ReceiptStore for ExecutorMock {
    type AdapterError = AdapterErrorMock;
    async fn store_receipt(&self, receipt: ReceivedReceipt) -> Result<u64, Self::AdapterError> {
        let mut id_pointer = self.unique_id.write().await;
        let id_previous = *id_pointer;
        let mut receipt_storage = self.receipt_storage.write().await;
        receipt_storage.insert(*id_pointer, receipt);
        *id_pointer += 1;
        Ok(id_previous)
    }
    async fn update_receipt_by_id(
        &self,
        receipt_id: u64,
        receipt: ReceivedReceipt,
    ) -> Result<(), Self::AdapterError> {
        let mut receipt_storage = self.receipt_storage.write().await;

        if !receipt_storage.contains_key(&receipt_id) {
            return Err(AdapterErrorMock::AdapterError {
                error: "Invalid receipt_id".to_owned(),
            });
        };

        receipt_storage.insert(receipt_id, receipt);
        *self.unique_id.write().await += 1;
        Ok(())
    }
    async fn remove_receipts_in_timestamp_range<R: RangeBounds<u64> + std::marker::Send>(
        &self,
        timestamp_ns: R,
    ) -> Result<(), Self::AdapterError> {
        let mut receipt_storage = self.receipt_storage.write().await;
        receipt_storage.retain(|_, rx_receipt| {
            !timestamp_ns.contains(&rx_receipt.signed_receipt().message.timestamp_ns)
        });
        Ok(())
    }
}

#[async_trait]
impl ReceiptRead for ExecutorMock {
    type AdapterError = AdapterErrorMock;
    async fn retrieve_receipts_in_timestamp_range<R: RangeBounds<u64> + std::marker::Send>(
        &self,
        timestamp_range_ns: R,
        limit: Option<u64>,
    ) -> Result<Vec<StoredReceipt>, Self::AdapterError> {
        let receipt_storage = self.receipt_storage.read().await;
        let mut receipts_in_range: Vec<(u64, ReceivedReceipt)> = receipt_storage
            .iter()
            .filter(|(_, rx_receipt)| {
                timestamp_range_ns.contains(&rx_receipt.signed_receipt().message.timestamp_ns)
            })
            .map(|(&id, rx_receipt)| (id, rx_receipt.clone()))
            .collect();

        if limit.is_some_and(|limit| receipts_in_range.len() > limit as usize) {
            safe_truncate_receipts(&mut receipts_in_range, limit.unwrap());
        }
        Ok(receipts_in_range.into_iter().map(|r| r.into()).collect())
    }
}

impl ExecutorMock {
    pub async fn escrow(&self, sender_id: Address) -> Result<u128, AdapterErrorMock> {
        let sender_escrow_storage = self.sender_escrow_storage.read().await;
        if let Some(escrow) = sender_escrow_storage.get(&sender_id) {
            return Ok(*escrow);
        }
        Err(AdapterErrorMock::AdapterError {
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
    ) -> Result<(), AdapterErrorMock> {
        let mut sender_escrow_storage = self.sender_escrow_storage.write().await;

        if let Some(current_value) = sender_escrow_storage.get(&sender_id) {
            let checked_new_value = current_value.checked_sub(value);
            if let Some(new_value) = checked_new_value {
                sender_escrow_storage.insert(sender_id, new_value);
                return Ok(());
            }
        }
        Err(AdapterErrorMock::AdapterError {
            error: "Provided value is greater than existing escrow.".to_owned(),
        })
    }
}

#[async_trait]
impl EscrowAdapter for ExecutorMock {
    type AdapterError = AdapterErrorMock;
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
