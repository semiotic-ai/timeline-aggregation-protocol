// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy_sol_types::Eip712Domain;

use crate::{
    adapters::escrow_adapter::EscrowAdapter,
    tap_receipt::{ReceiptError, ReceiptResult},
};

use super::{AwaitingReserve, ReceiptWithState};

pub struct ReceiptAuditor<E> {
    domain_separator: Eip712Domain,
    executor: E,
}

impl<E> ReceiptAuditor<E> {
    pub fn new(
        domain_separator: Eip712Domain,
        executor: E,
    ) -> Self {
        Self {
            domain_separator,
            executor,
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
