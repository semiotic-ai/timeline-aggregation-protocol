// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::{Arc, RwLock}};

use ethereum_types::Address;

use crate::adapters::collateral_adapter::CollateralAdapter;

pub struct CollateralAdapterMock {
    gateway_collateral_storage: Arc<RwLock<HashMap<Address, u128>>>,
}

use thiserror::Error;
#[derive(Debug, Error)]
pub enum AdpaterErrorMock {
    #[error("something went wrong: {error}")]
    AdapterError { error: String },
}

impl CollateralAdapterMock {
    pub fn new(gateway_collateral_storage: Arc<RwLock<HashMap<Address, u128>>>) -> Self {
        CollateralAdapterMock {
            gateway_collateral_storage,
        }
    }
    pub fn collateral(&self, gateway_id: Address) -> Result<u128, AdpaterErrorMock> {
        let gateway_collateral_storage = self.gateway_collateral_storage.read().unwrap();
        if let Some(collateral) = gateway_collateral_storage.get(&gateway_id) {
            return Ok(*collateral);
        }
        Err(AdpaterErrorMock::AdapterError {
            error: "No collateral exists for provided gateway ID.".to_owned(),
        })
    }

    pub fn increase_collateral(&mut self, gateway_id: Address, value: u128) {
        let mut gateway_collateral_storage = self.gateway_collateral_storage.write().unwrap();

        if let Some(current_value) = gateway_collateral_storage.get(&gateway_id) {
            let mut gateway_collateral_storage = self.gateway_collateral_storage.write().unwrap();
            gateway_collateral_storage.insert(gateway_id, current_value + value);
        } else {
            gateway_collateral_storage.insert(gateway_id, value);
        }
    }

    pub fn reduce_collateral(
        &mut self,
        gateway_id: Address,
        value: u128,
    ) -> Result<(), AdpaterErrorMock> {
        let mut gateway_collateral_storage = self.gateway_collateral_storage.write().unwrap();

        if let Some(current_value) = gateway_collateral_storage.get(&gateway_id) {
            let checked_new_value = current_value.checked_sub(value);
            if let Some(new_value) = checked_new_value {
                gateway_collateral_storage.insert(gateway_id, new_value);
                return Ok(());
            }
        }
        Err(AdpaterErrorMock::AdapterError {
            error: "Provided value is greater than existing collateral.".to_owned(),
        })
    }
}

impl CollateralAdapter for CollateralAdapterMock {
    type AdapterError = AdpaterErrorMock;
    fn get_available_collateral(&self, gateway_id: Address) -> Result<u128, Self::AdapterError> {
        self.collateral(gateway_id)
    }
    fn subtract_collateral(
        &mut self,
        gateway_id: Address,
        value: u128,
    ) -> Result<(), Self::AdapterError> {
        self.reduce_collateral(gateway_id, value)
    }
}
