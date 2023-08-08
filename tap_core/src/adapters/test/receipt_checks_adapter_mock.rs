// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use ethereum_types::Address;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::{
    adapters::receipt_checks_adapter::ReceiptChecksAdapter,
    eip_712_signed_message::EIP712SignedMessage,
    tap_receipt::{Receipt, ReceiptError, ReceivedReceipt},
};

pub struct ReceiptChecksAdapterMock {
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
    query_appraisals: Arc<RwLock<HashMap<u64, u128>>>,
    allocation_ids: Arc<RwLock<HashSet<Address>>>,
    gateway_ids: Arc<RwLock<HashSet<Address>>>,
}

#[derive(Debug, Error)]
pub enum AdapterErrorMock {
    #[error("something went wrong: {error}")]
    AdapterError { error: String },
}

impl From<AdapterErrorMock> for ReceiptError {
    fn from(val: AdapterErrorMock) -> Self {
        ReceiptError::CheckFailedToComplete {
            source_error_message: val.to_string(),
        }
    }
}

impl ReceiptChecksAdapterMock {
    pub fn new(
        receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
        query_appraisals: Arc<RwLock<HashMap<u64, u128>>>,
        allocation_ids: Arc<RwLock<HashSet<Address>>>,
        gateway_ids: Arc<RwLock<HashSet<Address>>>,
    ) -> Self {
        Self {
            receipt_storage,
            query_appraisals,
            allocation_ids,
            gateway_ids,
        }
    }
}

#[async_trait]
impl ReceiptChecksAdapter for ReceiptChecksAdapterMock {
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
                (stored_receipt.signed_receipt.message != receipt.message)
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

    async fn is_valid_gateway_id(&self, gateway_id: Address) -> Result<bool, Self::AdapterError> {
        let gateway_ids = self.gateway_ids.read().await;
        Ok(gateway_ids.contains(&gateway_id))
    }
}
