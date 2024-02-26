// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt type used for providing and verifying a payment
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.

use std::cmp;

use alloy_primitives::Address;
use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::{eip_712_signed_message::EIP712SignedMessage, tap_receipt::Receipt};

sol! {
    /// Holds information needed for promise of payment signed with ECDSA
    #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
    struct ReceiptAggregateVoucher {
        /// Unique allocation id this RAV belongs to
        address allocationId;
        /// Unix Epoch timestamp in nanoseconds (Truncated to 64-bits)
        /// corresponding to max timestamp from receipt batch aggregated
        uint64 timestampNs;
        /// Aggregated GRT value from receipt batch and any previous RAV provided (truncate to lower bits)
        uint128 valueAggregate;
    }
}

impl ReceiptAggregateVoucher {
    /// Aggregates a batch of validated receipts with optional validated previous RAV, returning a new RAV if all provided items are valid or an error if not.
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
            timestamp_max = prev_rav.message.timestampNs;
            value_aggregate = prev_rav.message.valueAggregate;
        }

        for receipt in receipts {
            value_aggregate = value_aggregate
                .checked_add(receipt.message.value)
                .ok_or(Error::AggregateOverflow)?;

            timestamp_max = cmp::max(timestamp_max, receipt.message.timestamp_ns)
        }

        Ok(Self {
            allocationId: allocation_id,
            timestampNs: timestamp_max,
            valueAggregate: value_aggregate,
        })
    }
}
