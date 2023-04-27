use anyhow::{Ok, Result};
use ethereum_types::Address;
use k256::ecdsa::{SigningKey, VerifyingKey};
use std::collections::hash_set;
use tap_core::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};

pub fn check_and_aggregate_receipts(
    receipts: &[EIP712SignedMessage<Receipt>],
    previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>> {
    // Check that the receipts are unique
    check_signatures_unique(receipts)?;

    // Check that the receipts are signed by ourselves
    for receipt in receipts.iter() {
        receipt.check_signature(verifying_key)?;
    }

    // Check that the previous rav is signed by ourselves
    if let Some(previous_rav) = previous_rav.clone() {
        previous_rav.check_signature(verifying_key)?;
    }

    // Check that the receipts timestamp is greater then the previous rav
    check_receipt_timestamps(receipts, previous_rav.clone())?;

    // Get the allocation id from the first receipt, return error if there are no receipts
    let allocation_id = match receipts.get(0) {
        Some(receipt) => receipt.message.allocation_id,
        None => {
            return Err(tap_core::Error::InvalidCheckError {
                check_string: "No receipts".into(),
            }
            .into())
        }
    };

    // Check that the receipts all have the same allocation id
    check_allocation_id(receipts, allocation_id)?;

    // Check that the rav has the correct allocation id
    if let Some(previous_rav) = previous_rav.clone() {
        let previous_rav = previous_rav.message;
        if previous_rav.allocation_id != allocation_id {
            return Err(tap_core::Error::InvalidCheckError {
                check_string: "Previous rav allocation id does not match receipts".into(),
            }
            .into());
        }
    }

    // Aggregate the receipts
    let rav = ReceiptAggregateVoucher::aggregate_receipts(allocation_id, receipts, previous_rav)?;

    // Sign the rav and return
    Ok(EIP712SignedMessage::new(rav, &signing_key)?)
}

fn check_allocation_id(
    receipts: &[EIP712SignedMessage<Receipt>],
    allocation_id: Address,
) -> Result<()> {
    for receipt in receipts.iter() {
        let receipt = &receipt.message;
        if receipt.allocation_id != allocation_id {
            return Err(tap_core::Error::InvalidCheckError {
                check_string: "Receipts allocation id is not uniform".into(),
            }
            .into());
        }
    }
    Ok(())
}

fn check_signatures_unique(receipts: &[EIP712SignedMessage<Receipt>]) -> Result<()> {
    let mut receipt_signatures: hash_set::HashSet<[u8; 64]> = hash_set::HashSet::new();
    for receipt in receipts.iter() {
        let signature = receipt.signature.to_bytes();
        if receipt_signatures.contains(signature.as_slice()) {
            return Err(tap_core::Error::InvalidCheckError {
                check_string: "Duplicate receipt signature".into(),
            }
            .into());
        }
        receipt_signatures.insert(signature.into());
    }
    Ok(())
}

fn check_receipt_timestamps(
    receipts: &[EIP712SignedMessage<Receipt>],
    previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
) -> Result<()> {
    if let Some(previous_rav) = previous_rav {
        let previous_rav = previous_rav.message;
        for receipt in receipts.iter() {
            let receipt = &receipt.message;
            if previous_rav.timestamp > receipt.timestamp_ns {
                return Err(tap_core::Error::InvalidCheckError {
                    check_string: "Receipt timestamp is less or equal then previous rav timestamp"
                        .into(),
                }
                .into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::aggregator;
    use ethereum_types::Address;
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rand_core::OsRng;
    use rstest::*;
    use std::time::UNIX_EPOCH;
    use std::{str::FromStr, time::SystemTime};
    use tap_core::{eip_712_signed_message::EIP712SignedMessage, tap_receipt::Receipt};

    #[fixture]
    fn keys() -> (SigningKey, VerifyingKey) {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        (signing_key, verifying_key)
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
    fn check_signatures_unique_fail(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
    ) {
        // Create the same receipt twice (replay attack)
        let mut receipts = Vec::new();
        let receipt =
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .unwrap();
        receipts.push(receipt.clone());
        receipts.push(receipt.clone());

        let res = aggregator::check_signatures_unique(&receipts);
        assert!(res.is_err());
    }

    #[rstest]
    fn check_signatures_unique_ok(keys: (SigningKey, VerifyingKey), allocation_ids: Vec<Address>) {
        // Create 2 different receipts
        let mut receipts = Vec::new();
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 43).unwrap(), &keys.0)
                .unwrap(),
        );

        let res = aggregator::check_signatures_unique(&receipts);
        assert!(res.is_ok());
    }

    #[rstest]
    /// Test that a receipt with a timestamp greater then the rav timestamp passes
    fn check_receipt_timestamps_ok(keys: (SigningKey, VerifyingKey), allocation_ids: Vec<Address>) {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Create rav
        let rav = EIP712SignedMessage::new(
            tap_core::receipt_aggregate_voucher::ReceiptAggregateVoucher {
                allocation_id: allocation_ids[0],
                timestamp: time,
                value_aggregate: 42,
            },
            &keys.0,
        )
        .unwrap();

        let mut receipts = Vec::new();
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .unwrap(),
        );

        aggregator::check_receipt_timestamps(&receipts, Some(rav)).unwrap();
    }

    #[rstest]
    /// Test that a receipt with a timestamp less then the rav timestamp fails
    fn check_receipt_timestamps_fail(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
    ) {
        let mut receipts = Vec::new();
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .unwrap(),
        );

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Create rav
        let rav = EIP712SignedMessage::new(
            tap_core::receipt_aggregate_voucher::ReceiptAggregateVoucher {
                allocation_id: allocation_ids[0],
                timestamp: time,
                value_aggregate: 42,
            },
            &keys.0,
        )
        .unwrap();

        let res = aggregator::check_receipt_timestamps(&receipts, Some(rav));

        assert!(res.is_err());
    }

    #[rstest]
    /// Test check_allocation_id with 2 receipts that have the correct allocation id
    /// and 1 receipt that has the wrong allocation id
    fn check_allocation_id_fail(keys: (SigningKey, VerifyingKey), allocation_ids: Vec<Address>) {
        let mut receipts = Vec::new();
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 43).unwrap(), &keys.0)
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[1], 44).unwrap(), &keys.0)
                .unwrap(),
        );

        let res = aggregator::check_allocation_id(&receipts, allocation_ids[0]);

        assert!(res.is_err());
    }

    #[rstest]
    /// Test check_allocation_id with 3 receipts that have the correct allocation id
    fn check_allocation_id_ok(keys: (SigningKey, VerifyingKey), allocation_ids: Vec<Address>) {
        let mut receipts = Vec::new();
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 43).unwrap(), &keys.0)
                .unwrap(),
        );
        receipts.push(
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 44).unwrap(), &keys.0)
                .unwrap(),
        );

        let res = aggregator::check_allocation_id(&receipts, allocation_ids[0]);

        assert!(res.is_ok());
    }
}
