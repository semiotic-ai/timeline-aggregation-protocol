// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod receipt_checks_adapter_unit_test {
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
        sync::Arc,
    };

    use alloy_primitives::Address;
    use alloy_sol_types::Eip712Domain;
    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder};
    use futures::{stream, StreamExt};
    use rstest::*;
    use tokio::sync::RwLock;

    use crate::{
        checks::{mock::get_full_list_of_checks, ReceiptCheck},
        eip_712_signed_message::EIP712SignedMessage,
        tap_eip712_domain,
        tap_receipt::{Receipt, ReceivedReceipt},
    };

    #[fixture]
    fn domain_separator() -> Eip712Domain {
        tap_eip712_domain(1, Address::from([0x11u8; 20]))
    }

    #[fixture]
    fn checks(domain_separator: Eip712Domain) -> Vec<ReceiptCheck> {
        get_full_list_of_checks(
            domain_separator,
            HashSet::new(),
            Arc::new(RwLock::new(HashSet::new())),
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(HashMap::new())),
        )
    }

    #[rstest]
    #[tokio::test]
    async fn receipt_checks_adapter_test(
        domain_separator: Eip712Domain,
        checks: Vec<ReceiptCheck>,
    ) {
        let sender_ids = [
            Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
            Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
            Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
        ];
        let _sender_ids_set = Arc::new(RwLock::new(HashSet::from(sender_ids)));

        let allocation_ids = [
            Address::from_str("0xabababababababababababababababababababab").unwrap(),
            Address::from_str("0xbabababababababababababababababababababa").unwrap(),
            Address::from_str("0xdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdfdf").unwrap(),
        ];
        let _allocation_ids_set = Arc::new(RwLock::new(HashSet::from(allocation_ids)));

        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
        let value = 100u128;

        let receipts: HashMap<u64, ReceivedReceipt> = stream::iter(0..10)
            .then(|id| {
                let wallet = wallet.clone();
                let domain_separator = &domain_separator;
                let checks = checks.clone();
                async move {
                    (
                        id,
                        ReceivedReceipt::new(
                            EIP712SignedMessage::new(
                                domain_separator,
                                Receipt::new(allocation_ids[0], value).unwrap(),
                                &wallet,
                            )
                            .unwrap(),
                            id,
                            &checks,
                        ),
                    )
                }
            })
            .collect::<HashMap<_, _>>()
            .await;
        let receipt_storage = Arc::new(RwLock::new(receipts));

        let query_appraisals = (0..11).map(|id| (id, 100u128)).collect::<HashMap<_, _>>();

        let _query_appraisals_storage = Arc::new(RwLock::new(query_appraisals));

        // let receipt_checks_adapter = ReceiptChecksAdapterMock::new(
        //     Arc::clone(&receipt_storage),
        //     query_appraisals_storage,
        //     allocation_ids_set,
        //     sender_ids_set,
        // );

        let new_receipt = (
            10u64,
            ReceivedReceipt::new(
                EIP712SignedMessage::new(
                    &domain_separator,
                    Receipt::new(allocation_ids[0], value).unwrap(),
                    &wallet,
                )
                .unwrap(),
                10u64,
                &checks,
            ),
        );

        let unique_receipt_id = 0u64;
        receipt_storage
            .write()
            .await
            .insert(unique_receipt_id, new_receipt.1.clone());

        // assert!(receipt_checks_adapter
        //     .is_unique(new_receipt.1.signed_receipt(), unique_receipt_id)
        //     .await
        //     .unwrap());
        // assert!(receipt_checks_adapter
        //     .is_valid_allocation_id(new_receipt.1.signed_receipt().message.allocation_id)
        //     .await
        //     .unwrap());
        // TODO: Add check when sender_id is available from received receipt (issue: #56)
        // assert!(receipt_checks_adapter.is_valid_sender_id(sender_id));
        // assert!(receipt_checks_adapter
        //     .is_valid_value(
        //         new_receipt.1.signed_receipt().message.value,
        //         new_receipt.1.query_id()
        //     )
        //     .await
        //     .unwrap());
    }
}
