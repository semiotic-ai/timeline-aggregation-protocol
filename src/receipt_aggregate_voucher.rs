// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt type used for providing and verifying a payment
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.

use crate::Error;
use crate::{eip_712_signed_message::EIP712SignedMessage, tap_receipt::Receipt};
use ethereum_types::Address;
use ethers_contract::EthAbiType;
use ethers_core::types::transaction::eip712::Eip712;
use ethers_derive_eip712::*;
use serde::{Deserialize, Serialize};
use std::cmp;

/// Holds information needed for promise of payment signed with ECDSA
#[derive(Debug, Serialize, Deserialize, Clone, Eip712, EthAbiType)]
#[eip712(
    //TODO: Update this info, or make it user defined?
    name = "tap",
    version = "1",
    chain_id = 1,
    verifying_contract = "0x0000000000000000000000000000000000000000"
)]
pub struct ReceiptAggregateVoucher {
    /// Unique allocation id this RAV belongs to
    pub allocation_id: Address,
    /// Unix Epoch timestamp in nanoseconds (Truncated to 64-bits)
    /// corresponding to max timestamp from receipt batch aggregated
    pub timestamp: u64,
    /// Aggregated GRT value from receipt batch and any previous RAV provided (truncate to lower bits)
    pub value_aggregate: u128,
}

impl ReceiptAggregateVoucher {
    /// Aggregates a batch of validated receipts with optional validated previous RAV, returning a new signed RAV if all provided items are valid or an error if not.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AggregateOverflow`] if any receipt value causes aggregate value to overflow
    ///
    pub fn aggregate_receipts(
        allocation_id: Address,
        receipts: &[EIP712SignedMessage<Receipt>],
        previous_rav: Option<EIP712SignedMessage<Self>>,
    ) -> crate::Result<Self> {
        //TODO(#29): When receipts in flight struct in created check that the state of every receipt is OK with all checks complete (relies on #28)
        // If there is a previous RAV get initalize values from it, otherwise get default values
        let mut timestamp_max = 0u64;
        let mut value_aggregate = 0u128;

        if let Some(prev_rav) = previous_rav {
            timestamp_max = prev_rav.message.timestamp;
            value_aggregate = prev_rav.message.value_aggregate;
        }

        for receipt in receipts {
            value_aggregate = value_aggregate
                .checked_add(receipt.message.value)
                .ok_or(Error::AggregateOverflow)?;

            timestamp_max = cmp::max(timestamp_max, receipt.message.timestamp_ns)
        }

        Ok(Self {
            allocation_id,
            timestamp: timestamp_max,
            value_aggregate,
        })
    }
}
