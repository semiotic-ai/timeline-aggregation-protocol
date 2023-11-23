// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod escrow_adapter_unit_test {
    use std::{collections::HashMap, sync::Arc};

    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;
    use tokio::sync::RwLock;

    use crate::adapters::{escrow_adapter::EscrowAdapter, escrow_adapter_mock::EscrowAdapterMock};

    #[rstest]
    #[tokio::test]
    async fn escrow_adapter_test() {
        let escrow_storage = Arc::new(RwLock::new(HashMap::new()));
        let mut escrow_adapter = EscrowAdapterMock::new(escrow_storage);

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
        let sender_id: [u8; 20] = wallet.address().into();
        let sender_id = sender_id.into();

        let invalid_wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(
                "wrong century settle satisfy market forest title connect ten push alley depend",
            )
            .build()
            .unwrap();
        let invalid_sender_id: [u8; 20] = invalid_wallet.address().into();
        let invalid_sender_id = invalid_sender_id.into();

        let initial_value = 500u128;

        escrow_adapter
            .increase_escrow(sender_id, initial_value)
            .await;

        // Check that sender exists and has valid value through adapter
        assert!(escrow_adapter.get_available_escrow(sender_id).await.is_ok());
        assert_eq!(
            escrow_adapter
                .get_available_escrow(sender_id)
                .await
                .unwrap(),
            initial_value
        );

        // Check that subtracting is valid for valid sender, and results in expected value
        assert!(escrow_adapter
            .subtract_escrow(sender_id, initial_value)
            .await
            .is_ok());
        assert!(escrow_adapter.get_available_escrow(sender_id).await.is_ok());
        assert_eq!(
            escrow_adapter
                .get_available_escrow(sender_id)
                .await
                .unwrap(),
            0
        );

        // Check that subtracting to negative escrow results in err
        assert!(escrow_adapter
            .subtract_escrow(sender_id, initial_value)
            .await
            .is_err());

        // Check that accessing non initialized sender results in err
        assert!(escrow_adapter
            .get_available_escrow(invalid_sender_id)
            .await
            .is_err());
    }
}
