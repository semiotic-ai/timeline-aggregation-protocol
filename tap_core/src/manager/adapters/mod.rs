// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Context adapters for the TAP manager.
//!
//! Each adapter should be defined by the user of the library based on their
//! specific storage and verification requirements. This modular design
//! allows for easy integration with various storage solutions and verification
//! procedures, thereby making the library adaptable to a wide range of use cases.

mod escrow;
mod rav;
mod receipt;

pub use escrow::EscrowHandler;
pub use rav::*;
pub use receipt::*;
