// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Aggregation of Receipts

use tap_eip712_message::Eip712SignedMessage;
use thegraph_core::alloy::sol_types::SolStruct;

use crate::{state::Checked, ReceiptWithState};

pub trait Aggregate<T>: SolStruct {
    /// Aggregates a batch of validated receipts with optional validated previous RAV,
    /// returning a new RAV if all provided items are valid or an error if not.
    fn aggregate_receipts(
        receipts: &[ReceiptWithState<Checked, T>],
        previous_rav: Option<Eip712SignedMessage<Self>>,
    ) -> Result<Self, AggregationError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AggregationError {
    /// Error when trying to aggregate receipts and the result overflows
    #[error("Aggregating receipt results in overflow")]
    AggregateOverflow,

    /// Error when no valid receipts are found for a RAV request
    #[error("Failed to produce rav request, no valid receipts")]
    NoValidReceiptsForRavRequest,

    /// Other user-defined error
    #[error(transparent)]
    Other(anyhow::Error),
}
