// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # EIP712 signed message
//!
//! This crate contains the `EIP712SignedMessage` struct which is used to sign and verify messages
//! using EIP712 standard.
//!
//! # Example
//! ```rust
//! # use thegraph_core::alloy::{dyn_abi::Eip712Domain, primitives::Address, signers::local::PrivateKeySigner};
//! # let domain_separator = Eip712Domain::default();
//! use tap_eip712_message::Eip712SignedMessage;
//! # let wallet = PrivateKeySigner::random();
//! # let wallet_address = wallet.address();
//! # let message = msg::Receipt::new(Address::from([0x11u8; 20]), 100).unwrap();
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
    primitives::{keccak256, Address, Signature},
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
        SignatureBytes(self.as_bytes())
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

    /// Generate a uniqueness identifier using both the message content and the recovered signer
    pub fn unique_hash(&self, domain_separator: &Eip712Domain) -> Result<MessageId, Eip712Error> {
        // Get the hash of just the message content
        let message_hash = self.message.eip712_hash_struct();

        // Recover the signer address
        let signer = self.recover_signer(domain_separator)?;

        // Create a new hash combining both the message hash and the signer address
        let mut input = Vec::with_capacity(32 + 20); // 32 bytes for hash, 20 bytes for address
        input.extend_from_slice(&message_hash.0);
        input.extend_from_slice(signer.as_ref());

        let combined_hash = keccak256(&input);

        Ok(MessageId(*combined_hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thegraph_core::alloy::{
        primitives::{Address, Signature, U256},
        signers::local::PrivateKeySigner,
        sol_types::eip712_domain,
    };

    #[test]
    fn test_signature_malleability_resistance() {
        // Create a domain separator for testing
        let domain_separator = eip712_domain! {
            name: "TAP",
            version: "1",
            chain_id: 1,
            verifying_contract: Address:: from([0x11u8; 20]),
        };

        let test_value = 100u128;
        let test_address = Address::from([0x11u8; 20]);
        let message = msg::Receipt::new(test_address, test_value).unwrap();

        // Create a new Ethereum signer
        let signer = PrivateKeySigner::random();

        // Create signed message using the original signature
        let signed_message =
            Eip712SignedMessage::new(&domain_separator, message.clone(), &signer).unwrap();

        // Get the original signature components
        let r = signed_message.signature.r();
        let s = signed_message.signature.s();
        let v = signed_message.signature.v();

        // Create a malleated signature by changing the s value
        // Get the Secp256k1 curve order
        let n = U256::from_str_radix(
            "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
            16,
        )
        .unwrap();
        let s_malleated = n - s;
        let v_malleated = !v; // Flip the parity bit

        // Create a new signature with the malleated components
        let signature_malleated = Signature::new(r, s_malleated, v_malleated);

        // Create a new signed message with the malleated signature
        let signed_message_malleated = Eip712SignedMessage {
            message: message.clone(),
            signature: signature_malleated,
        };

        // Verify both signatures recover to the same signer
        let signer_address = signer.address();
        let recovered_address = signed_message.recover_signer(&domain_separator).unwrap();
        let recovered_address_malleated = signed_message_malleated
            .recover_signer(&domain_separator)
            .unwrap();

        assert_eq!(
            recovered_address, signer_address,
            "Original signature should recover to the correct address"
        );
        assert_eq!(
            recovered_address_malleated, signer_address,
            "Malleated signature should recover to the same address"
        );

        // Verify that the raw signatures are different
        assert_ne!(
            signed_message.signature.as_bytes(),
            signed_message_malleated.signature.as_bytes(),
            "Raw signature bytes should be different"
        );

        // WITHOUT our fix, these would generate different IDs:
        let raw_sig_id_1 = signed_message.signature.get_signature_bytes();
        let raw_sig_id_2 = signed_message_malleated.signature.get_signature_bytes();
        assert_ne!(
            raw_sig_id_1, raw_sig_id_2,
            "Using only raw signatures would fail to detect duplicates"
        );

        // WITH our fix, these should generate the same unique ID:
        let unique_id_1 = signed_message.unique_hash(&domain_separator).unwrap();
        let unique_id_2 = signed_message_malleated
            .unique_hash(&domain_separator)
            .unwrap();

        assert_eq!(
            unique_id_1, unique_id_2,
            "Our fix should generate the same ID for both original and malleated signatures"
        );

        // Bonus: Verify that different messages generate different IDs
        let different_message = msg::Receipt::new(test_address, test_value + 1).unwrap();
        let different_signed_message =
            Eip712SignedMessage::new(&domain_separator, different_message, &signer).unwrap();
        let different_unique_id = different_signed_message
            .unique_hash(&domain_separator)
            .unwrap();

        assert_ne!(
            unique_id_1, different_unique_id,
            "Different messages should generate different unique IDs"
        );
    }
}
