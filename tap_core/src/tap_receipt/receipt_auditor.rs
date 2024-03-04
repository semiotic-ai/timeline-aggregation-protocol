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

pub struct ReceiptAuditor<EA> {
    domain_separator: Eip712Domain,
    escrow_adapter: EA,
}

impl<EA> ReceiptAuditor<EA> {
    pub fn new(
        domain_separator: Eip712Domain,
        escrow_adapter: EA,
    ) -> Self {
        Self {
            domain_separator,
            escrow_adapter,
        }
    }

    pub async fn check_rav_signature<F, Fut>(
        &self,
        signed_rav: &SignedRAV,
        verify_signer: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(Address) -> Fut,
        Fut: Future<Output = Result<bool, Error>>,
    {
        let recovered_address = signed_rav.recover_signer(&self.domain_separator)?;
        if verify_signer(recovered_address).await? {
            Ok(())
        } else {
            Err(Error::InvalidRecoveredSigner {
                address: recovered_address,
            })
        }
    }
}

impl<EA> ReceiptAuditor<EA>
where
    EA: EscrowAdapter,
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
            .escrow_adapter
            .subtract_escrow(receipt_signer_address, signed_receipt.message.value)
            .await
            .is_err()
        {
            return Err(ReceiptError::SubtractEscrowFailed);
        }

        Ok(())
    }
}
