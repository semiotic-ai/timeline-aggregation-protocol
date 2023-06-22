// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::{SignedRAV, SignedReceipt};
use crate::receipt_aggregate_voucher::ReceiptAggregateVoucher;

#[derive(Debug, Serialize, Deserialize, Clone)]

pub struct RAVRequest {
    pub valid_receipts: Vec<SignedReceipt>,
    pub previous_rav: Option<SignedRAV>,
    pub invalid_receipts: Vec<SignedReceipt>,
    pub expected_rav: ReceiptAggregateVoucher,
}
