// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy::{dyn_abi::Eip712Domain, primitives::Address, sol_types::SolStruct};
use async_trait::async_trait;

use crate::{
    manager::WithValueAndTimestamp,
    receipt::{state::AwaitingReserve, ReceiptError, ReceiptResult, ReceiptWithState},
    signed_message::EIP712SignedMessage,
    Error,
};

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

    /// Retrieves the local accounting amount of available escrow for a specified sender.
    async fn get_available_escrow(&self, sender_id: Address) -> Result<u128, Self::AdapterError>;

    /// Deducts a specified value from the local accounting of available escrow for a specified sender.
    async fn subtract_escrow(
        &self,
        sender_id: Address,
        value: u128,
    ) -> Result<(), Self::AdapterError>;

    /// Verifies the signer of the receipt
    ///
    /// Used by [`Self::check_rav_signature`] to verify the signer of the receipt
    async fn verify_signer(&self, signer_address: Address) -> Result<bool, Self::AdapterError>;

    /// Checks and reserves escrow for the received receipt
    async fn check_and_reserve_escrow<T: SolStruct + WithValueAndTimestamp + Sync>(
        &self,
        received_receipt: &ReceiptWithState<AwaitingReserve, T>,
        domain_separator: &Eip712Domain,
    ) -> ReceiptResult<()> {
        let signed_receipt = &received_receipt.signed_receipt;
        let receipt_signer_address =
            signed_receipt
                .recover_signer(domain_separator)
                .map_err(|err| ReceiptError::InvalidSignature {
                    source_error_message: err.to_string(),
                })?;

        if self
            .subtract_escrow(receipt_signer_address, signed_receipt.message.value())
            .await
            .is_err()
        {
            return Err(ReceiptError::SubtractEscrowFailed);
        }

        Ok(())
    }

    /// Checks the signature of the RAV
    async fn check_rav_signature<R: SolStruct + Sync>(
        &self,
        signed_rav: &EIP712SignedMessage<R>,
        domain_separator: &Eip712Domain,
    ) -> Result<(), Error> {
        let recovered_address = signed_rav.recover_signer(domain_separator)?;
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
