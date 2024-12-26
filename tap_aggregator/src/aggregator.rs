// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{hash_set, HashSet};

use alloy::{
    dyn_abi::Eip712Domain, primitives::Address, signers::local::PrivateKeySigner,
    sol_types::SolStruct,
};
use anyhow::{bail, Ok, Result};

use tap_core::{
    rav::ReceiptAggregateVoucher,
    receipt::Receipt,
    signed_message::{EIP712SignedMessage, SignatureBytes, SignatureBytesExt},
};

pub fn check_and_aggregate_receipts(
    domain_separator: &Eip712Domain,
    receipts: &[EIP712SignedMessage<Receipt>],
    previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    wallet: &PrivateKeySigner,
    accepted_addresses: &HashSet<Address>,
) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>> {
    check_signatures_unique(receipts)?;

    // Check that the receipts are signed by an accepted signer address
    receipts.iter().try_for_each(|receipt| {
        check_signature_is_from_one_of_addresses(
            receipt.clone(),
            domain_separator,
            accepted_addresses,
        )
    })?;

    // Check that the previous rav is signed by an accepted signer address
    if let Some(previous_rav) = &previous_rav {
        check_signature_is_from_one_of_addresses(
            previous_rav.clone(),
            domain_separator,
            accepted_addresses,
        )?;
    }

    // Check that the receipts timestamp is greater than the previous rav
    check_receipt_timestamps(receipts, previous_rav.as_ref())?;

    // Get the allocation id from the first receipt, return error if there are no receipts
    let (payer, data_service, service_provider) = match receipts.first() {
        Some(receipt) => (
            receipt.message.payer,
            receipt.message.data_service,
            receipt.message.service_provider,
        ),
        None => return Err(tap_core::Error::NoValidReceiptsForRAVRequest.into()),
    };

    // Check that the receipts all have the same payer
    check_payer(receipts, payer)?;

    // Check that the receipts all have the same service_provider
    check_data_service(receipts, data_service)?;

    // Check that the receipts all have the same service_provider
    check_service_provider(receipts, service_provider)?;

    // Check that the rav has the correct allocation id
    if let Some(previous_rav) = &previous_rav {
        let prev_payer = previous_rav.message.payer;
        if prev_payer != payer {
            return Err(tap_core::Error::RavPayerMismatch {
                prev_id: format!("{prev_payer:#X}"),
                new_id: format!("{payer:#X}"),
            }
            .into());
        }

        let prev_data_service = previous_rav.message.dataService;
        if prev_data_service != data_service {
            return Err(tap_core::Error::RavDataServiceMismatch {
                prev_id: format!("{prev_payer:#X}"),
                new_id: format!("{payer:#X}"),
            }
            .into());
        }

        let prev_service_provider = previous_rav.message.serviceProvider;

        if prev_service_provider != service_provider {
            return Err(tap_core::Error::RavServiceProviderMismatch {
                prev_id: format!("{prev_payer:#X}"),
                new_id: format!("{payer:#X}"),
            }
            .into());
        }
    }

    // Aggregate the receipts
    let rav = ReceiptAggregateVoucher::aggregate_receipts(
        payer,
        data_service,
        service_provider,
        receipts,
        previous_rav,
    )?;

    // Sign the rav and return
    Ok(EIP712SignedMessage::new(domain_separator, rav, wallet)?)
}

fn check_signature_is_from_one_of_addresses<M: SolStruct>(
    message: EIP712SignedMessage<M>,
    domain_separator: &Eip712Domain,
    accepted_addresses: &HashSet<Address>,
) -> Result<()> {
    let recovered_address = message.recover_signer(domain_separator)?;
    if !accepted_addresses.contains(&recovered_address) {
        bail!(tap_core::Error::InvalidRecoveredSigner {
            address: recovered_address,
        });
    }
    Ok(())
}

fn check_payer(receipts: &[EIP712SignedMessage<Receipt>], payer: Address) -> Result<()> {
    for receipt in receipts.iter() {
        let receipt = &receipt.message;
        if receipt.payer != payer {
            return Err(tap_core::Error::RavAllocationIdNotUniform.into());
        }
    }
    Ok(())
}

fn check_data_service(
    receipts: &[EIP712SignedMessage<Receipt>],
    data_service: Address,
) -> Result<()> {
    for receipt in receipts.iter() {
        let receipt = &receipt.message;
        if receipt.data_service != data_service {
            return Err(tap_core::Error::RavAllocationIdNotUniform.into());
        }
    }
    Ok(())
}

fn check_service_provider(
    receipts: &[EIP712SignedMessage<Receipt>],
    service_provider: Address,
) -> Result<()> {
    for receipt in receipts.iter() {
        let receipt = &receipt.message;
        if receipt.service_provider != service_provider {
            return Err(tap_core::Error::RavAllocationIdNotUniform.into());
        }
    }
    Ok(())
}

fn check_signatures_unique(receipts: &[EIP712SignedMessage<Receipt>]) -> Result<()> {
    let mut receipt_signatures: hash_set::HashSet<SignatureBytes> = hash_set::HashSet::new();
    for receipt in receipts.iter() {
        let signature = receipt.signature.get_signature_bytes();
        if !receipt_signatures.insert(signature) {
            return Err(tap_core::Error::DuplicateReceiptSignature(format!(
                "{:?}",
                receipt.signature
            ))
            .into());
        }
    }
    Ok(())
}

fn check_receipt_timestamps(
    receipts: &[EIP712SignedMessage<Receipt>],
    previous_rav: Option<&EIP712SignedMessage<ReceiptAggregateVoucher>>,
) -> Result<()> {
    if let Some(previous_rav) = &previous_rav {
        for receipt in receipts.iter() {
            let receipt = &receipt.message;
            if previous_rav.message.timestampNs >= receipt.timestamp_ns {
                return Err(tap_core::Error::ReceiptTimestampLowerThanRav {
                    rav_ts: previous_rav.message.timestampNs,
                    receipt_ts: receipt.timestamp_ns,
                }
                .into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy::{
        dyn_abi::Eip712Domain,
        primitives::{address, Address, Bytes},
        signers::local::PrivateKeySigner,
    };
    use rstest::*;

    use crate::aggregator;
    use tap_core::{receipt::Receipt, signed_message::EIP712SignedMessage, tap_eip712_domain};

    #[fixture]
    fn keys() -> (PrivateKeySigner, Address) {
        let wallet = PrivateKeySigner::random();
        let address = wallet.address();
        (wallet, address)
    }

    #[fixture]
    fn payer() -> Address {
        address!("abababababababababababababababababababab")
    }

    #[fixture]
    fn data_service() -> Address {
        address!("deaddeaddeaddeaddeaddeaddeaddeaddeaddead")
    }

    #[fixture]
    fn service_provider() -> Address {
        address!("beefbeefbeefbeefbeefbeefbeefbeefbeefbeef")
    }

    #[fixture]
    fn other_address() -> Address {
        address!("1234567890abcdef1234567890abcdef12345678")
    }

    #[fixture]
    fn domain_separator() -> Eip712Domain {
        tap_eip712_domain(1, Address::from([0x11u8; 20]))
    }

    #[rstest]
    #[test]
    fn check_signatures_unique_fail(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        // Create the same receipt twice (replay attack)
        let mut receipts = Vec::new();
        let receipt = EIP712SignedMessage::new(
            &domain_separator,
            Receipt::new(payer, data_service, service_provider, 42).unwrap(),
            &keys.0,
        )
        .unwrap();
        receipts.push(receipt.clone());
        receipts.push(receipt);

        let res = aggregator::check_signatures_unique(&receipts);
        assert!(res.is_err());
    }

    #[rstest]
    #[test]
    fn check_signatures_unique_ok(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        // Create 2 different receipts
        let receipts = vec![
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = aggregator::check_signatures_unique(&receipts);
        assert!(res.is_ok());
    }

    #[rstest]
    #[test]
    /// Test that a receipt with a timestamp greater then the rav timestamp passes
    fn check_receipt_timestamps(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        // Create receipts with consecutive timestamps
        let receipt_timestamp_range = 10..20;
        let mut receipts = Vec::new();
        for i in receipt_timestamp_range.clone() {
            receipts.push(
                EIP712SignedMessage::new(
                    &domain_separator,
                    Receipt {
                        payer,
                        data_service,
                        service_provider,
                        timestamp_ns: i,
                        nonce: 0,
                        value: 42,
                    },
                    &keys.0,
                )
                .unwrap(),
            );
        }

        // Create rav with max_timestamp below the receipts timestamps
        let rav = EIP712SignedMessage::new(
            &domain_separator,
            tap_core::rav::ReceiptAggregateVoucher {
                payer,
                dataService: data_service,
                serviceProvider: service_provider,
                timestampNs: receipt_timestamp_range.clone().min().unwrap() - 1,
                valueAggregate: 42,
                metadata: Bytes::new(),
            },
            &keys.0,
        )
        .unwrap();
        assert!(aggregator::check_receipt_timestamps(&receipts, Some(&rav)).is_ok());

        // Create rav with max_timestamp equal to the lowest receipt timestamp
        // Aggregation should fail
        let rav = EIP712SignedMessage::new(
            &domain_separator,
            tap_core::rav::ReceiptAggregateVoucher {
                payer,
                dataService: data_service,
                serviceProvider: service_provider,
                timestampNs: receipt_timestamp_range.clone().min().unwrap(),
                valueAggregate: 42,
                metadata: Bytes::new(),
            },
            &keys.0,
        )
        .unwrap();
        assert!(aggregator::check_receipt_timestamps(&receipts, Some(&rav)).is_err());

        // Create rav with max_timestamp above highest receipt timestamp
        // Aggregation should fail
        let rav = EIP712SignedMessage::new(
            &domain_separator,
            tap_core::rav::ReceiptAggregateVoucher {
                payer,
                dataService: data_service,
                serviceProvider: service_provider,
                timestampNs: receipt_timestamp_range.clone().max().unwrap() + 1,
                valueAggregate: 42,
                metadata: Bytes::new(),
            },
            &keys.0,
        )
        .unwrap();
        assert!(aggregator::check_receipt_timestamps(&receipts, Some(&rav)).is_err());
    }

    #[rstest]
    #[test]
    /// Test check_service_provider with 2 receipts that have the correct service_provicer
    /// and 1 receipt that has the wrong service_provider
    fn check_service_provider_fail(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        other_address: Address,
        domain_separator: Eip712Domain,
    ) {
        let receipts = vec![
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, other_address, 44).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = aggregator::check_service_provider(&receipts, service_provider);

        assert!(res.is_err());
    }

    #[rstest]
    #[test]
    /// Test check_service_provider with 3 receipts that have the correct service provider
    fn check_service_provider_ok(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        let receipts = vec![
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 44).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = aggregator::check_service_provider(&receipts, service_provider);

        assert!(res.is_ok());
    }

    #[rstest]
    #[test]
    /// Test check_payer with 2 receipts that have the correct payer
    /// and 1 receipt that has the wrong payer
    fn check_payer_fail(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        other_address: Address,
        domain_separator: Eip712Domain,
    ) {
        let receipts = vec![
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(other_address, data_service, service_provider, 44).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = aggregator::check_payer(&receipts, payer);

        assert!(res.is_err());
    }

    #[rstest]
    #[test]
    /// Test check_payer with 3 receipts that have the correct payer
    fn check_payer_ok(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        let receipts = vec![
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 44).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = aggregator::check_payer(&receipts, payer);

        assert!(res.is_ok());
    }

    #[rstest]
    #[test]
    /// Test check_data_service with 2 receipts that have the correct data service
    /// and 1 receipt that has the wrong data service
    fn check_data_service_fail(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        other_address: Address,
        domain_separator: Eip712Domain,
    ) {
        let receipts = vec![
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, other_address, service_provider, 44).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = aggregator::check_data_service(&receipts, data_service);

        assert!(res.is_err());
    }

    #[rstest]
    #[test]
    /// Test check_data_service with 3 receipts that have the correct data service
    fn check_data_service_ok(
        keys: (PrivateKeySigner, Address),
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        let receipts = vec![
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
            EIP712SignedMessage::new(
                &domain_separator,
                Receipt::new(payer, data_service, service_provider, 44).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = aggregator::check_data_service(&receipts, data_service);

        assert!(res.is_ok());
    }
}
