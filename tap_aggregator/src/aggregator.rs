// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::hash_set;

use anyhow::{Ok, Result};
use ethers_core::types::{Address, Signature};
use ethers_signers::{LocalWallet, Signer};

use tap_core::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};

pub async fn check_and_aggregate_receipts(
    receipts: &[EIP712SignedMessage<Receipt>],
    previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    wallet: &LocalWallet,
) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>> {
    // Check that the receipts are unique
    check_signatures_unique(receipts)?;

    // Check that the receipts are signed by ourselves
    receipts
        .iter()
        .try_for_each(|receipt| receipt.verify(wallet.address()))?;

    // Check that the previous rav is signed by ourselves
    if let Some(previous_rav) = &previous_rav {
        previous_rav.verify(wallet.address())?;
    }

    // Check that the receipts timestamp is greater than the previous rav
    check_receipt_timestamps(receipts, previous_rav.as_ref())?;

    // Get the allocation id from the first receipt, return error if there are no receipts
    let allocation_id = match receipts.get(0) {
        Some(receipt) => receipt.message.allocation_id,
        None => return Err(tap_core::Error::NoValidReceiptsForRAVRequest.into()),
    };

    // Check that the receipts all have the same allocation id
    check_allocation_id(receipts, allocation_id)?;

    // Check that the rav has the correct allocation id
    if let Some(previous_rav) = &previous_rav {
        let prev_id = previous_rav.message.allocation_id;
        if prev_id != allocation_id {
            return Err(tap_core::Error::RavAllocationIdMismatch {
                prev_id: format!("{prev_id:#X}"),
                new_id: format!("{allocation_id:#X}"),
            }
            .into());
        }
    }

    // Aggregate the receipts
    let rav = ReceiptAggregateVoucher::aggregate_receipts(allocation_id, receipts, previous_rav)?;

    // Sign the rav and return
    Ok(EIP712SignedMessage::new(rav, wallet).await?)
}

fn check_allocation_id(
    receipts: &[EIP712SignedMessage<Receipt>],
    allocation_id: Address,
) -> Result<()> {
    for receipt in receipts.iter() {
        let receipt = &receipt.message;
        if receipt.allocation_id != allocation_id {
            return Err(tap_core::Error::RavAllocationIdNotUniform.into());
        }
    }
    Ok(())
}

fn check_signatures_unique(receipts: &[EIP712SignedMessage<Receipt>]) -> Result<()> {
    let mut receipt_signatures: hash_set::HashSet<Signature> = hash_set::HashSet::new();
    for receipt in receipts.iter() {
        let signature = receipt.signature;
        if !receipt_signatures.insert(signature) {
            return Err(tap_core::Error::DuplicateReceiptSignature(signature.to_string()).into());
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
            if previous_rav.message.timestamp_ns >= receipt.timestamp_ns {
                return Err(tap_core::Error::ReceiptTimestampLowerThanRav {
                    rav_ts: previous_rav.message.timestamp_ns,
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
    use std::str::FromStr;

    use ethers_core::types::Address;
    use ethers_signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;

    use crate::aggregator;
    use tap_core::{eip_712_signed_message::EIP712SignedMessage, tap_receipt::Receipt};

    #[fixture]
    fn keys() -> (LocalWallet, Address) {
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
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

    #[rstest]
    #[tokio::test]
    async fn check_signatures_unique_fail(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
    ) {
        // Create the same receipt twice (replay attack)
        let mut receipts = Vec::new();
        let receipt =
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .await
                .unwrap();
        receipts.push(receipt.clone());
        receipts.push(receipt);

        let res = aggregator::check_signatures_unique(&receipts);
        assert!(res.is_err());
    }

    #[rstest]
    #[tokio::test]
    async fn check_signatures_unique_ok(
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
    ) {
        // Create 2 different receipts
        let mut receipts = Vec::new();
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .await
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 43).unwrap(), &keys.0)
                .await
                .unwrap(),
        );

        let res = aggregator::check_signatures_unique(&receipts);
        assert!(res.is_ok());
    }

    #[rstest]
    #[tokio::test]
    /// Test that a receipt with a timestamp greater then the rav timestamp passes
    async fn check_receipt_timestamps(keys: (LocalWallet, Address), allocation_ids: Vec<Address>) {
        // Create receipts with consecutive timestamps
        let receipt_timestamp_range = 10..20;
        let mut receipts = Vec::new();
        for i in receipt_timestamp_range.clone() {
            receipts.push(
                EIP712SignedMessage::new(
                    Receipt {
                        allocation_id: allocation_ids[0],
                        timestamp_ns: i,
                        nonce: 0,
                        value: 42,
                    },
                    &keys.0,
                )
                .await
                .unwrap(),
            );
        }

        // Create rav with max_timestamp below the receipts timestamps
        let rav = EIP712SignedMessage::new(
            tap_core::receipt_aggregate_voucher::ReceiptAggregateVoucher {
                allocation_id: allocation_ids[0],
                timestamp_ns: receipt_timestamp_range.clone().min().unwrap() - 1,
                value_aggregate: 42,
            },
            &keys.0,
        )
        .await
        .unwrap();
        assert!(aggregator::check_receipt_timestamps(&receipts, Some(&rav)).is_ok());

        // Create rav with max_timestamp equal to the lowest receipt timestamp
        // Aggregation should fail
        let rav = EIP712SignedMessage::new(
            tap_core::receipt_aggregate_voucher::ReceiptAggregateVoucher {
                allocation_id: allocation_ids[0],
                timestamp_ns: receipt_timestamp_range.clone().min().unwrap(),
                value_aggregate: 42,
            },
            &keys.0,
        )
        .await
        .unwrap();
        assert!(aggregator::check_receipt_timestamps(&receipts, Some(&rav)).is_err());

        // Create rav with max_timestamp above highest receipt timestamp
        // Aggregation should fail
        let rav = EIP712SignedMessage::new(
            tap_core::receipt_aggregate_voucher::ReceiptAggregateVoucher {
                allocation_id: allocation_ids[0],
                timestamp_ns: receipt_timestamp_range.clone().max().unwrap() + 1,
                value_aggregate: 42,
            },
            &keys.0,
        )
        .await
        .unwrap();
        assert!(aggregator::check_receipt_timestamps(&receipts, Some(&rav)).is_err());
    }

    #[rstest]
    #[tokio::test]
    /// Test check_allocation_id with 2 receipts that have the correct allocation id
    /// and 1 receipt that has the wrong allocation id
    async fn check_allocation_id_fail(keys: (LocalWallet, Address), allocation_ids: Vec<Address>) {
        let mut receipts = Vec::new();
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .await
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 43).unwrap(), &keys.0)
                .await
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[1], 44).unwrap(), &keys.0)
                .await
                .unwrap(),
        );

        let res = aggregator::check_allocation_id(&receipts, allocation_ids[0]);

        assert!(res.is_err());
    }

    #[rstest]
    #[tokio::test]
    /// Test check_allocation_id with 3 receipts that have the correct allocation id
    async fn check_allocation_id_ok(keys: (LocalWallet, Address), allocation_ids: Vec<Address>) {
        let mut receipts = Vec::new();
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .await
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 43).unwrap(), &keys.0)
                .await
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 44).unwrap(), &keys.0)
                .await
                .unwrap(),
        );

        let res = aggregator::check_allocation_id(&receipts, allocation_ids[0]);

        assert!(res.is_ok());
    }
}
