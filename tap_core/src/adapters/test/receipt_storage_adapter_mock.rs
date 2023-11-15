// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, ops::RangeBounds, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    adapters::receipt_storage_adapter::{safe_truncate_receipts, ReceiptStorageAdapter},
    tap_receipt::ReceivedReceipt,
};

pub struct ReceiptStorageAdapterMock {
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
    unique_id: RwLock<u64>,
}

impl ReceiptStorageAdapterMock {
    pub fn new(receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>) -> Self {
        Self {
            receipt_storage,
            unique_id: RwLock::new(0u64),
        }
    }
    pub async fn retrieve_receipt_by_id(
        &self,
        receipt_id: u64,
    ) -> Result<ReceivedReceipt, AdapterErrorMock> {
        let receipt_storage = self.receipt_storage.read().await;

        receipt_storage
            .get(&receipt_id)
            .cloned()
            .ok_or(AdapterErrorMock::AdapterError {
                error: "No receipt found with ID".to_owned(),
            })
    }
    pub async fn retrieve_receipts_by_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, AdapterErrorMock> {
        let receipt_storage = self.receipt_storage.read().await;
        Ok(receipt_storage
            .iter()
            .filter(|(_, rx_receipt)| {
                rx_receipt.signed_receipt.message.timestamp_ns == timestamp_ns
            })
            .map(|(&id, rx_receipt)| (id, rx_receipt.clone()))
            .collect())
    }
    pub async fn retrieve_receipts_upto_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, AdapterErrorMock> {
        self.retrieve_receipts_in_timestamp_range(..=timestamp_ns, None)
            .await
    }
    pub async fn remove_receipt_by_id(&mut self, receipt_id: u64) -> Result<(), AdapterErrorMock> {
        let mut receipt_storage = self.receipt_storage.write().await;
        receipt_storage
            .remove(&receipt_id)
            .map(|_| ())
            .ok_or(AdapterErrorMock::AdapterError {
                error: "No receipt found with ID".to_owned(),
            })
    }
    pub async fn remove_receipts_by_ids(
        &mut self,
        receipt_ids: &[u64],
    ) -> Result<(), AdapterErrorMock> {
        for receipt_id in receipt_ids {
            self.remove_receipt_by_id(*receipt_id).await?;
        }
        Ok(())
    }
}

use thiserror::Error;
#[derive(Debug, Error)]
pub enum AdapterErrorMock {
    #[error("something went wrong: {error}")]
    AdapterError { error: String },
}

#[async_trait]
impl ReceiptStorageAdapter for ReceiptStorageAdapterMock {
    type AdapterError = AdapterErrorMock;
    async fn store_receipt(&self, receipt: ReceivedReceipt) -> Result<u64, Self::AdapterError> {
        let mut id_pointer = self.unique_id.write().await;
        let id_previous = *id_pointer;
        let mut receipt_storage = self.receipt_storage.write().await;
        receipt_storage.insert(*id_pointer, receipt);
        *id_pointer += 1;
        Ok(id_previous)
    }
    async fn retrieve_receipts_in_timestamp_range<R: RangeBounds<u64> + std::marker::Send>(
        &self,
        timestamp_range_ns: R,
        limit: Option<u64>,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, Self::AdapterError> {
        let receipt_storage = self.receipt_storage.read().await;
        let mut receipts_in_range: Vec<(u64, ReceivedReceipt)> = receipt_storage
            .iter()
            .filter(|(_, rx_receipt)| {
                timestamp_range_ns.contains(&rx_receipt.signed_receipt.message.timestamp_ns)
            })
            .map(|(&id, rx_receipt)| (id, rx_receipt.clone()))
            .collect();

        if limit.is_some_and(|limit| receipts_in_range.len() > limit as usize) {
            safe_truncate_receipts(&mut receipts_in_range, limit.unwrap());

            Ok(receipts_in_range)
        } else {
            Ok(receipts_in_range)
        }
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
            !timestamp_ns.contains(&rx_receipt.signed_receipt.message.timestamp_ns)
        });
        Ok(())
    }
}
