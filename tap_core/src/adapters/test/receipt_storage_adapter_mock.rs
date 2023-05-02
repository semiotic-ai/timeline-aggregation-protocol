// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::{
    adapters::receipt_storage_adapter::ReceiptStorageAdapter, tap_receipt::ReceivedReceipt,
};

#[derive(Default)]
pub struct ReceiptAdapterMock {
    receipt_storage: HashMap<u64, ReceivedReceipt>,
    unique_id: u64,
}

impl ReceiptAdapterMock {
    pub fn new() -> Self {
        ReceiptAdapterMock {
            receipt_storage: <HashMap<u64, ReceivedReceipt>>::new(),
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

impl ReceiptStorageAdapter for ReceiptAdapterMock {
    type AdapterError = AdpaterErrorMock;
    fn store_receipt(&mut self, receipt: ReceivedReceipt) -> Result<u64, Self::AdapterError> {
        let id = self.unique_id;
        self.receipt_storage.insert(id, receipt);
        self.unique_id += 1;
        Ok(id)
    }
    fn retrieve_receipt_by_id(
        &self,
        receipt_id: u64,
    ) -> Result<ReceivedReceipt, Self::AdapterError> {
        self.receipt_storage
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
        Ok(self
            .receipt_storage
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
        Ok(self
            .receipt_storage
            .iter()
            .filter(|(_, rx_receipt)| {
                rx_receipt.signed_receipt.message.timestamp_ns <= timestamp_ns
            })
            .map(|(&id, rx_receipt)| (id, rx_receipt.clone()))
            .collect())
    }
    fn remove_receipt_by_id(&mut self, receipt_id: u64) -> Result<(), Self::AdapterError> {
        self.receipt_storage
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
