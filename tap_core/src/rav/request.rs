// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy::sol_types::SolStruct;

use crate::{
    receipt::{
        state::{Failed, Reserved},
        ReceiptWithState,
    },
    signed_message::EIP712SignedMessage,
    Error,
};

/// Request to `tap_aggregator` to aggregate receipts into a Signed RAV.
#[derive(Debug)]
pub struct RAVRequest<T, Rav>
where
    T: SolStruct,
    Rav: SolStruct,
{
    /// List of checked and reserved receipts to aggregate
    pub valid_receipts: Vec<ReceiptWithState<Reserved, T>>,
    /// Optional previous RAV to aggregate with
    pub previous_rav: Option<EIP712SignedMessage<Rav>>,
    /// List of failed receipt used to log invalid receipts
    pub invalid_receipts: Vec<ReceiptWithState<Failed, T>>,
    /// Expected RAV to be created
    pub expected_rav: Result<Rav, Error>,
}
