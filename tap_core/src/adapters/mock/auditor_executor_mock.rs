// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::adapters::escrow_adapter::EscrowAdapter;
use alloy_primitives::Address;
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use super::executor_mock::AdapterErrorMock;


#[derive(Clone)]
pub struct AuditorExecutorMock {
    sender_escrow_storage: Arc<RwLock<HashMap<Address, u128>>>,
}

impl AuditorExecutorMock {
    pub fn new(sender_escrow_storage: Arc<RwLock<HashMap<Address, u128>>>) -> Self {
        AuditorExecutorMock {
            sender_escrow_storage,
        }
    }
}

impl AuditorExecutorMock {
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
impl EscrowAdapter for AuditorExecutorMock {
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
