// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing EIP712 message and signature
//!

use crate::{Error, Result};
use ethers_core::types::transaction::eip712;
use k256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EIP712SignedMessage<M: eip712::Eip712> {
    /// Message to be signed
    pub message: M,
    /// ECDSA Signature of eip712 hash of message
    pub signature: Signature,
}

impl<M: eip712::Eip712> EIP712SignedMessage<M> {
    /// creates signed message with signed EIP712 hash of `message` using `signing_key`
    pub fn new(message: M, signing_key: &SigningKey) -> Result<Self> {
        let encoded_message = Self::get_eip712_encoding(&message)?;

        Ok(Self {
            message,
            signature: signing_key.sign(&encoded_message),
        })
    }

    /// Checks that receipts signature is valid for given verifying key, returns `Ok` if it is valid.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSignature`] if the signature is not valid with provided `verifying_key`
    ///
    pub fn check_signature(&self, verifying_key: VerifyingKey) -> Result<()> {
        verifying_key
            .verify(&Self::get_eip712_encoding(&self.message)?, &self.signature)
            .map_err(|err| crate::Error::InvalidSignature {
                source_error_message: err.to_string(),
            })?;
        Ok(())
    }

    /// Unable to cleanly typecast encode_eip712 associated error type to crate
    /// error type, so abstract away the error translating to this function
    fn get_eip712_encoding(message: &M) -> Result<[u8; 32]> {
        message
            .encode_eip712()
            .map_err(|e| Error::EIP712EncodeError {
                source_error_message: e.to_string(),
            })
    }
}
