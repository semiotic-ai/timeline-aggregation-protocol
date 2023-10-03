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

use super::ReceivedReceipt;

pub struct ReceiptAuditor<EA: EscrowAdapter, RCA: ReceiptChecksAdapter> {
    domain_separator: Eip712Domain,
    escrow_adapter: EA,
    receipt_checks_adapter: RCA,
    min_timestamp_ns: RwLock<u64>,
}

impl<EA: EscrowAdapter, RCA: ReceiptChecksAdapter> ReceiptAuditor<EA, RCA> {
    pub fn new(
        domain_separator: Eip712Domain,
        escrow_adapter: EA,
        receipt_checks_adapter: RCA,
        starting_min_timestamp_ns: u64,
    ) -> Self {
        Self {
            domain_separator,
            escrow_adapter,
            receipt_checks_adapter,
            min_timestamp_ns: RwLock::new(starting_min_timestamp_ns),
        }
    }

    /// Updates the minimum timestamp that will be accepted for a receipt (exclusive).
    pub async fn update_min_timestamp_ns(&self, min_timestamp_ns: u64) {
        *self.min_timestamp_ns.write().await = min_timestamp_ns;
    }

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
            ReceiptCheck::CheckAndReserveEscrow => {
                self.check_and_reserve_escrow(signed_receipt).await
            }
        }
    }

    pub async fn check_bunch(
        &self,
        receipt_check: &ReceiptCheck,
        received_receipts: &mut [ReceivedReceipt],
    ) -> Result<()> {
        let result = match receipt_check {
            ReceiptCheck::CheckUnique => self.check_uniqueness_bunch(received_receipts).await,
            ReceiptCheck::CheckAllocationId => {
                self.check_allocation_id_bunch(received_receipts).await
            }
            ReceiptCheck::CheckSignature => self.check_signature_bunch(received_receipts).await,
            ReceiptCheck::CheckTimestamp => self.check_timestamp_bunch(received_receipts).await,
            ReceiptCheck::CheckValue => self.check_value_bunch(received_receipts).await,
            ReceiptCheck::CheckAndReserveEscrow => {
                self.check_and_reserve_escrow_bunch(received_receipts).await
            }
        };

        for received_receipt in received_receipts.iter_mut() {
            received_receipt.update_state();
        }

        result
    }

    async fn check_uniqueness(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
        receipt_id: u64,
    ) -> ReceiptResult<()> {
        if !self
            .receipt_checks_adapter
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

    async fn check_uniqueness_bunch(
        &self,
        received_receipts: &mut [ReceivedReceipt],
    ) -> Result<()> {
        // If at least one of the receipts in the bunch hasn't been checked for uniqueness yet, check the whole bunch.
        if received_receipts
            .iter()
            .filter(|r| r.checks.get(&ReceiptCheck::CheckUnique).is_some())
            .any(|r| r.checks[&ReceiptCheck::CheckUnique].is_none())
        {
            let mut signatures: HashSet<Signature> = HashSet::new();

            for received_receipt in received_receipts {
                let signature = received_receipt.signed_receipt.signature;
                if signatures.insert(signature) {
                    received_receipt
                        .update_check(&ReceiptCheck::CheckUnique, Some(Ok(())))
                        .unwrap();
                } else {
                    received_receipt
                        .update_check(
                            &ReceiptCheck::CheckUnique,
                            Some(Err(ReceiptError::NonUniqueReceipt)),
                        )
                        .unwrap();
                }
            }
        }

        Ok(())
    }

    async fn check_allocation_id(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
    ) -> ReceiptResult<()> {
        if !self
            .receipt_checks_adapter
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

    async fn check_allocation_id_bunch(
        &self,
        received_receipts: &mut [ReceivedReceipt],
    ) -> Result<()> {
        for received_receipt in received_receipts
            .iter_mut()
            .filter(|r| r.checks.get(&ReceiptCheck::CheckAllocationId).is_some())
        {
            if received_receipt.checks[&ReceiptCheck::CheckAllocationId].is_none() {
                let signed_receipt = &received_receipt.signed_receipt;
                let result = self.check_allocation_id(signed_receipt).await;

                received_receipt
                    .update_check(&ReceiptCheck::CheckAllocationId, Some(result))
                    .unwrap();
            }
        }

        Ok(())
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

    async fn check_timestamp_bunch(&self, received_receipts: &mut [ReceivedReceipt]) -> Result<()> {
        for received_receipt in received_receipts
            .iter_mut()
            .filter(|r| r.checks.get(&ReceiptCheck::CheckTimestamp).is_some())
        {
            if received_receipt.checks[&ReceiptCheck::CheckTimestamp].is_none() {
                let signed_receipt = &received_receipt.signed_receipt;
                let result = self.check_timestamp(signed_receipt).await;

                received_receipt
                    .update_check(&ReceiptCheck::CheckTimestamp, Some(result))
                    .unwrap();
            }
        }

        Ok(())
    }

    async fn check_value(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
        query_id: u64,
    ) -> ReceiptResult<()> {
        if !self
            .receipt_checks_adapter
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

    async fn check_value_bunch(&self, received_receipts: &mut [ReceivedReceipt]) -> Result<()> {
        for received_receipt in received_receipts
            .iter_mut()
            .filter(|r| r.checks.get(&ReceiptCheck::CheckValue).is_some())
        {
            if received_receipt.checks[&ReceiptCheck::CheckValue].is_none() {
                let signed_receipt = &received_receipt.signed_receipt;
                let result = self
                    .check_value(signed_receipt, received_receipt.query_id)
                    .await;

                received_receipt
                    .update_check(&ReceiptCheck::CheckValue, Some(result))
                    .unwrap();
            }
        }

        Ok(())
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
            .receipt_checks_adapter
            .is_valid_gateway_id(receipt_signer_address)
            .await
            .map_err(|e| ReceiptError::CheckFailedToComplete {
                source_error_message: e.to_string(),
            })?
        {
            return Err(ReceiptError::InvalidSignature {
                source_error_message: format!(
                    "Recovered gateway id is not valid: {}",
                    receipt_signer_address
                ),
            });
        }
        Ok(())
    }

    async fn check_signature_bunch(&self, received_receipts: &mut [ReceivedReceipt]) -> Result<()> {
        for received_receipt in received_receipts
            .iter_mut()
            .filter(|r| r.checks.get(&ReceiptCheck::CheckSignature).is_some())
        {
            if received_receipt.checks[&ReceiptCheck::CheckSignature].is_none() {
                let signed_receipt = &received_receipt.signed_receipt;
                let result = self.check_signature(signed_receipt).await;

                received_receipt
                    .update_check(&ReceiptCheck::CheckSignature, Some(result))
                    .unwrap();
            }
        }

        Ok(())
    }

    async fn check_and_reserve_escrow(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
    ) -> ReceiptResult<()> {
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

    async fn check_and_reserve_escrow_bunch(
        &self,
        received_receipts: &mut [ReceivedReceipt],
    ) -> Result<()> {
        for received_receipt in received_receipts.iter_mut().filter(|r| {
            r.escrow_reserve_attempt_required() && !r.escrow_reserve_attempt_completed()
        }) {
            let signed_receipt = &received_receipt.signed_receipt;
            let result = self.check_and_reserve_escrow(signed_receipt).await;

            received_receipt
                .update_check(&ReceiptCheck::CheckAndReserveEscrow, Some(result))
                .unwrap();
        }

        Ok(())
    }

    pub async fn check_rav_signature(
        &self,
        signed_rav: &EIP712SignedMessage<ReceiptAggregateVoucher>,
    ) -> Result<()> {
        let rav_signer_address = signed_rav.recover_signer(&self.domain_separator)?;
        if !self
            .receipt_checks_adapter
            .is_valid_gateway_id(rav_signer_address)
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
