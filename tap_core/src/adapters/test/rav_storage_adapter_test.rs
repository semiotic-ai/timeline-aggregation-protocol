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
    use k256::ecdsa::SigningKey;
    use rand_core::OsRng;
    use rstest::*;
    use std::str::FromStr;

    #[rstest]
    fn rav_storage_adapter_test() {
        let mut rav_storage_adapter = RAVStorageAdapterMock::new();

        let signing_key = SigningKey::random(&mut OsRng);

        let allocation_id =
            Address::from_str("0xabababababababababababababababababababab").unwrap();

        // Create receipts
        let mut receipts = Vec::new();
        for value in 50..60 {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_id, value).unwrap(), &signing_key)
                    .unwrap(),
            );
        }

        let signed_rav = EIP712SignedMessage::new(
            ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None).unwrap(),
            &signing_key,
        )
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
