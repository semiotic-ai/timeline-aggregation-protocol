// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy::{dyn_abi::Eip712Domain, primitives::Address, sol_types::SolStruct};
use async_trait::async_trait;

use crate::{signed_message::EIP712SignedMessage, Error};

/// Manages the escrow operations
///
/// # Example
///
/// For example code see [crate::manager::context::memory::EscrowStorage]

#[async_trait]
pub trait EscrowHandler: Send + Sync {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from
    /// the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Verifies the signer of the receipt
    async fn verify_signer(&self, signer_address: Address) -> Result<bool, Self::AdapterError>;

    /// Checks if the signed message has a sender signature
    async fn check_signature<T: SolStruct + Sync>(
        &self,
        signed_message: &EIP712SignedMessage<T>,
        domain_separator: &Eip712Domain,
    ) -> Result<(), Error> {
        let recovered_address = signed_message.recover_signer(domain_separator)?;
        if self
            .verify_signer(recovered_address)
            .await
            .map_err(|e| Error::FailedToVerifySigner(e.to_string()))?
        {
            Ok(())
        } else {
            Err(Error::InvalidRecoveredSigner {
                address: recovered_address,
            })
        }
    }
}
