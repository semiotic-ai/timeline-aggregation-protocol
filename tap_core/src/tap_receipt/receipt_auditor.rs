// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy_primitives::Address;
use alloy_sol_types::Eip712Domain;
use futures::Future;

use crate::{
    adapters::escrow_adapter::EscrowAdapter,
    tap_manager::SignedRAV,
    tap_receipt::{ReceiptError, ReceiptResult},
    Error,
};

use super::{AwaitingReserve, ReceiptWithState};

pub struct ReceiptAuditor<E> {
    domain_separator: Eip712Domain,
    executor: E,
}

impl<E> ReceiptAuditor<E> {
    pub fn new(domain_separator: Eip712Domain, executor: E) -> Self {
        Self {
            domain_separator,
            executor,
        }
    }

    pub async fn check_rav_signature<F, Fut, Err>(
        &self,
        signed_rav: &SignedRAV,
        verify_signer: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(Address) -> Fut,
        Fut: Future<Output = Result<bool, Err>>,
        Err: std::fmt::Display,
    {
        let recovered_address = signed_rav.recover_signer(&self.domain_separator)?;
        if verify_signer(recovered_address)
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

impl<E> ReceiptAuditor<E>
where
    E: EscrowAdapter,
{
    pub async fn check_and_reserve_escrow(
        &self,
        received_receipt: &ReceiptWithState<AwaitingReserve>,
    ) -> ReceiptResult<()> {
        let signed_receipt = &received_receipt.signed_receipt;
        let receipt_signer_address = signed_receipt
            .recover_signer(&self.domain_separator)
            .map_err(|err| ReceiptError::InvalidSignature {
                source_error_message: err.to_string(),
            })?;

        if self
            .executor
            .subtract_escrow(receipt_signer_address, signed_receipt.message.value)
            .await
            .is_err()
        {
            return Err(ReceiptError::SubtractEscrowFailed);
        }

        Ok(())
    }
}
