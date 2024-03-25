// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # EIP712 message and signature
//!
//! This module contains the `EIP712SignedMessage` struct which is used to sign and verify messages
//! using EIP712 standard.
//!
//! # Example
//! ```rust
//! # use alloy_sol_types::Eip712Domain;
//! # let domain_separator = Eip712Domain::default();
//! # use ethers::{
//! #     signers::LocalWallet,
//! #     signers::Signer
//! # };
//! # use alloy_primitives::Address;
//! use tap_core::{
//!     signed_message::EIP712SignedMessage,
//!     receipt::Receipt
//! };
//! # let wallet = LocalWallet::new(&mut rand::thread_rng());
//! # let wallet_address = Address::from_slice(wallet.address().as_bytes());
//! # let message = Receipt::new(Address::from([0x11u8; 20]), 100).unwrap();
//!
//! let signed_message = EIP712SignedMessage::new(&domain_separator, message, &wallet).unwrap();
//! let signer = signed_message.recover_signer(&domain_separator).unwrap();
//!
//! assert_eq!(signer, wallet_address);
//! ```
//!

use alloy_primitives::Address;
use alloy_sol_types::{Eip712Domain, SolStruct};
use ethers::{signers::LocalWallet, types::Signature};
use serde::{Deserialize, Serialize};

use crate::Result;

/// EIP712 signed message
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct EIP712SignedMessage<M: SolStruct> {
    /// Message to be signed
    pub message: M,
    /// ECDSA Signature of eip712 hash of message
    pub signature: Signature,
}

/// Unique identifier for a message
///
/// This is equal to the hash of the hash of the contents of a message. This means
/// that two receipts signed by two different signers will have the same id.
///
///
/// This cannot be used as a unique identifier for a message, but can be used as a key
/// for a hashmap where the value is the message.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct MessageId(pub [u8; 32]);

impl<M: SolStruct> EIP712SignedMessage<M> {
    /// Creates a signed message with signed EIP712 hash of `message` using `signing_wallet`
    pub fn new(
        domain_separator: &Eip712Domain,
        message: M,
        signing_wallet: &LocalWallet,
    ) -> Result<Self> {
        let recovery_message_hash: [u8; 32] = message.eip712_signing_hash(domain_separator).into();

        let signature = signing_wallet.sign_hash(recovery_message_hash.into())?;

        Ok(Self { message, signature })
    }

    /// Recovers and returns the signer of the message from the signature.
    pub fn recover_signer(&self, domain_separator: &Eip712Domain) -> Result<Address> {
        let recovery_message_hash: [u8; 32] =
            self.message.eip712_signing_hash(domain_separator).into();
        let recovered_address: [u8; 20] = self.signature.recover(recovery_message_hash)?.into();
        Ok(recovered_address.into())
    }

    /// Checks that receipts signature is valid for given verifying key, returns `Ok` if it is valid.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::SignatureError`] if the recovered address from the
    /// signature is not equal to `expected_address`
    ///
    pub fn verify(&self, domain_separator: &Eip712Domain, expected_address: Address) -> Result<()> {
        let recovery_message_hash: [u8; 32] =
            self.message.eip712_signing_hash(domain_separator).into();
        let expected_address: [u8; 20] = expected_address.into();

        self.signature
            .verify(recovery_message_hash, expected_address)?;
        Ok(())
    }

    /// Use this a simple key for testing
    pub fn unique_hash(&self) -> MessageId {
        MessageId(self.message.eip712_hash_struct().into())
    }
}
