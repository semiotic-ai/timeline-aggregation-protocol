#[cfg(test)]
mod rav_storage_adapter_unit_test {
    use crate::adapters::{
        rav_storage_adapter::RAVStorageAdapter, rav_storage_adapter_mock::RAVStorageAdapterMock,
    };
    use crate::{
        eip_712_signed_message::EIP712SignedMessage,
        receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
    };
    use ethereum_types::Address;
    use ethers::signers::coins_bip39::English;
    use ethers::signers::{LocalWallet, MnemonicBuilder};
    use rstest::*;
    use std::str::FromStr;

    #[rstest]
    async fn rav_storage_adapter_test() {
        let mut rav_storage_adapter = RAVStorageAdapterMock::new();

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

        let rav_id = rav_storage_adapter.store_rav(signed_rav).unwrap();

        // Retreive rav with id expected to be valid
        assert!(rav_storage_adapter.retrieve_rav_by_id(rav_id).is_ok());
        // Retreive rav with arbitrary id expected to be invalid
        assert!(rav_storage_adapter.retrieve_rav_by_id(999).is_err());

        // Remove rav with id expected to be valid
        assert!(rav_storage_adapter.remove_rav_by_id(rav_id).is_ok());
        // Remove rav with arbitrary id expected to be invalid
        assert!(rav_storage_adapter.remove_rav_by_id(999).is_err());

        // Retreive rav that was removed previously
        assert!(rav_storage_adapter.retrieve_rav_by_id(rav_id).is_err());

        // Remove rav that was removed previously
        assert!(rav_storage_adapter.retrieve_rav_by_id(rav_id).is_err());
    }
}
