// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt type used for providing and verifying a payment
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.

use ethereum_types::Address;
use ethers_contract::EthAbiType;
use ethers_core::types::transaction::eip712::Eip712;
use ethers_derive_eip712::*;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Holds information needed for promise of payment signed with ECDSA
#[derive(Debug, Serialize, Deserialize, Clone, Eip712, EthAbiType)]
#[eip712(
    //TODO: Update this info, or make it user defined?
    name = "tap",
    version = "1",
    chain_id = 1,
    verifying_contract = "0x0000000000000000000000000000000000000000"
)]
pub struct Receipt {
    /// Unique allocation id this receipt belongs to
    pub allocation_id: Address,
    /// Unix Epoch timestamp in nanoseconds (Truncated to 64-bits)
    pub timestamp_ns: u64,
    /// Random value used to avoid collisions from multiple receipts with one timestamp
    pub nonce: u64,
    /// GRT value for transaction (truncate to lower bits)
    pub value: u128,
}

impl Receipt {
    /// Returns a receipt with provided values signed with `signing_key`
    pub fn new(allocation_id: Address, value: u128) -> crate::Result<Self> {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| crate::Error::InvalidSystemTime {
                source_error_message: err.to_string(),
            })?
            .as_millis() as u64;
        let nonce = thread_rng().gen::<u64>();
        Ok(Self {
            allocation_id,
            timestamp_ns,
            nonce,
            value,
        })
    }
}
