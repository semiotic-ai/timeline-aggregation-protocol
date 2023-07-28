// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod rav_storage_adapter_unit_test {
    use std::{str::FromStr, sync::Arc};

    use ethereum_types::Address;
    use ethers::signers::coins_bip39::English;
    use ethers::signers::{LocalWallet, MnemonicBuilder};
    use rstest::*;
    use tokio::sync::RwLock;

    use crate::adapters::{
        rav_storage_adapter::RAVStorageAdapter, rav_storage_adapter_mock::RAVStorageAdapterMock,
    };
    use crate::{
        eip_712_signed_message::EIP712SignedMessage,
        receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
    };

    #[rstest]
    #[tokio::test]
    async fn rav_storage_adapter_test() {
        let rav_storage = Arc::new(RwLock::new(None));
        let rav_storage_adapter = RAVStorageAdapterMock::new(rav_storage);

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();

        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();

        // Create receipts
        let mut receipts = Vec::new();
        for value in 50..60 {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_id, value).unwrap(), &wallet)
                    .await
                    .unwrap(),
            );
        }

        let signed_rav = EIP712SignedMessage::new(
            ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None).unwrap(),
            &wallet,
        )
        .await
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
                EIP712SignedMessage::new(Receipt::new(allocation_id, value).unwrap(), &wallet)
                    .await
                    .unwrap(),
            );
        }

        let signed_rav = EIP712SignedMessage::new(
            ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None).unwrap(),
            &wallet,
        )
        .await
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
