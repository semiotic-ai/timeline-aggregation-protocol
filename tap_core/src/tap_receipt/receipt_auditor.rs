// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use alloy_sol_types::Eip712Domain;
use ethers::types::Signature;
use tokio::sync::RwLock;

use crate::{
    adapters::{escrow_adapter::EscrowAdapter, receipt_checks_adapter::ReceiptChecksAdapter},
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher,
    tap_receipt::{Receipt, ReceiptCheck, ReceiptError, ReceiptResult},
    Error, Result,
};

use super::{received_receipt::Checking, AwaitingReserve, ReceiptWithState};

pub struct ReceiptAuditor<E> {
    domain_separator: Eip712Domain,
    executor: E,
    min_timestamp_ns: RwLock<u64>,
}

impl<E> ReceiptAuditor<E> {
    pub fn new(
        domain_separator: Eip712Domain,
        executor: E,
        starting_min_timestamp_ns: u64,
    ) -> Self {
        Self {
            domain_separator,
            executor,
            min_timestamp_ns: RwLock::new(starting_min_timestamp_ns),
        }
    }

    /// Updates the minimum timestamp that will be accepted for a receipt (exclusive).
    pub async fn update_min_timestamp_ns(&self, min_timestamp_ns: u64) {
        *self.min_timestamp_ns.write().await = min_timestamp_ns;
    }

    async fn check_timestamp(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
    ) -> ReceiptResult<()> {
        let min_timestamp_ns = *self.min_timestamp_ns.read().await;
        if signed_receipt.message.timestamp_ns <= min_timestamp_ns {
            return Err(ReceiptError::InvalidTimestamp {
                received_timestamp: signed_receipt.message.timestamp_ns,
                timestamp_min: min_timestamp_ns,
            });
        }
        Ok(())
    }

    async fn check_timestamp_batch(
        &self,
        received_receipts: &mut [ReceiptWithState<Checking>],
    ) -> Vec<ReceiptResult<()>> {
        let mut results = Vec::new();

        for received_receipt in received_receipts
            .iter_mut()
            .filter(|r| r.state.checks.contains_key(&ReceiptCheck::CheckTimestamp))
        {
            if received_receipt.state.checks[&ReceiptCheck::CheckTimestamp].is_none() {
                let signed_receipt = &received_receipt.signed_receipt;
                results.push(self.check_timestamp(signed_receipt).await);
            }
        }

        results
    }

    async fn check_uniqueness_batch(
        &self,
        received_receipts: &mut [ReceiptWithState<Checking>],
    ) -> Vec<ReceiptResult<()>> {
        let mut results = Vec::new();

        // If at least one of the receipts in the batch hasn't been checked for uniqueness yet, check the whole batch.
        if received_receipts
            .iter()
            .filter(|r| r.state.checks.contains_key(&ReceiptCheck::CheckUnique))
            .any(|r| r.state.checks[&ReceiptCheck::CheckUnique].is_none())
        {
            let mut signatures: HashSet<Signature> = HashSet::new();

            for received_receipt in received_receipts {
                let signature = received_receipt.signed_receipt.signature;
                if signatures.insert(signature) {
                    results.push(Ok(()));
                } else {
                    results.push(Err(ReceiptError::NonUniqueReceipt));
                }
            }
        }

        results
    }
}

impl<E> ReceiptAuditor<E>
where
    E: EscrowAdapter + ReceiptChecksAdapter,
{
    pub async fn check(
        &self,
        receipt_check: &ReceiptCheck,
        signed_receipt: &EIP712SignedMessage<Receipt>,
        query_id: u64,
        receipt_id: u64,
    ) -> ReceiptResult<()> {
        match receipt_check {
            ReceiptCheck::CheckUnique => self.check_uniqueness(signed_receipt, receipt_id).await,
            ReceiptCheck::CheckAllocationId => self.check_allocation_id(signed_receipt).await,
            ReceiptCheck::CheckSignature => self.check_signature(signed_receipt).await,
            ReceiptCheck::CheckTimestamp => self.check_timestamp(signed_receipt).await,
            ReceiptCheck::CheckValue => self.check_value(signed_receipt, query_id).await,
        }
    }

    pub async fn check_batch(
        &self,
        receipt_check: &ReceiptCheck,
        received_receipts: &mut [ReceiptWithState<Checking>],
    ) -> Vec<ReceiptResult<()>> {
        match receipt_check {
            ReceiptCheck::CheckUnique => self.check_uniqueness_batch(received_receipts).await,
            ReceiptCheck::CheckAllocationId => {
                self.check_allocation_id_batch(received_receipts).await
            }
            ReceiptCheck::CheckSignature => self.check_signature_batch(received_receipts).await,
            ReceiptCheck::CheckTimestamp => self.check_timestamp_batch(received_receipts).await,
            ReceiptCheck::CheckValue => self.check_value_batch(received_receipts).await,
        }
    }
}

impl<E> ReceiptAuditor<E>
where
    E: ReceiptChecksAdapter,
{
    async fn check_uniqueness(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
        receipt_id: u64,
    ) -> ReceiptResult<()> {
        if !self
            .executor
            .is_unique(signed_receipt, receipt_id)
            .await
            .map_err(|e| ReceiptError::CheckFailedToComplete {
                source_error_message: e.to_string(),
            })?
        {
            return Err(ReceiptError::NonUniqueReceipt);
        }
        Ok(())
    }

    async fn check_allocation_id(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
    ) -> ReceiptResult<()> {
        if !self
            .executor
            .is_valid_allocation_id(signed_receipt.message.allocation_id)
            .await
            .map_err(|e| ReceiptError::CheckFailedToComplete {
                source_error_message: e.to_string(),
            })?
        {
            return Err(ReceiptError::InvalidAllocationID {
                received_allocation_id: signed_receipt.message.allocation_id,
            });
        }
        Ok(())
    }

    async fn check_allocation_id_batch(
        &self,
        received_receipts: &mut [ReceiptWithState<Checking>],
    ) -> Vec<ReceiptResult<()>> {
        let mut results = Vec::new();

        for received_receipt in received_receipts.iter_mut().filter(|r| {
            r.state
                .checks
                .contains_key(&ReceiptCheck::CheckAllocationId)
        }) {
            if received_receipt.state.checks[&ReceiptCheck::CheckAllocationId].is_none() {
                let signed_receipt = &received_receipt.signed_receipt;
                results.push(self.check_allocation_id(signed_receipt).await);
            }
        }

        results
    }

    async fn check_value(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
        query_id: u64,
    ) -> ReceiptResult<()> {
        if !self
            .executor
            .is_valid_value(signed_receipt.message.value, query_id)
            .await
            .map_err(|e| ReceiptError::CheckFailedToComplete {
                source_error_message: e.to_string(),
            })?
        {
            return Err(ReceiptError::InvalidValue {
                received_value: signed_receipt.message.value,
            });
        }
        Ok(())
    }

    async fn check_value_batch(
        &self,
        received_receipts: &mut [ReceiptWithState<Checking>],
    ) -> Vec<ReceiptResult<()>> {
        let mut results = Vec::new();

        for received_receipt in received_receipts
            .iter_mut()
            .filter(|r| r.state.checks.contains_key(&ReceiptCheck::CheckValue))
        {
            if received_receipt.state.checks[&ReceiptCheck::CheckValue].is_none() {
                let signed_receipt = &received_receipt.signed_receipt;
                results.push(
                    self.check_value(signed_receipt, received_receipt.query_id)
                        .await,
                );
            }
        }

        results
    }

    async fn check_signature(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
    ) -> ReceiptResult<()> {
        let receipt_signer_address = signed_receipt
            .recover_signer(&self.domain_separator)
            .map_err(|err| ReceiptError::InvalidSignature {
                source_error_message: err.to_string(),
            })?;
        if !self
            .executor
            .is_valid_sender_id(receipt_signer_address)
            .await
            .map_err(|e| ReceiptError::CheckFailedToComplete {
                source_error_message: e.to_string(),
            })?
        {
            return Err(ReceiptError::InvalidSignature {
                source_error_message: format!(
                    "Recovered sender id is not valid: {}",
                    receipt_signer_address
                ),
            });
        }
        Ok(())
    }

    async fn check_signature_batch(
        &self,
        received_receipts: &mut [ReceiptWithState<Checking>],
    ) -> Vec<ReceiptResult<()>> {
        let mut results = Vec::new();

        for received_receipt in received_receipts
            .iter_mut()
            .filter(|r| r.state.checks.contains_key(&ReceiptCheck::CheckSignature))
        {
            if received_receipt.state.checks[&ReceiptCheck::CheckSignature].is_none() {
                let signed_receipt = &received_receipt.signed_receipt;
                results.push(self.check_signature(signed_receipt).await);
            }
        }

        results
    }

    pub async fn check_rav_signature(
        &self,
        signed_rav: &EIP712SignedMessage<ReceiptAggregateVoucher>,
    ) -> Result<()> {
        let rav_signer_address = signed_rav.recover_signer(&self.domain_separator)?;
        if !self
            .executor
            .is_valid_sender_id(rav_signer_address)
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?
        {
            return Err(Error::InvalidRecoveredSigner {
                address: rav_signer_address,
            });
        }
        Ok(())
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
