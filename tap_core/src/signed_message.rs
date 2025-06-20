// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # EIP712 message and signatures
//!
//! This module contains the `EIP712SignedMessage` struct which is used to sign and verify messages
//! using EIP712 standard.
//!
//! # Example
//! ```rust
//! # use thegraph_core::alloy::{dyn_abi::Eip712Domain, primitives::{Address, U256}, signers::local::PrivateKeySigner};
//! # use tap_graph::{Receipt};
//! # let domain_separator = Eip712Domain::default();
//! use tap_core::signed_message::Eip712SignedMessage;
//! # let wallet = PrivateKeySigner::random();
//! # let wallet_address = wallet.address();
//! # let message = Receipt::new(Address::from([0x11u8; 20]), U256::from(100)).unwrap();
//!
//! let signed_message = Eip712SignedMessage::new(&domain_separator, message, &wallet).unwrap();
//! let signer = signed_message.recover_signer(&domain_separator).unwrap();
//!
//! assert_eq!(signer, wallet_address);
//! ```
//!

pub use ::tap_eip712_message::*;
