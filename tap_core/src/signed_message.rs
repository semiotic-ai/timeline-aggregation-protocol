// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # EIP712 message and signatures
//!
//! This module contains the `EIP712SignedMessage` struct which is used to sign and verify messages
//! using EIP712 standard.
//!
//! # Example
//! ```rust
//! # use alloy::{dyn_abi::Eip712Domain, primitives::Address, signers::local::PrivateKeySigner};
//! # let domain_separator = Eip712Domain::default();
//! use tap_eip712_message::EIP712SignedMessage;
//! # let wallet = PrivateKeySigner::random();
//! # let wallet_address = wallet.address();
//! # let message = msg::Receipt::new(Address::from([0x11u8; 20]), 100).unwrap();
//!
//! let signed_message = EIP712SignedMessage::new(&domain_separator, message, &wallet).unwrap();
//! let signer = signed_message.recover_signer(&domain_separator).unwrap();
//!
//! assert_eq!(signer, wallet_address);
//! ```
//!

pub use ::tap_eip712_message::*;
