// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod collateral_adapter_unit_test {
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    };

    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;

    use crate::adapters::{
        collateral_adapter::CollateralAdapter, collateral_adapter_mock::CollateralAdapterMock,
    };

    #[rstest]
    fn collateral_adapter_test() {
        let collateral_storage = Arc::new(RwLock::new(HashMap::new()));
        let mut collateral_adapter = CollateralAdapterMock::new(collateral_storage);

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
        let gateway_id = wallet.address();

        let invalid_wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(
                "wrong century settle satisfy market forest title connect ten push alley depend",
            )
            .build()
            .unwrap();
        let invalid_gateway_id = invalid_wallet.address();

        let initial_value = 500u128;

        collateral_adapter.increase_collateral(gateway_id, initial_value);

        // Check that gateway exists and has valid value through adapter
        assert!(collateral_adapter
            .get_available_collateral(gateway_id)
            .is_ok());
        assert_eq!(
            collateral_adapter
                .get_available_collateral(gateway_id)
                .unwrap(),
            initial_value
        );

        // Check that subtracting is valid for valid gateway, and results in expected value
        assert!(collateral_adapter
            .subtract_collateral(gateway_id, initial_value)
            .is_ok());
        assert!(collateral_adapter
            .get_available_collateral(gateway_id)
            .is_ok());
        assert_eq!(
            collateral_adapter
                .get_available_collateral(gateway_id)
                .unwrap(),
            0
        );

        // Check that subtracting to negative collateral results in err
        assert!(collateral_adapter
            .subtract_collateral(gateway_id, initial_value)
            .is_err());

        // Check that accessing non initialized gateway results in err
        assert!(collateral_adapter
            .get_available_collateral(invalid_gateway_id)
            .is_err());
    }
}
