// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt type used for providing and verifying a payment
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.

use alloy_sol_types::sol;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use thegraph::types::Address;

sol! {
    /// Holds information needed for promise of payment signed with ECDSA
    #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
    struct Receipt {
        /// Unique allocation id this receipt belongs to
        address allocation_id;
        /// Unix Epoch timestamp in nanoseconds (Truncated to 64-bits)
        uint64 timestamp_ns;
        /// Random value used to avoid collisions from multiple receipts with one timestamp
        uint64 nonce;
        /// GRT value for transaction (truncate to lower bits)
        uint128 value;
    }
}

impl Receipt {
    /// Returns a receipt with provided values
    pub fn new(allocation_id: Address, value: u128) -> crate::Result<Self> {
        let timestamp_ns = crate::get_current_timestamp_u64_ns()?;
        let nonce = thread_rng().gen::<u64>();
        Ok(Self {
            allocation_id,
            timestamp_ns,
            nonce,
            value,
        })
    }
}
