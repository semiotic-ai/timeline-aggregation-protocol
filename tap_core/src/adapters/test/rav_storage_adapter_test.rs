// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod rav_storage_adapter_unit_test {
    use std::{str::FromStr, sync::Arc};

    use alloy_primitives::Address;
    use alloy_sol_types::Eip712Domain;
    use ethers::signers::coins_bip39::English;
    use ethers::signers::{LocalWallet, MnemonicBuilder};
    use rstest::*;
    use tokio::sync::RwLock;

    use crate::adapters::rav_storage_adapter::RAVRead;
    use crate::adapters::{
        rav_storage_adapter::RAVStore, rav_storage_adapter_mock::RAVStorageAdapterMock,
    };
    use crate::tap_eip712_domain;
    use crate::{
        eip_712_signed_message::EIP712SignedMessage,
        receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
    };

    #[fixture]
    fn domain_separator() -> Eip712Domain {
        tap_eip712_domain(1, Address::from([0x11u8; 20]))
    }

    #[rstest]
    #[tokio::test]
    async fn rav_storage_adapter_test(domain_separator: Eip712Domain) {
        let rav_storage = Arc::new(RwLock::new(None));
        let rav_storage_adapter = RAVStorageAdapterMock::new(rav_storage);

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();

        let allocation_id: [u8; 20] =
            Address::from_str("0xabababababababababababababababababababab")
                .unwrap()
                .into();
        let allocation_id = allocation_id.into();

        // Create receipts
        let mut receipts = Vec::new();
        for value in 50..60 {
            receipts.push(
                EIP712SignedMessage::new(
                    &domain_separator,
                    Receipt::new(allocation_id, value).unwrap(),
                    &wallet,
                )
                .unwrap(),
            );
        }

        let signed_rav = EIP712SignedMessage::new(
            &domain_separator,
            ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None).unwrap(),
            &wallet,
        )
        .unwrap();

        rav_storage_adapter
            .update_last_rav(signed_rav.clone())
            .await
            .unwrap();

        // Retreive rav
        let retrieved_rav = rav_storage_adapter.last_rav().await;
        assert!(retrieved_rav.unwrap().unwrap() == signed_rav);

        // Testing the last rav update...

        // Create more receipts
        let mut receipts = Vec::new();
        for value in 60..70 {
            receipts.push(
                EIP712SignedMessage::new(
                    &domain_separator,
                    Receipt::new(allocation_id, value).unwrap(),
                    &wallet,
                )
                .unwrap(),
            );
        }

        let signed_rav = EIP712SignedMessage::new(
            &domain_separator,
            ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None).unwrap(),
            &wallet,
        )
        .unwrap();

        // Update the last rav
        rav_storage_adapter
            .update_last_rav(signed_rav.clone())
            .await
            .unwrap();

        // Retreive rav
        let retrieved_rav = rav_storage_adapter.last_rav().await;
        assert!(retrieved_rav.unwrap().unwrap() == signed_rav);
    }
}
