// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0


//! # Timeline Aggregation Protocol
//!
//! ## Overview
//!
//! The TAP (Timeline Aggregation Protocol) facilitates a series of payments from
//! a sender to a receiver, who aggregates these payments into a single payment.
//! This aggregate payment can then be verified on-chain by a payment verifier,
//! reducing the number of transactions and simplifying the payment process.
//!
//! ## Key Components
//!
//! - **Sender:** Initiates the payment.
//! - **Receiver:** Receives the payment.
//! - **Signers:** Multiple signers authorized by the sender to sign receipts.
//! - **State Channel:** A one-way channel opened by the sender with the receiver
//! for sending receipts.
//! - **Receipt:** A record of payment sent by the sender to the receiver.
//! - **ReceiptAggregateVoucher (RAV):** A signed message containing the aggregate
//! value of the receipts.
//! - **tap_aggregator:** An entity managed by the sender that signs RAV requests.
//! - **EscrowAccount:** An account created in the blockchain to hold funds for
//! the sender-receiver pair.
//!
//! ## Security Measures
//!
//! - The protocol uses asymmetric cryptography to sign and verify messages,
//! ensuring the integrity of receipts and RAVs.
//!
//! ## Process
//!
//! 1. **Opening a State Channel:** A state channel is opened via a blockchain
//! contract, creating an EscrowAccount for the sender-receiver pair.
//! 2. **Sending Receipts:** The sender sends receipts to the receiver through
//! the state channel.
//! 3. **Storing Receipts:** The receiver stores the receipts and tracks the
//! aggregate payment.
//! 4. **Creating a RAV Request:** A RAV request consists of a list of receipts
//! and, optionally, the previous RAV.
//! 5. **Signing the RAV:** The receiver sends the RAV request to the
//! tap_aggregator, which signs it into a RAV.
//! 6. **Tracking Aggregate Value:** The receiver tracks the aggregate value and
//! new receipts since the last RAV.
//! 7. **Requesting a New RAV:** The receiver sends new receipts and the last
//! RAV to the tap_aggregator for a new RAV.
//! 8. **Closing the State Channel:** When the allocation period ends, the receiver
//! can send the last RAV to the blockchain and receive payment from the EscrowAccount.
//!
//! ## Performance Considerations
//!
//! - The primary performance limitations are the time required to verify receipts
//! and network limitations for sending requests to the tap_aggregator.
//!
//! ## Use Cases
//!
//! - The TAP protocol is suitable for systems with sequential operations that
//! are too expensive to redeem individually on-chain. By aggregating operations
//! and redeeming them in one transaction, costs can be reduced.
//!
//! ## Compatibility
//!
//! - The current implementation is for EVM-compatible blockchains, with most
//! of the system being off-chain.
//!
//! ## Getting started
//!
//! To get started with the TAP protocol, take a look on the [`manager`] module
//! to see how to manage the state channel and implement the needed adapters.

use std::time::{SystemTime, UNIX_EPOCH};

use alloy_sol_types::eip712_domain;
use thiserror::Error;

mod error;
pub mod manager;
pub mod rav;
pub mod receipt;
pub mod signed_message;

pub use error::Error;
use error::Result;

fn get_current_timestamp_u64_ns() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| Error::InvalidSystemTime {
            source_error_message: err.to_string(),
        })?
        .as_nanos() as u64)
}

/// The EIP712 domain separator builder for the TAP protocol.
///
/// This is the current domain separator that is used for the [EIP712](https://eips.ethereum.org/EIPS/eip-712) signature scheme.
///
///
/// It's used to validate the signature of the `ReceiptAggregateVoucher` and `Receipt` structs.
///
/// You can take a look on deployed [TAPVerfiers](https://github.com/semiotic-ai/timeline-aggregation-protocol-contracts/blob/4dc87fc616680c924b99dbaf285bdd449c777261/src/TAPVerifier.sol)
/// contracts [here](https://github.com/semiotic-ai/timeline-aggregation-protocol-contracts/blob/4dc87fc616680c924b99dbaf285bdd449c777261/addresses.json)
///
/// The domain separator is defined as:
/// - `name`: "TAP"
/// - `version`: "1"
/// - `chain_id`: The chain ID of the chain where the domain separator is deployed.
/// - `verifying_contract`: The address of the contract that is verifying the signature.
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
        rav::ReceiptAggregateVoucher, receipt::Receipt, signed_message::EIP712SignedMessage,
        tap_eip712_domain,
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
    #[test]
    fn signed_rav_is_valid_with_no_previous_rav(
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
                .unwrap(),
            );
        }

        // Skipping receipts validation in this test, aggregate_receipts assumes receipts are valid.

        let rav = ReceiptAggregateVoucher::aggregate_receipts(allocation_ids[0], &receipts, None)
            .unwrap();
        let signed_rav = EIP712SignedMessage::new(&domain_separator, rav, &keys.0).unwrap();
        assert!(signed_rav.recover_signer(&domain_separator).unwrap() == keys.1);
    }

    #[rstest]
    #[case::basic_rav_test(vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![0,0,0,0])]
    #[test]
    fn signed_rav_is_valid_with_previous_rav(
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
        let signed_prev_rav =
            EIP712SignedMessage::new(&domain_separator, prev_rav, &keys.0).unwrap();

        // Create new RAV from last half of receipts and prev_rav
        let rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[receipts.len() / 2..receipts.len()],
            Some(signed_prev_rav),
        )
        .unwrap();
        let signed_rav = EIP712SignedMessage::new(&domain_separator, rav, &keys.0).unwrap();

        assert!(signed_rav.recover_signer(&domain_separator).unwrap() == keys.1);
    }

    #[rstest]
    #[test]
    fn verify_signature(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
    ) {
        let signed_message = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(allocation_ids[0], 42).unwrap(),
            &keys.0,
        )
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
