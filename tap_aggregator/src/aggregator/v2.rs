// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use anyhow::{bail, Ok, Result};
use rayon::prelude::*;
use tap_core::{receipt::WithUniqueId, signed_message::Eip712SignedMessage};
use tap_graph::v2::{Receipt, ReceiptAggregateVoucher};
use thegraph_core::alloy::{
    dyn_abi::Eip712Domain,
    primitives::{Address, FixedBytes},
    signers::local::PrivateKeySigner,
    sol_types::SolStruct,
};

pub fn check_and_aggregate_receipts(
    domain_separator: &Eip712Domain,
    receipts: &[Eip712SignedMessage<Receipt>],
    previous_rav: Option<Eip712SignedMessage<ReceiptAggregateVoucher>>,
    wallet: &PrivateKeySigner,
    accepted_addresses: &HashSet<Address>,
) -> Result<Eip712SignedMessage<ReceiptAggregateVoucher>> {
    check_signatures_unique(receipts)?;

    // Check that the receipts are signed by an accepted signer address
    receipts.par_iter().try_for_each(|receipt| {
        check_signature_is_from_one_of_addresses(receipt, domain_separator, accepted_addresses)
    })?;

    // Check that the previous rav is signed by an accepted signer address
    if let Some(previous_rav) = &previous_rav {
        check_signature_is_from_one_of_addresses(
            previous_rav,
            domain_separator,
            accepted_addresses,
        )?;
    }

    // Check that the receipts timestamp is greater than the previous rav
    check_receipt_timestamps(receipts, previous_rav.as_ref())?;

    // Get the allocation id from the first receipt, return error if there are no receipts
    let (collection_id, payer, data_service, service_provider) = match receipts.first() {
        Some(receipt) => (
            receipt.message.collection_id,
            receipt.message.payer,
            receipt.message.data_service,
            receipt.message.service_provider,
        ),
        None => return Err(tap_core::Error::NoValidReceiptsForRavRequest.into()),
    };

    // Check that the receipts all have the same collection id
    check_collection_id(
        receipts,
        collection_id,
        payer,
        data_service,
        service_provider,
    )?;

    // Check that the rav has the correct allocation id
    if let Some(previous_rav) = &previous_rav {
        let prev_id = previous_rav.message.collectionId;
        let prev_payer = previous_rav.message.payer;
        let prev_data_service = previous_rav.message.dataService;
        let prev_service_provider = previous_rav.message.serviceProvider;
        if prev_id != collection_id {
            return Err(tap_core::Error::RavAllocationIdMismatch {
                prev_id: format!("{prev_id:#X}"),
                new_id: format!("{collection_id:#X}"),
            }
            .into());
        }
        if prev_payer != payer {
            return Err(tap_core::Error::RavAllocationIdMismatch {
                prev_id: format!("{prev_id:#X}"),
                new_id: format!("{collection_id:#X}"),
            }
            .into());
        }

        if prev_data_service != data_service {
            return Err(tap_core::Error::RavAllocationIdMismatch {
                prev_id: format!("{prev_id:#X}"),
                new_id: format!("{collection_id:#X}"),
            }
            .into());
        }
        if prev_service_provider != service_provider {
            return Err(tap_core::Error::RavAllocationIdMismatch {
                prev_id: format!("{prev_id:#X}"),
                new_id: format!("{collection_id:#X}"),
            }
            .into());
        }
    }

    // Aggregate the receipts
    let rav = ReceiptAggregateVoucher::aggregate_receipts(
        collection_id,
        payer,
        data_service,
        service_provider,
        receipts,
        previous_rav,
    )?;

    // Sign the rav and return
    Ok(Eip712SignedMessage::new(domain_separator, rav, wallet)?)
}

fn check_signature_is_from_one_of_addresses<M: SolStruct>(
    message: &Eip712SignedMessage<M>,
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

fn check_collection_id(
    receipts: &[Eip712SignedMessage<Receipt>],
    collection_id: FixedBytes<32>,
    payer: Address,
    data_service: Address,
    service_provider: Address,
) -> Result<()> {
    for receipt in receipts.iter() {
        let receipt = &receipt.message;
        if receipt.collection_id != collection_id {
            return Err(tap_core::Error::RavAllocationIdNotUniform.into());
        }
        if receipt.payer != payer {
            return Err(tap_core::Error::RavAllocationIdNotUniform.into());
        }
        if receipt.data_service != data_service {
            return Err(tap_core::Error::RavAllocationIdNotUniform.into());
        }
        if receipt.service_provider != service_provider {
            return Err(tap_core::Error::RavAllocationIdNotUniform.into());
        }
    }
    Ok(())
}

fn check_signatures_unique(receipts: &[Eip712SignedMessage<Receipt>]) -> Result<()> {
    let mut receipt_signatures = HashSet::new();
    for receipt in receipts.iter() {
        let signature = receipt.unique_id();
        if !receipt_signatures.insert(signature) {
            return Err(tap_core::Error::DuplicateReceiptSignature(format!(
                "{:?}",
                receipt.unique_id()
            ))
            .into());
        }
    }
    Ok(())
}

fn check_receipt_timestamps(
    receipts: &[Eip712SignedMessage<Receipt>],
    previous_rav: Option<&Eip712SignedMessage<ReceiptAggregateVoucher>>,
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
    use rstest::*;
    use tap_core::{signed_message::Eip712SignedMessage, tap_eip712_domain, TapVersion};
    use tap_graph::v2::{Receipt, ReceiptAggregateVoucher};
    use thegraph_core::alloy::{
        dyn_abi::Eip712Domain,
        primitives::{address, fixed_bytes, Address, Bytes, FixedBytes},
        signers::local::PrivateKeySigner,
    };

    #[fixture]
    fn keys() -> (PrivateKeySigner, Address) {
        let wallet = PrivateKeySigner::random();
        let address = wallet.address();
        (wallet, address)
    }

    #[fixture]
    fn collection_id() -> FixedBytes<32> {
        fixed_bytes!("deaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddeaddead")
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
    fn other_collection_id() -> FixedBytes<32> {
        fixed_bytes!("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
    }
    #[fixture]
    fn domain_separator() -> Eip712Domain {
        tap_eip712_domain(1, Address::from([0x11u8; 20]), TapVersion::V2)
    }

    #[rstest]
    #[test]
    fn check_signatures_unique_fail(
        keys: (PrivateKeySigner, Address),
        collection_id: FixedBytes<32>,
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        // Create the same receipt twice (replay attack)
        let mut receipts = Vec::new();
        let receipt = Eip712SignedMessage::new(
            &domain_separator,
            Receipt::new(collection_id, payer, data_service, service_provider, 42).unwrap(),
            &keys.0,
        )
        .unwrap();
        receipts.push(receipt.clone());
        receipts.push(receipt);

        let res = super::check_signatures_unique(&receipts);
        assert!(res.is_err());
    }

    #[rstest]
    #[test]
    fn check_signatures_unique_ok(
        keys: (PrivateKeySigner, Address),
        collection_id: FixedBytes<32>,
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        // Create 2 different receipts
        let receipts = vec![
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(collection_id, payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(collection_id, payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = super::check_signatures_unique(&receipts);
        assert!(res.is_ok());
    }

    #[rstest]
    #[test]
    /// Test that a receipt with a timestamp greater than the rav timestamp passes
    fn check_receipt_timestamps(
        keys: (PrivateKeySigner, Address),
        collection_id: FixedBytes<32>,
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
                Eip712SignedMessage::new(
                    &domain_separator,
                    Receipt {
                        collection_id,
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
        let rav = Eip712SignedMessage::new(
            &domain_separator,
            ReceiptAggregateVoucher {
                collectionId: collection_id,
                dataService: data_service,
                payer,
                serviceProvider: service_provider,
                timestampNs: receipt_timestamp_range.clone().min().unwrap() - 1,
                valueAggregate: 42,
                metadata: Bytes::new(),
            },
            &keys.0,
        )
        .unwrap();
        assert!(super::check_receipt_timestamps(&receipts, Some(&rav)).is_ok());

        // Create rav with max_timestamp equal to the lowest receipt timestamp
        // Aggregation should fail
        let rav = Eip712SignedMessage::new(
            &domain_separator,
            ReceiptAggregateVoucher {
                collectionId: collection_id,
                dataService: data_service,
                payer,
                serviceProvider: service_provider,
                timestampNs: receipt_timestamp_range.clone().min().unwrap(),
                valueAggregate: 42,
                metadata: Bytes::new(),
            },
            &keys.0,
        )
        .unwrap();
        assert!(super::check_receipt_timestamps(&receipts, Some(&rav)).is_err());

        // Create rav with max_timestamp above highest receipt timestamp
        // Aggregation should fail
        let rav = Eip712SignedMessage::new(
            &domain_separator,
            ReceiptAggregateVoucher {
                collectionId: collection_id,
                dataService: data_service,
                payer,
                serviceProvider: service_provider,
                timestampNs: receipt_timestamp_range.clone().max().unwrap() + 1,
                valueAggregate: 42,
                metadata: Bytes::new(),
            },
            &keys.0,
        )
        .unwrap();
        assert!(super::check_receipt_timestamps(&receipts, Some(&rav)).is_err());
    }

    #[rstest]
    #[test]
    /// Test check_allocation_id with 2 receipts that have the correct allocation id
    /// and 1 receipt that has the wrong allocation id
    fn check_allocation_id_fail(
        keys: (PrivateKeySigner, Address),
        collection_id: FixedBytes<32>,
        payer: Address,
        data_service: Address,
        service_provider: Address,
        other_collection_id: FixedBytes<32>,
        domain_separator: Eip712Domain,
    ) {
        let receipts = vec![
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(collection_id, payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(collection_id, payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(
                    other_collection_id,
                    payer,
                    data_service,
                    service_provider,
                    44,
                )
                .unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = super::check_collection_id(
            &receipts,
            collection_id,
            payer,
            data_service,
            service_provider,
        );

        assert!(res.is_err());
    }

    #[rstest]
    #[test]
    /// Test check_allocation_id with 3 receipts that have the correct allocation id
    fn check_allocation_id_ok(
        keys: (PrivateKeySigner, Address),
        collection_id: FixedBytes<32>,
        payer: Address,
        data_service: Address,
        service_provider: Address,
        domain_separator: Eip712Domain,
    ) {
        let receipts = vec![
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(collection_id, payer, data_service, service_provider, 42).unwrap(),
                &keys.0,
            )
            .unwrap(),
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(collection_id, payer, data_service, service_provider, 43).unwrap(),
                &keys.0,
            )
            .unwrap(),
            Eip712SignedMessage::new(
                &domain_separator,
                Receipt::new(collection_id, payer, data_service, service_provider, 44).unwrap(),
                &keys.0,
            )
            .unwrap(),
        ];

        let res = super::check_collection_id(
            &receipts,
            collection_id,
            payer,
            data_service,
            service_provider,
        );

        assert!(res.is_ok());
    }
}
