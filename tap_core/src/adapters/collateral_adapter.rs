// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use ethereum_types::Address;

pub trait CollateralAdapter<T> {
    fn get_available_collateral(&self, gateway_id: Address) -> Result<u128, T>;
    fn subtract_collateral(&mut self, gateway_id: Address, value: u128) -> Result<(), T>;
}
