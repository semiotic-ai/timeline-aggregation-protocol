// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # Receipts states and checks
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.
//!
//! A list of checks are performed on the received receipts to ensure they are valid.
//! The checks are performed when storing the receipt and when aggregating the receipts
//! into a Receipt Aggregate Voucher (RAV).
//!
//! Each receipt is wrapped into a State Machine that can be in one of the following states:
//! - `Checking`: The receipt is being checked.
//! - `Failed`: The receipt has failed a check or validation.
//! - `AwaitingReserve`: The receipt has passed all checks and is awaiting escrow reservation.
//! - `Reserved`: The receipt has successfully reserved escrow.
//!
//!

pub use ::tap_receipt::*;
