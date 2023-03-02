// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt type used for providing and verifying a payment
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.

use ethereum_types::Address;
use k256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};

use crate::{Error, Result};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
/// Holds information needed for promise of payment signed with ECDSA
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Receipt {
    /// Unique allocation id this receipt belongs to
    pub allocation_id: Address,
    /// Unix Epoch timestamp in nanoseconds (Truncated to 64-bits)
    pub timestamp_ns: u64,
    /// Random value used to avoid collisions from multiple receipts with one timestamp
    pub nonce: u64,
    /// Payment value for transaction
    pub value: u64,
    /// ECDSA Signature of all other values in receipt
    pub signature: Signature,
}

impl Receipt {
    /// Returns a receipt with provided values signed with `signing_key`
    pub fn new(
        allocation_id: Address,
        timestamp_ns: u64,
        value: u64,
        signing_key: &SigningKey,
    ) -> Receipt {
        // TODO: Should we generate timestamp in library?
        let nonce = thread_rng().gen::<u64>();
        Receipt {
            allocation_id,
            timestamp_ns,
            nonce, // Decide what RNG should be used (prioritize cheap compute)
            value,
            signature: signing_key.sign(&Self::get_message_bytes(
                allocation_id,
                timestamp_ns,
                nonce,
                value,
            )),
        }
    }
    /// Verifies given values match values on receipt and that receipts signature is valid for given verifying key, returns `Ok` if valid or an `Error` indicate what was found to be invalid.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidAllocationID`] if the allocation ID on the receipt does not exist in provided `allocation_ids`
    ///
    /// Returns [`Error::InvalidTimestamp`] if the receipt timestamp is not within the half-open range [`timestamp_min`, `timestamp_max`)
    ///
    /// Returns [`Error::InvalidValue`] if `Some` `expected_value` is provided but does not match receipts value
    ///
    /// Returns [`Error::InvalidSignature`] if the signature is not valid with provided `verifying_key`
    ///
    pub fn is_valid(
        self: &Self,
        verifying_key: VerifyingKey, //TODO: with multiple gateway operators how is this value known
        allocation_ids: &[Address],
        timestamp_min: u64,
        timestamp_max: u64,
        expected_value: Option<u64>,
    ) -> Result<()> {
        // TODO: update to return the public key found with ECRECOVER
        let timestamp_range = timestamp_min..timestamp_max;
        if !allocation_ids.contains(&self.allocation_id) {
            return Err(Error::InvalidAllocationID {
                received_allocation_id: self.allocation_id,
                expected_allocation_ids: format!("{:?}", allocation_ids),
            });
        }
        if !timestamp_range.contains(&self.timestamp_ns) {
            return Err(Error::InvalidTimestamp {
                received_timestamp: self.timestamp_ns,
                timestamp_min,
                timestamp_max,
            });
        }
        if let Some(expected_val) = expected_value {
            if expected_val != self.value {
                return Err(Error::InvalidValue {
                    received_value: self.value,
                    expected_value: expected_val,
                });
            }
        }
        self.is_valid_signature(verifying_key)
    }

    /// Checks that receipts signature is valid for given verifying key, returns `Ok` if it is valid.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSignature`] if the signature is not valid with provided `verifying_key`
    ///
    pub fn is_valid_signature(self: &Self, verifying_key: VerifyingKey) -> Result<()> {
        verifying_key.verify(
            &Self::get_message_bytes(
                self.allocation_id,
                self.timestamp_ns,
                self.nonce,
                self.value,
            ),
            &self.signature,
        )?;
        Ok(())
    }

    /// Creates a byte vector of the receipts message for signing
    ///
    fn get_message_bytes(
        allocation_id: Address,
        timestamp_ns: u64,
        nonce: u64,
        value: u64,
    ) -> Vec<u8> {
        allocation_id
            .as_bytes()
            .iter()
            .copied()
            .chain(timestamp_ns.to_be_bytes())
            .chain(nonce.to_be_bytes())
            .chain(value.to_be_bytes())
            .collect()
    }
}
