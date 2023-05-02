// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use ethereum_types::Address;

use crate::adapters::collateral_adapter::CollateralAdapter;

pub struct CollateralAdapterMock {
    gateway_collateral_storage: HashMap<Address, u128>,
}

use thiserror::Error;
#[derive(Debug, Error)]
pub enum AdpaterErrorMock {
    #[error("something went wrong: {Error}")]
    AdapterError { Error: String },
}

impl CollateralAdapterMock {
    pub fn new() -> Self {
        CollateralAdapterMock {
            gateway_collateral_storage: <HashMap<Address, u128>>::new(),
        }
    }
    pub fn collateral(&self, gateway_id: Address) -> Result<u128, AdpaterErrorMock> {
        if let Some(collateral) = self.gateway_collateral_storage.get(&gateway_id) {
            return Ok(*collateral);
        }
        Err(AdpaterErrorMock::AdapterError {
            Error: "No collateral exists for provided gateway ID.".to_owned(),
        })
    }

    pub fn increase_collateral(&mut self, gateway_id: Address, value: u128) {
        if let Some(current_value) = self.gateway_collateral_storage.get(&gateway_id) {
            self.gateway_collateral_storage
                .insert(gateway_id, current_value + value);
        } else {
            self.gateway_collateral_storage.insert(gateway_id, value);
        }
    }

    pub fn reduce_collateral(
        &mut self,
        gateway_id: Address,
        value: u128,
    ) -> Result<(), AdpaterErrorMock> {
        if let Some(current_value) = self.gateway_collateral_storage.get(&gateway_id) {
            let checked_new_value = current_value.checked_sub(value);
            if let Some(new_value) = checked_new_value {
                self.gateway_collateral_storage
                    .insert(gateway_id, new_value);
                return Ok(());
            }
        }
        Err(AdpaterErrorMock::AdapterError {
            Error: "Provided value is greater than existing collateral.".to_owned(),
        })
    }
}

impl Default for CollateralAdapterMock {
    fn default() -> Self {
        Self::new()
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
