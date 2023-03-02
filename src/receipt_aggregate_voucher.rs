// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt Aggregation Voucher (RAV) type used as a signed aggregate of receipt batches
//!
//! Receipt Aggregate Vouchers are used to aggregate batches of ordered receipts
//! into a single signed voucher. In the [`TAP`](crate) protocol a RAV can be requested
//! at any time by providing the batch of receipts to be aggragated and optionally the
//! most recent previous RAV received.

use std::{cmp, u64::MAX};

use crate::{receipt::Receipt, Error, Result};
use ethereum_types::Address;
use k256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptAggregateVoucher {
    /// Unique allocation id this RAV belongs to
    pub allocation_id: Address,
    /// Unix Epoch timestamp in nanoseconds (Truncated to 64-bits)
    /// corresponding to max timestamp from receipt batch aggregated
    pub timestamp: u64,
    /// Aggregated value from receipt batch and any previous RAV provided
    pub value_aggregate: u64,
    /// ECDSA Signature of all other values in RAV
    pub signature: Signature,
}

impl ReceiptAggregateVoucher {
    /// Aggregates a batch of receipts with optional previous RAV, returning a new signed RAV if all provided items are valid or an error if not.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidAllocationID`] if any receipt has a allocation ID that does match other `receipts` or `previous_rav`
    ///
    /// Returns [`Error::InvalidTimestamp`] if any receipt has a timestamp that is not *greater than* timestamp on `previous_rav` (if no `previous_rav` is provided there is no timestamp requirement)
    ///
    /// Returns [`Error::InvalidSignature`] if the signature on `previous_rav` or one or more `receipts` is not valid with provided `verifying_key`
    ///
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

            value_aggregate = value_aggregate
                .checked_add(receipt.value)
                .ok_or(Error::AggregateOverflow)?;

            timestamp_max = cmp::max(timestamp_max, receipt.timestamp_ns)
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

    /// Verifies RAV has matching allocation ID and signature is valid with `verifying_key`
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidAllocationID`] if the allocation ID on the RAV does not match `allocation_id`
    ///
    /// Returns [`Error::InvalidSignature`] if the signature is not valid with provided `verifying_key`
    ///
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

    /// Checks that RAV signature is valid for given verifying key, returns `Ok` if it is valid.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSignature`] if the signature is not valid with provided `verifying_key`
    ///
    pub fn is_valid_signature(self: &Self, verifying_key: VerifyingKey) -> Result<()> {
        Ok(verifying_key.verify(
            &Self::get_message_bytes(self.allocation_id, self.timestamp, self.value_aggregate),
            &self.signature,
        )?)
    }

    /// If a previous RAV is provided verify it and get the correct
    /// corresponding initial values otherwise get the default initial
    /// values.
    fn check_rav_and_get_initial_values(
        verifying_key: VerifyingKey,
        previous_rav: Option<Self>,
        receipts: &[Receipt],
    ) -> Result<(u64, u64, Address)> {
        // All allocation IDs need to match, so initial allocation ID is set to ID
        // from an arbitry given receipt. This must then be compared against all
        // other receipts/RAV allocation IDs.
        let allocation_id = receipts[0].allocation_id;

        if let Some(prev_rav) = previous_rav {
            prev_rav.is_valid_for_rav_request(verifying_key, allocation_id)?;

            // Add one to timestamp because only timestamps *AFTER* previous RAV timestamps are valid
            let timestamp = prev_rav.timestamp + 1;

            return Ok((timestamp, prev_rav.value_aggregate, allocation_id));
        }
        // If no RAV is provided then timestamp and value aggregate can be set to zero
        return Ok((0u64, 0u64, receipts[0].allocation_id));
    }

    /// Checks if a RAV received in a new RAV request is valid. This is different from
    /// a full is_valid check because the RAV has no expected values except allocation ID.
    /// If that is valid and the signature is correct then all other values can be used.
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

    /// Creates a byte vector of the receipt aggregate vouchers message for signing
    ///
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
