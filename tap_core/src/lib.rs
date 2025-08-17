// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]
//! ## Getting started
//!
//! To get started with the TAP protocol, take a look on the [`manager`] module
//! to see how to manage the state channel and implement the needed adapters.

use std::time::{SystemTime, UNIX_EPOCH};

use thegraph_core::alloy::{dyn_abi::Eip712Domain, primitives::Address, sol_types::eip712_domain};
use thiserror::Error;

mod error;
pub mod manager;
pub mod rav_request;
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
/// TAP protocol version for EIP-712 domain separator
#[derive(Debug, Clone, Copy)]
pub enum TapVersion {
    V1,
    V2,
}

impl TapVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            TapVersion::V1 => "1",
            TapVersion::V2 => "2",
        }
    }
}

/// The domain separator is defined as:
/// - `name`: "TAP"
/// - `version`: "1" or "2" depending on protocol version
/// - `chain_id`: The chain ID of the chain where the domain separator is deployed.
/// - `verifying_contract`: The address of the contract that is verifying the signature.
pub fn tap_eip712_domain(
    chain_id: u64,
    verifying_contract_address: Address,
    version: TapVersion,
) -> Eip712Domain {
    eip712_domain! {
        name: "TAP",
        version: version.as_str(),
        chain_id: chain_id,
        verifying_contract: verifying_contract_address,
    }
}

#[cfg(test)]
mod tap_tests {
    use std::str::FromStr;

    use rstest::*;
    use tap_graph::{Receipt, ReceiptAggregateVoucher};
    use thegraph_core::alloy::{
        dyn_abi::Eip712Domain, primitives::Address, signers::local::PrivateKeySigner,
    };

    use crate::{signed_message::Eip712SignedMessage, tap_eip712_domain, TapVersion};

    #[fixture]
    fn keys() -> (PrivateKeySigner, Address) {
        let wallet = PrivateKeySigner::random();
        let address = wallet.address();

        (wallet, address)
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
        tap_eip712_domain(1, Address::from([0x11u8; 20]), TapVersion::V1)
    }

    #[rstest]
    #[case::basic_rav_test (vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts (vec![0,0,0,0])]
    #[test]
    fn signed_rav_is_valid_with_no_previous_rav(
        keys: (PrivateKeySigner, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        #[case] values: Vec<u128>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                Eip712SignedMessage::new(
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
        let signed_rav = Eip712SignedMessage::new(&domain_separator, rav, &keys.0).unwrap();
        assert!(signed_rav.recover_signer(&domain_separator).unwrap() == keys.1);
    }

    #[rstest]
    #[case::basic_rav_test(vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts(vec![0,0,0,0])]
    #[test]
    fn signed_rav_is_valid_with_previous_rav(
        keys: (PrivateKeySigner, Address),
        allocation_ids: Vec<Address>,
        domain_separator: Eip712Domain,
        #[case] values: Vec<u128>,
    ) {
        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                Eip712SignedMessage::new(
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
            Eip712SignedMessage::new(&domain_separator, prev_rav, &keys.0).unwrap();

        // Create new RAV from last half of receipts and prev_rav
        let rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[receipts.len() / 2..receipts.len()],
            Some(signed_prev_rav),
        )
        .unwrap();
        let signed_rav = Eip712SignedMessage::new(&domain_separator, rav, &keys.0).unwrap();

        assert!(signed_rav.recover_signer(&domain_separator).unwrap() == keys.1);
    }
}
