// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use ethereum_types::Address;
use tokio::sync::RwLock;

use crate::adapters::escrow_adapter::EscrowAdapter;

pub struct EscrowAdapterMock {
    gateway_escrow_storage: Arc<RwLock<HashMap<Address, u128>>>,
}

use thiserror::Error;
#[derive(Debug, Error)]
pub enum AdpaterErrorMock {
    #[error("something went wrong: {error}")]
    AdapterError { error: String },
}

impl EscrowAdapterMock {
    pub fn new(gateway_escrow_storage: Arc<RwLock<HashMap<Address, u128>>>) -> Self {
        EscrowAdapterMock {
            gateway_escrow_storage,
        }
    }
    pub async fn escrow(&self, gateway_id: Address) -> Result<u128, AdpaterErrorMock> {
        let gateway_escrow_storage = self.gateway_escrow_storage.read().await;
        if let Some(escrow) = gateway_escrow_storage.get(&gateway_id) {
            return Ok(*escrow);
        }
        Err(AdpaterErrorMock::AdapterError {
            error: "No escrow exists for provided gateway ID.".to_owned(),
        })
    }

    pub async fn increase_escrow(&mut self, gateway_id: Address, value: u128) {
        let mut gateway_escrow_storage = self.gateway_escrow_storage.write().await;

        if let Some(current_value) = gateway_escrow_storage.get(&gateway_id) {
            let mut gateway_escrow_storage = self.gateway_escrow_storage.write().await;
            gateway_escrow_storage.insert(gateway_id, current_value + value);
        } else {
            gateway_escrow_storage.insert(gateway_id, value);
        }
    }

    pub async fn reduce_escrow(
        &self,
        gateway_id: Address,
        value: u128,
    ) -> Result<(), AdpaterErrorMock> {
        let mut gateway_escrow_storage = self.gateway_escrow_storage.write().await;

        if let Some(current_value) = gateway_escrow_storage.get(&gateway_id) {
            let checked_new_value = current_value.checked_sub(value);
            if let Some(new_value) = checked_new_value {
                gateway_escrow_storage.insert(gateway_id, new_value);
                return Ok(());
            }
        }
        Err(AdpaterErrorMock::AdapterError {
            error: "Provided value is greater than existing escrow.".to_owned(),
        })
    }
}

#[async_trait]
impl EscrowAdapter for EscrowAdapterMock {
    type AdapterError = AdpaterErrorMock;
    async fn get_available_escrow(&self, gateway_id: Address) -> Result<u128, Self::AdapterError> {
        self.escrow(gateway_id).await
    }
    async fn subtract_escrow(
        &self,
        gateway_id: Address,
        value: u128,
    ) -> Result<(), Self::AdapterError> {
        self.reduce_escrow(gateway_id, value).await
    }
}
