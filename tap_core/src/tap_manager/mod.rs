// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

mod manager;
mod rav_request;

pub use manager::Manager;
pub use rav_request::RAVRequest;

use crate::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};

mod test;

pub type SignedReceipt = EIP712SignedMessage<Receipt>;
pub type SignedRAV = EIP712SignedMessage<ReceiptAggregateVoucher>;
