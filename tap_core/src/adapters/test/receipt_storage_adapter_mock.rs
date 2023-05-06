// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    adapters::receipt_storage_adapter::ReceiptStorageAdapter, tap_receipt::ReceivedReceipt,
};

pub struct ReceiptStorageAdapterMock {
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
    unique_id: u64,
}

impl ReceiptStorageAdapterMock {
    pub fn new(receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>) -> Self {
        Self {
            receipt_storage: receipt_storage,
            unique_id: 0u64,
        }
    }
}

use thiserror::Error;
#[derive(Debug, Error)]
pub enum AdpaterErrorMock {
    #[error("something went wrong: {error}")]
    AdapterError { error: String },
}

impl ReceiptStorageAdapter for ReceiptStorageAdapterMock {
    type AdapterError = AdpaterErrorMock;
    fn store_receipt(&mut self, receipt: ReceivedReceipt) -> Result<u64, Self::AdapterError> {
        let id = self.unique_id;
        let mut receipt_storage = self.receipt_storage.write().unwrap();
        receipt_storage.insert(id, receipt);
        self.unique_id += 1;
        Ok(id)
    }
    fn retrieve_receipt_by_id(
        &self,
        receipt_id: u64,
    ) -> Result<ReceivedReceipt, Self::AdapterError> {
        let receipt_storage = self.receipt_storage.read().unwrap();

        receipt_storage
            .get(&receipt_id)
            .cloned()
            .ok_or(AdpaterErrorMock::AdapterError {
                error: "No receipt found with ID".to_owned(),
            })
    }
    fn retrieve_receipts_by_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, Self::AdapterError> {
        let receipt_storage = self.receipt_storage.read().unwrap();
        Ok(receipt_storage
            .iter()
            .filter(|(_, rx_receipt)| {
                rx_receipt.signed_receipt.message.timestamp_ns == timestamp_ns
            })
            .map(|(&id, rx_receipt)| (id, rx_receipt.clone()))
            .collect())
    }
    fn retrieve_receipts_upto_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, Self::AdapterError> {
        let receipt_storage = self.receipt_storage.read().unwrap();
        Ok(receipt_storage
            .iter()
            .filter(|(_, rx_receipt)| {
                rx_receipt.signed_receipt.message.timestamp_ns <= timestamp_ns
            })
            .map(|(&id, rx_receipt)| (id, rx_receipt.clone()))
            .collect())
    }
    fn update_receipt_by_id(
        &mut self,
        receipt_id: u64,
        receipt: ReceivedReceipt,
    ) -> Result<(), Self::AdapterError> {
        let mut receipt_storage = self.receipt_storage.write().unwrap();

        if !receipt_storage.contains_key(&receipt_id) {
            return Err(AdpaterErrorMock::AdapterError {
                error: "Invalid receipt_id".to_owned(),
            });
        };

        receipt_storage.insert(receipt_id, receipt);
        self.unique_id += 1;
        Ok(())
    }
    fn remove_receipt_by_id(&mut self, receipt_id: u64) -> Result<(), Self::AdapterError> {
        let mut receipt_storage = self.receipt_storage.write().unwrap();
        receipt_storage
            .remove(&receipt_id)
            .map(|_| ())
            .ok_or(AdpaterErrorMock::AdapterError {
                error: "No receipt found with ID".to_owned(),
            })
    }
    fn remove_receipts_by_ids(&mut self, receipt_ids: &[u64]) -> Result<(), Self::AdapterError> {
        for receipt_id in receipt_ids {
            self.remove_receipt_by_id(*receipt_id)?;
        }
        Ok(())
    }
}
