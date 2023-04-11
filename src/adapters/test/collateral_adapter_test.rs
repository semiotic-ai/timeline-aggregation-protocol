#[cfg(test)]
mod collateral_adapter_unit_test {
    use crate::adapters::{
        collateral_adapter::CollateralAdapter, collateral_adapter_mock::CollateralAdapterMock,
    };
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rand_core::OsRng;
    use rstest::*;

    #[rstest]
    fn collateral_adapter_test() {
        let mut collateral_adapter = CollateralAdapterMock::new();

        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        let gateway_id = crate::verifying_key_to_address(&verifying_key);

        let invalid_signing_key = SigningKey::random(&mut OsRng);
        let invalid_verifying_key = VerifyingKey::from(&invalid_signing_key);
        let invalid_gateway_id = crate::verifying_key_to_address(&invalid_verifying_key);

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
