// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    adapters::{
        collateral_adapter::CollateralAdapter, receipt_checks_adapter::ReceiptChecksAdapter,
    },
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher,
    tap_receipt::{Receipt, ReceiptCheck, ReceiptError, ReceiptResult},
    Error, Result,
};

pub struct ReceiptAuditor<CA: CollateralAdapter, RCA: ReceiptChecksAdapter> {
    collateral_adapter: CA,
    receipt_checks_adapter: RCA,
    min_timestamp_ns: u64,
}

impl<CA: CollateralAdapter, RCA: ReceiptChecksAdapter> ReceiptAuditor<CA, RCA> {
    pub fn new(
        collateral_adapter: CA,
        receipt_checks_adapter: RCA,
        starting_min_timestamp_ns: u64,
    ) -> Self {
        Self {
            collateral_adapter,
            receipt_checks_adapter,
            min_timestamp_ns: starting_min_timestamp_ns,
        }
    }

    pub fn update_min_timestamp_ns(&mut self, min_timestamp_ns: u64) {
        self.min_timestamp_ns = min_timestamp_ns;
    }

    pub fn check(
        &mut self,
        receipt_check: &ReceiptCheck,
        signed_receipt: &EIP712SignedMessage<Receipt>,
        query_id: u64,
        receipt_id: u64,
    ) -> ReceiptResult<()> {
        match receipt_check {
            ReceiptCheck::CheckUnique => self.check_uniqueness(signed_receipt, receipt_id),
            ReceiptCheck::CheckAllocationId => self.check_allocation_id(signed_receipt),
            ReceiptCheck::CheckSignature => self.check_signature(signed_receipt),
            ReceiptCheck::CheckTimestamp => self.check_timestamp(signed_receipt),
            ReceiptCheck::CheckValue => self.check_value(signed_receipt, query_id),
            ReceiptCheck::CheckAndReserveCollateral => {
                self.check_and_reserve_collateral(signed_receipt)
            }
        }
    }

    fn check_uniqueness(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
        receipt_id: u64,
    ) -> ReceiptResult<()> {
        if !self
            .receipt_checks_adapter
            .is_unique(signed_receipt, receipt_id)
        {
            return Err(ReceiptError::NonUniqueReceipt);
        }
        Ok(())
    }

    fn check_allocation_id(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
    ) -> ReceiptResult<()> {
        if !self
            .receipt_checks_adapter
            .is_valid_allocation_id(signed_receipt.message.allocation_id)
        {
            return Err(ReceiptError::InvalidAllocationID {
                received_allocation_id: signed_receipt.message.allocation_id,
            });
        }
        Ok(())
    }

    fn check_timestamp(&self, signed_receipt: &EIP712SignedMessage<Receipt>) -> ReceiptResult<()> {
        if signed_receipt.message.timestamp_ns <= self.min_timestamp_ns {
            return Err(ReceiptError::InvalidTimestamp {
                received_timestamp: signed_receipt.message.timestamp_ns,
                timestamp_min: self.min_timestamp_ns,
            });
        }
        Ok(())
    }
    fn check_value(
        &self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
        query_id: u64,
    ) -> ReceiptResult<()> {
        if !self
            .receipt_checks_adapter
            .is_valid_value(signed_receipt.message.value, query_id)
        {
            return Err(ReceiptError::InvalidValue {
                received_value: signed_receipt.message.value,
            });
        }
        Ok(())
    }

    fn check_signature(&self, signed_receipt: &EIP712SignedMessage<Receipt>) -> ReceiptResult<()> {
        let receipt_signer_address =
            signed_receipt
                .recover_signer()
                .map_err(|err| ReceiptError::InvalidSignature {
                    source_error_message: err.to_string(),
                })?;
        if !self
            .receipt_checks_adapter
            .is_valid_gateway_id(receipt_signer_address)
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

    fn check_and_reserve_collateral(
        &mut self,
        signed_receipt: &EIP712SignedMessage<Receipt>,
    ) -> ReceiptResult<()> {
        let receipt_signer_address =
            signed_receipt
                .recover_signer()
                .map_err(|err| ReceiptError::InvalidSignature {
                    source_error_message: err.to_string(),
                })?;
        if self
            .collateral_adapter
            .subtract_collateral(receipt_signer_address, signed_receipt.message.value)
            .is_err()
        {
            return Err(ReceiptError::SubtractCollateralFailed);
        }

        Ok(())
    }

    pub fn check_rav_signature(
        &self,
        signed_rav: &EIP712SignedMessage<ReceiptAggregateVoucher>,
    ) -> Result<()> {
        let rav_signer_address = signed_rav.recover_signer()?;
        if !self
            .receipt_checks_adapter
            .is_valid_gateway_id(rav_signer_address)
        {
            return Err(Error::InvalidRecoveredSigner {
                address: rav_signer_address,
            });
        }
        Ok(())
    }
}
