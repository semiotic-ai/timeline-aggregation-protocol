// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::escrow_adapter_mock::AdpaterErrorMock;
use crate::adapters::escrow_adapter::EscrowAdapter;
// use crate::adapters::receipt_checks_adapter::ReceiptChecksAdapter;
use crate::tap_receipt::ReceivedReceipt;
use alloy_primitives::Address;
use async_trait::async_trait;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

#[derive(Clone)]
#[allow(dead_code)]
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
