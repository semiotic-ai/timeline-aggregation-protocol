// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # The Graph TAP structs
//!
//! These structs are used for communication between The Graph systems.
//!

// AUDIT_SCOPE: V1_LEGACY - V1 module (allocation-based, out of scope)
mod v1;

#[cfg(any(test, feature = "v2"))]
pub mod v2;

// AUDIT_SCOPE: V1_LEGACY - Default exports are V1 types (out of scope)
pub use v1::{Receipt, ReceiptAggregateVoucher, SignedRav, SignedReceipt};
