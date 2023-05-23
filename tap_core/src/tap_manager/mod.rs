// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The tap_manager is used to manage receipt and RAV validation and storage flow
//!
//! The tap_manager
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.

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
