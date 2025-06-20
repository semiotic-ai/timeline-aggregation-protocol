// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # EIP712 signed message
//!
//! This crate contains the `EIP712SignedMessage` struct which is used to sign and verify messages
//! using EIP712 standard.
//!
//! # Example
//! ```rust
//! # use thegraph_core::alloy::{dyn_abi::Eip712Domain, primitives::{Address, U256}, signers::local::PrivateKeySigner};
//! # let domain_separator = Eip712Domain::default();
//! use tap_eip712_message::Eip712SignedMessage;
//! # let wallet = PrivateKeySigner::random();
//! # let wallet_address = wallet.address();
//! # let message = msg::Receipt::new(Address::from([0x11u8; 20]), U256::from(100)).unwrap();
//!
//! let signed_message = Eip712SignedMessage::new(&domain_separator, message, &wallet).unwrap();
//! let signer = signed_message.recover_signer(&domain_separator).unwrap();
//!
//! assert_eq!(signer, wallet_address);
//! ```
//!

use serde::{Deserialize, Serialize};
use thegraph_core::alloy::{
    dyn_abi::Eip712Domain,
    primitives::{Address, Signature},
    signers::{local::PrivateKeySigner, SignerSync},
    sol_types::SolStruct,
};

/// Errors returned by creation of messages and verify signature
#[derive(thiserror::Error, Debug)]
pub enum Eip712Error {
    /// `alloy` wallet error
    #[error(transparent)]
    WalletError(#[from] thegraph_core::alloy::signers::Error),

    /// `alloy` signature error
    #[error(transparent)]
    SignatureError(#[from] thegraph_core::alloy::primitives::SignatureError),
}

/// EIP712 signed message
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Eip712SignedMessage<M: SolStruct> {
    /// Message to be signed
    pub message: M,
    /// ECDSA Signature of eip712 hash of message
    pub signature: Signature,
}

/// Signature that can be used in a HashSet
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SignatureBytes([u8; 65]);

/// Extension for Signature to return [SignatureBytes]
pub trait SignatureBytesExt {
    fn get_signature_bytes(&self) -> SignatureBytes;
}

impl SignatureBytesExt for Signature {
    fn get_signature_bytes(&self) -> SignatureBytes {
        // Canonicalize to low-S form before returning bytes
        let canonical = self.normalized_s();
        SignatureBytes(canonical.as_bytes())
    }
}

/// Unique identifier for a message
///
/// This is equal to the hash of the contents of a message, excluding the signature.
/// This means that two receipts signed by two different signers will have the same id.
///
///
/// This cannot be used as a unique identifier for a message, but can be used as a key
/// for a hashmap where the value is the message.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct MessageId(pub [u8; 32]);

impl<M: SolStruct> Eip712SignedMessage<M> {
    /// Creates a signed message with signed EIP712 hash of `message` using `signing_wallet`
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::WalletError`] if could not sign using the wallet
    ///
    pub fn new(
        domain_separator: &Eip712Domain,
        message: M,
        signing_wallet: &PrivateKeySigner,
    ) -> Result<Self, Eip712Error> {
        let recovery_message_hash = message.eip712_signing_hash(domain_separator);

        let signature = signing_wallet.sign_hash_sync(&recovery_message_hash)?;

        Ok(Self { message, signature })
    }

    /// Recovers and returns the signer of the message from the signature.
    pub fn recover_signer(&self, domain_separator: &Eip712Domain) -> Result<Address, Eip712Error> {
        let recovery_message_hash = self.message.eip712_signing_hash(domain_separator);
        let recovered_address = self
            .signature
            .recover_address_from_prehash(&recovery_message_hash)?;
        Ok(recovered_address)
    }

    /// Use this as a simple key for testing
    pub fn unique_hash(&self) -> MessageId {
        MessageId(self.message.eip712_hash_struct().into())
    }
}
