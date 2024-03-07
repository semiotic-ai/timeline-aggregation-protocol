// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing EIP712 message and signature
//!

use alloy_primitives::Address;
use alloy_sol_types::{Eip712Domain, SolStruct};
use ethers::{signers::LocalWallet, types::Signature};
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct EIP712SignedMessage<M: SolStruct> {
    /// Message to be signed
    pub message: M,
    /// ECDSA Signature of eip712 hash of message
    pub signature: Signature,
}

#[derive(Eq, PartialEq, Hash)]
pub struct MessageId(pub [u8; 32]);

impl<M: SolStruct> EIP712SignedMessage<M> {
    /// creates signed message with signed EIP712 hash of `message` using `signing_wallet`
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
    /// Returns [`Error::InvalidSignature`] if the signature is not valid with provided `verifying_key`
    ///
    pub fn verify(&self, domain_separator: &Eip712Domain, expected_address: Address) -> Result<()> {
        let recovery_message_hash = self.hash(domain_separator);
        let expected_address: [u8; 20] = expected_address.into();

        self.signature
            .verify(recovery_message_hash, expected_address)?;
        Ok(())
    }

    pub fn unique_hash(&self) -> MessageId {
        MessageId(self.message.eip712_hash_struct().into())
    }

    fn hash(&self, domain_separator: &Eip712Domain) -> [u8; 32] {
        let recovery_message_hash: [u8; 32] =
            self.message.eip712_signing_hash(domain_separator).into();
        recovery_message_hash
    }
}
