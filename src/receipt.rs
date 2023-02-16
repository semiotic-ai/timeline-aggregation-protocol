// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use ethereum_types::Address;
use k256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};

use crate::{Error, Result};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Receipt {
    allocation_id: Address,
    timestamp_ns: u64,
    nonce: u64,
    value: u64,
    signature: Signature,
}

impl Receipt {
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
            nonce,
            value,
            signature: signing_key.sign(&Self::get_message_bytes(
                allocation_id,
                timestamp_ns,
                nonce,
                value,
            )),
        }
    }

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

    pub fn get_value(self: &Self) -> u64 {
        self.value
    }

    pub fn get_allocation_id(self: &Self) -> Address {
        self.allocation_id
    }

    pub fn get_timestamp_ns(self: &Self) -> u64 {
        self.timestamp_ns
    }

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
