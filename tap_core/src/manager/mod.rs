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

#[cfg(feature = "in_memory")]
pub mod context;
pub mod strategy;
mod tap_manager;

pub use tap_manager::Manager;
