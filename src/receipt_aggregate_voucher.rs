// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{cmp, u64::MAX};

use crate::{receipt::Receipt, Error, Result};
use ethereum_types::Address;
use k256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptAggregateVoucher {
    allocation_id: Address,
    timestamp: u64,
    value_aggregate: u64,
    signature: Signature,
}

impl ReceiptAggregateVoucher {
    pub fn aggregate_receipt(
        receipts: &[Receipt],
        verifying_key: VerifyingKey,
        signing_key: &SigningKey,
        previous_rav: Option<Self>,
    ) -> Result<ReceiptAggregateVoucher> {
        let (timestamp_min, mut value_aggregate, allocation_id) =
            Self::check_rav_and_get_initial_values(verifying_key, previous_rav, receipts)?;
        let mut timestamp_max = timestamp_min;

        for receipt in receipts {
            receipt.is_valid(
                verifying_key,
                &[allocation_id],
                timestamp_min,
                MAX, // TODO: What should the timestamp max be during RAV_REQ? User (gateway) Defined?
                None,
            )?;

            value_aggregate += receipt.get_value();
            timestamp_max = cmp::max(timestamp_max, receipt.get_timestamp_ns())
        }
        Ok(ReceiptAggregateVoucher {
            allocation_id: allocation_id,
            timestamp: timestamp_max,
            value_aggregate: value_aggregate,
            signature: signing_key.sign(&Self::get_message_bytes(
                allocation_id,
                timestamp_max,
                value_aggregate,
            )),
        })
    }

    pub fn is_valid(
        self: &Self,
        verifying_key: VerifyingKey,
        allocation_id: Address,
    ) -> Result<()> {
        if self.allocation_id != allocation_id {
            return Err(Error::InvalidAllocationID {
                received_allocation_id: self.allocation_id,
                expected_allocation_ids: format!("{:?}", allocation_id),
            });
        }
        self.is_valid_signature(verifying_key)
    }

    pub fn is_valid_signature(self: &Self, verifying_key: VerifyingKey) -> Result<()> {
        Ok(verifying_key.verify(
            &Self::get_message_bytes(self.allocation_id, self.timestamp, self.value_aggregate),
            &self.signature,
        )?)
    }

    fn check_rav_and_get_initial_values(
        verifying_key: VerifyingKey,
        previous_rav: Option<Self>,
        receipts: &[Receipt],
    ) -> Result<(u64, u64, Address)> {
        // All allocation IDs need to match, so initial allocation ID is set to ID
        // from an arbitry given receipt. This must then be compared against all
        // other receipts/RAV allocation IDs.
        let allocation_id = receipts[0].get_allocation_id();

        if let Some(prev_rav) = previous_rav {
            prev_rav.is_valid_for_rav_request(verifying_key, allocation_id)?;

            // Add one to timestamp because only timestamps *AFTER* previous RAV timestamps are valid
            let timestamp = prev_rav.timestamp + 1;

            return Ok((timestamp, prev_rav.value_aggregate, allocation_id));
        }
        // If no RAV is provided then timestamp and value aggregate can be set to zero
        return Ok((0u64, 0u64, receipts[0].get_allocation_id()));
    }

    /// Checks is a RAV received in a new RAV request is valid. This is different from
    /// a full is_valid check because the RAV has no expected values except allocation ID,
    /// if that is valid and the signature is correct then all other values can be used.
    fn is_valid_for_rav_request(
        self: &Self,
        verifying_key: VerifyingKey,
        allocation_id: Address,
    ) -> Result<()> {
        if self.allocation_id != allocation_id {
            return Err(Error::InvalidAllocationID {
                received_allocation_id: self.allocation_id,
                expected_allocation_ids: format!("{:?}", allocation_id),
            });
        }
        self.is_valid_signature(verifying_key)
    }

    fn get_message_bytes(allocation_id: Address, timestamp: u64, value: u64) -> Vec<u8> {
        allocation_id
            .as_bytes()
            .iter()
            .copied()
            .chain(timestamp.to_be_bytes())
            .chain(value.to_be_bytes())
            .collect()
    }
}
