use std::cmp;

use crate::{receipt::Receipt, Result, Error};
use ethereum_types::Address;
use k256::{
    ecdsa::{SigningKey, Signature, signature::Signer, VerifyingKey, signature::Verifier}
};

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptAggregateVoucher{
    allocation_id: Address,
    timestamp: u64,
    value_aggregate: u64,
    signature: Signature
}

impl ReceiptAggregateVoucher{
    pub fn aggregate_receipt(
        receipts: &[Receipt],
        verifying_key: VerifyingKey,
        signing_key: &SigningKey,
        allocation_id: Address,
        previous_rav: Option<Self>
    ) -> Result<ReceiptAggregateVoucher>{
        let (min_timestamp, mut max_timestamp, mut value_aggregate) =
            Self::get_previous_rav_values_or_defaults(verifying_key, allocation_id, previous_rav)?;

        for receipt in receipts{
            receipt.is_valid(verifying_key, allocation_id, min_timestamp)?;

            value_aggregate += receipt.get_value();
            max_timestamp = cmp::max(max_timestamp, receipt.get_timestamp())
        }
        Ok(
            ReceiptAggregateVoucher{
                allocation_id: allocation_id,
                timestamp: max_timestamp,
                value_aggregate: value_aggregate,
                signature: signing_key.sign(&Self::get_message_bytes(allocation_id, max_timestamp, value_aggregate))
            }
        )
    }

    pub fn is_valid(self: &Self, verifying_key: VerifyingKey, allocation_id: Address) -> Result<()>{
        if self.allocation_id != allocation_id {
            return Err(Error::InvalidAllocationID { received_allocation_id: self.allocation_id, expected_allocation_id: allocation_id});
        }
        Ok(verifying_key
        .verify(
            &Self::get_message_bytes(
                self.allocation_id,
                self.timestamp,
                self.value_aggregate
            ),
            &self.signature
        )?)
    }

    fn get_previous_rav_values_or_defaults(
        verifying_key: VerifyingKey,
        allocation_id: Address,
        previous_rav: Option<Self>
    )
    ->Result<(u64, u64, u64)>{
        if let Some(prev_rav) = previous_rav {
            prev_rav.is_valid(verifying_key, allocation_id)?;
            return Ok((
                    prev_rav.timestamp,
                    prev_rav.timestamp,
                    prev_rav.value_aggregate
                )
            )
        }
        return Ok((0u64, 0u64, 0u64))
    }

    fn get_message_bytes(allocation_id: Address, timestamp: u64, value: u64) -> Vec<u8>{
        allocation_id.as_bytes().iter().copied()
            .chain(timestamp.to_be_bytes())
            .chain(value.to_be_bytes())
            .collect()
    }
}