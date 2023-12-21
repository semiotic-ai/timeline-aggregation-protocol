// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy_primitives::Address;
use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::tap_receipt::{ReceiptError, ReceivedReceipt};

pub struct ReceiptChecksAdapterMock {
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
    query_appraisals: Arc<RwLock<HashMap<u64, u128>>>,
    allocation_ids: Arc<RwLock<HashSet<Address>>>,
    sender_ids: Arc<RwLock<HashSet<Address>>>,
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
        sender_ids: Arc<RwLock<HashSet<Address>>>,
    ) -> Self {
        Self {
            receipt_storage,
            query_appraisals,
            allocation_ids,
            sender_ids,
        }
    }
}
