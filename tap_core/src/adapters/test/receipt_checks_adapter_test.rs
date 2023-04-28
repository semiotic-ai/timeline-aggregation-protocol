#[cfg(test)]
mod receipt_checks_adapter_unit_test {
    use crate::{
        adapters::{
            receipt_checks_adapter::ReceiptChecksAdapter,
            receipt_checks_adapter_mock::ReceiptChecksAdapterMock,
        },
        eip_712_signed_message::EIP712SignedMessage,
        tap_receipt::{get_full_list_of_checks, Receipt, ReceivedReceipt},
    };
    use ethereum_types::Address;
    use k256::ecdsa::SigningKey;
    use rand_core::OsRng;
    use rstest::*;
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
    };

    #[rstest]
    fn receipt_checks_adapter_test() {
        let gateway_ids = [
            Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
            Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
            Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
        ];
        let gateway_ids_set = HashSet::from(gateway_ids);

        let allocation_ids = [
            Address::from_str("0xabababababababababababababababababababab").unwrap(),
            Address::from_str("0xbabababababababababababababababababababa").unwrap(),
            Address::from_str("0xdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdf").unwrap(),
        ];
        let allocation_ids_set = HashSet::from(allocation_ids);

        let signing_key = SigningKey::random(&mut OsRng);
        let value = 100u128;

        let receipt_storage = (0..10)
            .into_iter()
            .map(|id| {
                (
                    id,
                    ReceivedReceipt::new(
                        EIP712SignedMessage::new(
                            Receipt::new(allocation_ids[0], value).unwrap(),
                            &signing_key,
                        )
                        .unwrap(),
                        id,
                        get_full_list_of_checks(),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        let query_appraisals = (0..11)
            .into_iter()
            .map(|id| (id, 100u128))
            .collect::<HashMap<_, _>>();

        let receipt_checks_adapter = ReceiptChecksAdapterMock::new(
            receipt_storage,
            query_appraisals,
            allocation_ids_set,
            gateway_ids_set,
        );

        let new_receipt = (
            10u64,
            ReceivedReceipt::new(
                EIP712SignedMessage::new(
                    Receipt::new(allocation_ids[0], value).unwrap(),
                    &signing_key,
                )
                .unwrap(),
                10u64,
                get_full_list_of_checks(),
            ),
        );

        assert!(receipt_checks_adapter.is_unique(&new_receipt.1.signed_receipt));
        assert!(receipt_checks_adapter
            .is_valid_allocation_id(new_receipt.1.signed_receipt.message.allocation_id));
        // TODO: Add check when gateway_id is available from received receipt (issue: #56)
        // assert!(receipt_checks_adapter.is_valid_gateway_id(gateway_id));
        assert!(receipt_checks_adapter.is_valid_value(
            new_receipt.1.signed_receipt.message.value,
            new_receipt.1.query_id
        ));
    }
}
