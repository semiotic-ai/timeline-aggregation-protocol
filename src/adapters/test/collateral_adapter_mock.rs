use ethereum_types::Address;
use std::collections::HashMap;

use crate::adapters::collateral_adapter::CollateralAdapter;

pub struct CollateralAdapterMock {
    gateway_collateral_storage: HashMap<Address, u128>,
}

impl CollateralAdapterMock {
    pub fn new() -> Self {
        CollateralAdapterMock {
            gateway_collateral_storage: <HashMap<Address, u128>>::new(),
        }
    }
    pub fn collateral(&self, gateway_id: Address) -> Result<u128, &'static str> {
        if let Some(collateral) = self.gateway_collateral_storage.get(&gateway_id) {
            return Ok(*collateral);
        }
        Err("No collateral exists for provided gateway ID.")
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
    ) -> Result<(), &'static str> {
        if let Some(current_value) = self.gateway_collateral_storage.get(&gateway_id) {
            let checked_new_value = current_value.checked_sub(value);
            if let Some(new_value) = checked_new_value {
                self.gateway_collateral_storage
                    .insert(gateway_id, new_value);
                return Ok(());
            }
        }
        Err("Provided value is greater than existing collateral.")
    }
}

impl Default for CollateralAdapterMock {
    fn default() -> Self {
        Self::new()
    }
}

impl CollateralAdapter<&'static str> for CollateralAdapterMock {
    fn get_available_collateral(&self, gateway_id: Address) -> Result<u128, &'static str> {
        self.collateral(gateway_id)
    }
    fn subtract_collateral(
        &mut self,
        gateway_id: Address,
        value: u128,
    ) -> Result<(), &'static str> {
        self.reduce_collateral(gateway_id, value)
    }
}
