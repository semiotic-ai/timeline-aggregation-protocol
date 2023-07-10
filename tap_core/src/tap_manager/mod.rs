// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The `tap_manager` module provides facilities for managing TAP receipt and RAV validation, as well as storage flow.
//!
//! This module should be the primary interface for the receiver of funds to verify, store, and manage TAP receipts and RAVs.
//! The `Manager` struct within this module allows the user to specify what checks should be performed on receipts, as well as
//! when these checks should occur (either when a receipt is first received, or when it is being added to a RAV request).
//!
//! The `Manager` uses user-defined adapters (see [crate::adapters]) for check and storage handling.
//! This design offers a high degree of flexibility, letting the user define their own behavior for these critical operations.

mod manager;
mod rav_request;

pub use manager::Manager;
pub use rav_request::RAVRequest;

use crate::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};

pub type SignedReceipt = EIP712SignedMessage<Receipt>;
pub type SignedRAV = EIP712SignedMessage<ReceiptAggregateVoucher>;
