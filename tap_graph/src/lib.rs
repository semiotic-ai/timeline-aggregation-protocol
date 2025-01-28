// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # The Graph TAP structs
//!
//! These structs are used for communication between The Graph systems.
//!

mod rav;
mod receipt;

pub use rav::{ReceiptAggregateVoucher, SignedRav};
pub use receipt::{Receipt, SignedReceipt};
