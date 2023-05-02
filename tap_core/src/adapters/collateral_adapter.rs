// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use ethereum_types::Address;

pub trait CollateralAdapter {
    /// User defined error type;
    type AdapterError: std::error::Error + std::fmt::Debug;

    fn get_available_collateral(&self, gateway_id: Address) -> Result<u128, Self::AdapterError>;
    fn subtract_collateral(
        &mut self,
        gateway_id: Address,
        value: u128,
    ) -> Result<(), Self::AdapterError>;
}
