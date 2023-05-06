// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{SignedRAV, SignedReceipt};
use crate::{
    adapters::{
        collateral_adapter::CollateralAdapter, rav_storage_adapter::RAVStorageAdapter,
        receipt_checks_adapter::ReceiptChecksAdapter,
        receipt_storage_adapter::ReceiptStorageAdapter,
    },
    receipt_aggregate_voucher::ReceiptAggregateVoucher,
    tap_receipt::{ReceiptAuditor, ReceiptCheck, ReceivedReceipt},
    Error,
};

pub struct Manager<
    CA: CollateralAdapter,
    RCA: ReceiptChecksAdapter,
    RSA: ReceiptStorageAdapter,
    RAVSA: RAVStorageAdapter,
> {
    rav_storage_adapter: RAVSA,
    receipt_storage_adapter: RSA,
    required_checks: Vec<ReceiptCheck>,
    current_rav_id: Option<u64>,
    receipt_auditor: ReceiptAuditor<CA, RCA>,
}

impl<
        CA: CollateralAdapter,
        RCA: ReceiptChecksAdapter,
        RSA: ReceiptStorageAdapter,
        RAVSA: RAVStorageAdapter,
    > Manager<CA, RCA, RSA, RAVSA>
{
    pub fn new(
        collateral_adapter: CA,
        receipt_checks_adapter: RCA,
        rav_storage_adapter: RAVSA,
        receipt_storage_adapter: RSA,
        required_checks: Vec<ReceiptCheck>,
        starting_min_timestamp_ns: u64,
    ) -> Self {
        let receipt_auditor = ReceiptAuditor::new(
            collateral_adapter,
            receipt_checks_adapter,
            starting_min_timestamp_ns,
        );
        Self {
            rav_storage_adapter,
            receipt_storage_adapter,
            required_checks,
            current_rav_id: None,
            receipt_auditor,
        }
    }

    pub fn verify_and_store_receipt(
        &mut self,
        signed_receipt: SignedReceipt,
        query_id: u64,
        initial_checks: Vec<ReceiptCheck>,
    ) -> std::result::Result<(), Error> {
        let mut received_receipt =
            ReceivedReceipt::new(signed_receipt, query_id, &self.required_checks);
        let receipt_id = self
            .receipt_storage_adapter
            .store_receipt(received_receipt.clone())
            .map_err(|err| Error::AdapterError {
                source_error_message: err.to_string(),
            })?;
        received_receipt.perform_checks(
            initial_checks.as_slice(),
            receipt_id,
            &mut self.receipt_auditor,
        )?;
        self.receipt_storage_adapter
            .update_receipt_by_id(receipt_id, received_receipt)
            .map_err(|err| Error::AdapterError {
                source_error_message: err.to_string(),
            })?;
        Ok(())
    }

    pub fn verify_and_store_rav(
        &mut self,
        expected_rav: ReceiptAggregateVoucher,
        signed_rav: SignedRAV,
    ) -> std::result::Result<(), Error> {
        self.receipt_auditor.check_rav_signature(&signed_rav)?;

        if signed_rav.message != expected_rav {
            return Err(Error::InvalidReceivedRAV {
                received_rav: signed_rav.message,
                expected_rav,
            });
        }

        self.rav_storage_adapter
            .store_rav(signed_rav)
            .map_err(|err| Error::AdapterError {
                source_error_message: err.to_string(),
            })?;

        Ok(())
    }

    // TODO: create a rav request struct for return (receipts for rav request, failed receipt, expected rav)
    pub fn create_rav_request(
        &mut self,
        timestamp_buffer_ns: u64,
    ) -> Result<
        (
            Vec<SignedReceipt>,
            Vec<SignedReceipt>,
            ReceiptAggregateVoucher,
        ),
        Error,
    > {
        // get previous rav
        let mut previous_rav: Option<SignedRAV> = None;
        if let Some(current_rav_id) = self.current_rav_id {
            let stored_previous_rav = self
                .rav_storage_adapter
                .retrieve_rav_by_id(current_rav_id)
                .map_err(|err| Error::AdapterError {
                    source_error_message: err.to_string(),
                })?;
            previous_rav = Some(stored_previous_rav);
        }
        let (valid_receipts, invalid_receipts) = self.collect_receipts(timestamp_buffer_ns)?;

        let expected_rav = Self::generate_expected_rav(&valid_receipts, previous_rav)?;

        self.receipt_auditor
            .update_min_timestamp_ns(expected_rav.timestamp_ns + 1);

        Ok((valid_receipts, invalid_receipts, expected_rav))
    }

    fn collect_receipts(
        &mut self,
        timestamp_buffer_ns: u64,
    ) -> Result<(Vec<SignedReceipt>, Vec<SignedReceipt>), Error> {
        let cutoff_timestamp = crate::get_current_timestamp_u64_ns()? - timestamp_buffer_ns;
        let received_receipts = self
            .receipt_storage_adapter
            .retrieve_receipts_upto_timestamp(cutoff_timestamp)
            .map_err(|err| Error::AdapterError {
                source_error_message: err.to_string(),
            })?;

        let mut accepted_signed_receipts = Vec::<SignedReceipt>::new();
        let mut failed_signed_receipts = Vec::<SignedReceipt>::new();

        for (receipt_id, mut received_receipt) in received_receipts {
            received_receipt.finalize_receipt_checks(receipt_id, &mut self.receipt_auditor)?;
            if received_receipt.is_accepted() {
                accepted_signed_receipts.push(received_receipt.signed_receipt);
            } else {
                failed_signed_receipts.push(received_receipt.signed_receipt);
            }
        }

        Ok((accepted_signed_receipts, failed_signed_receipts))
    }

    fn generate_expected_rav(
        receipts: &[SignedReceipt],
        previous_rav: Option<SignedRAV>,
    ) -> Result<ReceiptAggregateVoucher, Error> {
        if receipts.is_empty() {
            return Err(Error::NoValidReceiptsForRAVRequest);
        }
        let allocation_id = receipts[0].message.allocation_id;
        ReceiptAggregateVoucher::aggregate_receipts(allocation_id, receipts, previous_rav)
    }
}
