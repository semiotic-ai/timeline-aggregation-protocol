// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The Timeline Aggregation Protocol (TAP) is a micro-trust
//! state channel payment solution allowing one-way payments
//! from a payment sender to be aggregated then cheaply
//! verified on-chain by a payment receiver.

use std::time::{SystemTime, UNIX_EPOCH};

use alloy_sol_types::eip712_domain;
use thiserror::Error;

pub mod adapters;
pub mod checks;
pub mod eip_712_signed_message;
mod error;
pub mod receipt_aggregate_voucher;
pub mod tap_manager;
pub mod tap_receipt;

pub use error::{Error, Result};

pub(crate) fn get_current_timestamp_u64_ns() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::InvalidSystemTime {
            source_error_message: err.to_string(),
        })?
        .as_nanos() as u64)
}

pub fn tap_eip712_domain(
    chain_id: u64,
    verifying_contract_address: alloy_primitives::Address,
) -> alloy_sol_types::Eip712Domain {
    eip712_domain! {
        name: "TAP",
        version: "1",
        chain_id: chain_id,
        verifying_contract: verifying_contract_address,
    }
}

#[cfg(test)]
mod tap_tests {
    use std::str::FromStr;

    use alloy_primitives::Address;
    use alloy_sol_types::Eip712Domain;
    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;

    use crate::{
        eip_712_signed_message::EIP712SignedMessage,
        receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_eip712_domain,
        tap_receipt::Receipt,
    };

    #[fixture]
    fn keys() -> (LocalWallet, Address) {
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
        // Alloy library does not have feature parity with ethers library (yet) This workaround is needed to get the address
        // to convert to an alloy Address. This will not be needed when the alloy library has wallet support.
        let address: [u8; 20] = wallet.address().into();

        (wallet, address.into())
    }

    #[fixture]
    fn allocation_ids() -> Vec<Address> {
        vec![
            Address::from_str("0xabababababababababababababababababababab").unwrap(),
            Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead").unwrap(),
            Address::from_str("0xbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef").unwrap(),
            Address::from_str("0x1234567890abcdef1234567890abcdef12345678").unwrap(),
        ]
    }

    #[fixture]
    fn domain_separator() -> Eip712Domain {
        tap_eip712_domain(1, Address::from([0x11u8; 20]))
    }

    #[rstest]
    #[case::basic_rav_test (vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts (vec![0,0,0,0])]
    #[tokio::test]
    async fn signed_rav_is_valid_with_no_previous_rav(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        #[case] values: Vec<u128>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                EIP712SignedMessage::new(
                    &domain_separator,
                    Receipt::new(allocation_ids[0], value).unwrap(),
                    &keys.0,
                )
                .await
                .unwrap(),
            );
        }

        // Skipping receipts validation in this test, aggregate_receipts assumes receipts are valid.

        let rav = ReceiptAggregateVoucher::aggregate_receipts(allocation_ids[0], &receipts, None)
            .unwrap();
        let signed_rav = EIP712SignedMessage::new(&domain_separator, rav, &keys.0)
            .await
            .unwrap();
        assert!(signed_rav.recover_signer(&domain_separator).unwrap() == keys.1);
    }

    #[rstest]
    #[case::basic_rav_test(vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![0,0,0,0])]
    #[tokio::test]
    async fn signed_rav_is_valid_with_previous_rav(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        #[case] values: Vec<u128>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                EIP712SignedMessage::new(
                    &domain_separator,
                    Receipt::new(allocation_ids[0], value).unwrap(),
                    &keys.0,
                )
                .await
                .unwrap(),
            );
        }

        // Create previous RAV from first half of receipts
        let prev_rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[0..receipts.len() / 2],
            None,
        )
        .unwrap();
        let signed_prev_rav = EIP712SignedMessage::new(&domain_separator, prev_rav, &keys.0)
            .await
            .unwrap();

        // Create new RAV from last half of receipts and prev_rav
        let rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[receipts.len() / 2..receipts.len()],
            Some(signed_prev_rav),
        )
        .unwrap();
        let signed_rav = EIP712SignedMessage::new(&domain_separator, rav, &keys.0)
            .await
            .unwrap();

        assert!(signed_rav.recover_signer(&domain_separator).unwrap() == keys.1);
    }

    #[rstest]
    #[tokio::test]
    async fn verify_signature(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
    ) {
        let signed_message = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], 42).unwrap(),
            &keys.0,
        )
        .await
        .unwrap();

        assert!(signed_message.verify(&domain_separator, keys.1).is_ok());
        assert!(signed_message
            .verify(
                &domain_separator,
                Address::from_str("0x76f4eeD9fE41262669D0250b2A97db79712aD855").unwrap()
            )
            .is_err());
    }
}
