// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The adapters module provides interfaces that allow flexibility in storing and verifying TAP components.
//!
//! Each adapter should be defined by the user of the library based on their specific storage and verification requirements. This modular design
//! allows for easy integration with various storage solutions and verification procedures, thereby making the library adaptable to a wide range
//! of use cases.
//!
//! The following adapters are defined:
//! - `collateral_adapter`: An interface for checking and updating collateral availability.
//! - `rav_storage_adapter`: An interface for storing and retrieving/replacing RAVs.
//! - `receipt_checks_adapter`: An interface for verifying TAP receipts.
//! - `receipt_storage_adapter`: An interface for storing, retrieving, updating, and removing TAP receipts.
//!
//! In addition, this module also includes mock implementations of each adapter for testing and example purposes.

pub mod collateral_adapter;
pub mod rav_storage_adapter;
pub mod receipt_checks_adapter;
pub mod receipt_storage_adapter;

mod test;

pub use test::collateral_adapter_mock;
pub use test::rav_storage_adapter_mock;
pub use test::receipt_checks_adapter_mock;
pub use test::receipt_storage_adapter_mock;
