use thiserror::Error;
use ethereum_types::Address;

pub mod receipt;
pub mod receipt_aggregate_voucher;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid allocation ID on RAV: {received_allocation_id} (expected {expected_allocation_id})")]
    InvalidAllocationID{received_allocation_id: Address, expected_allocation_id: Address},
    #[error("invalid signature on Receipt Aggregate Voucher")]
    InvalidSignature(#[from] k256::ecdsa::Error),
    #[error("invalid timestamp: {received_timestamp} (expected > {min_timestamp})")]
    InvalidTimestamp{received_timestamp: u64, min_timestamp: u64}
}
type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::{SystemTime, UNIX_EPOCH}};
    use ethereum_types::Address;


    use k256::{
        ecdsa::{SigningKey, VerifyingKey},
    };
    use rand::{Rng, thread_rng};
    use rand_core::OsRng;

    use crate::{receipt::Receipt, receipt_aggregate_voucher::ReceiptAggregateVoucher, Error};

    #[test]
    fn test_receipt() {
        let signing_key = SigningKey::random(&mut OsRng);

        let allocation_id = Address::from_str("0xabababababababababababababababababababab").unwrap();
        let value = 10u64;
        let timestamp = u64::try_from(
            SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
        ).unwrap();
        let nonce = thread_rng().gen::<u64>();
        let test_receipt = Receipt::new(allocation_id, timestamp+10, nonce, value, &signing_key);

        println!(
            "time in ms: {}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        assert!(test_receipt.is_valid(VerifyingKey::from(signing_key), allocation_id, timestamp).is_ok())
    }

    #[test]
    fn test_rav() {
        let values = (0u64..2^20).collect::<Vec<_>>();
        let mut receipts = Vec::new();
        let allocation_id = Address::from_str("0xabababababababababababababababababababab").unwrap();
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        for value in values {
            let nonce = thread_rng().gen::<u64>();

            receipts.push(
                Receipt::new(
                    allocation_id,
                    value,
                    nonce,
                    value,
                    &signing_key
                )
            )
        }

        let rav =
            ReceiptAggregateVoucher::aggregate_receipt(
                &receipts,
                verifying_key,
                &signing_key,
                allocation_id,
                None
            ).unwrap();
        assert!(rav.is_valid(verifying_key, allocation_id).is_ok());
    }

    #[test]
    fn test_incorrect_allocation(){
        let values = (0u64..2^20).collect::<Vec<_>>();
        let mut receipts = Vec::new();
        let allocation_id = Address::from_str("0xabababababababababababababababababababab").unwrap();
        let false_allocation_id = Address::from_str("0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd").unwrap();
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        for value in values {
            let nonce = thread_rng().gen::<u64>();
            receipts.push(
                Receipt::new(
                    allocation_id,
                    value,
                    nonce,
                    value,
                    &signing_key,
                )
            )
        }

        let false_rav =
            ReceiptAggregateVoucher::aggregate_receipt(
                &receipts,
                verifying_key,
                &signing_key,
                false_allocation_id,
                None
            ).unwrap_err();
        assert!(matches!(false_rav, Error::InvalidAllocationID {..}));
    }
}
