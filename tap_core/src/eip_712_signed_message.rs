// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing EIP712 message and signature
//!

use crate::{Error, Result};
use ethers::{
    signers::{LocalWallet, Signer},
    types::Signature,
};
use ethers_core::types::{transaction::eip712, Address};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EIP712SignedMessage<M: eip712::Eip712 + Send + Sync> {
    /// Message to be signed
    pub message: M,
    /// ECDSA Signature of eip712 hash of message
    pub signature: Signature,
}

impl<M: eip712::Eip712 + Send + Sync> EIP712SignedMessage<M> {
    /// creates signed message with signed EIP712 hash of `message` using `signing_wallet`
    pub async fn new(message: M, signing_wallet: &LocalWallet) -> Result<Self> {
        let signature = signing_wallet.sign_typed_data(&message).await?;

        Ok(Self { message, signature })
    }

    /// Recovers and returns the signer of the message from the signature.
    pub fn recover_signer(&self) -> Result<Address> {
        Ok(self
            .signature
            .recover(Self::get_eip712_encoding(&self.message)?)?)
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
