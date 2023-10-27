// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy_sol_types::Eip712Domain;

use super::{RAVRequest, SignedRAV, SignedReceipt};
use crate::{
    adapters::{
        escrow_adapter::EscrowAdapter, rav_storage_adapter::RAVStorageAdapter,
        receipt_checks_adapter::ReceiptChecksAdapter,
        receipt_storage_adapter::ReceiptStorageAdapter,
    },
    receipt_aggregate_voucher::ReceiptAggregateVoucher,
    tap_receipt::{ReceiptAuditor, ReceiptCheck, ReceivedReceipt},
    Error,
};

pub struct Manager<
    CA: EscrowAdapter,
    RCA: ReceiptChecksAdapter,
    RSA: ReceiptStorageAdapter,
    RAVSA: RAVStorageAdapter,
> {
    /// Adapter for RAV CRUD
    rav_storage_adapter: RAVSA,
    /// Adapter for receipt CRUD
    receipt_storage_adapter: RSA,
    /// Checks that must be completed for each receipt before being confirmed or denied for rav request
    required_checks: Vec<ReceiptCheck>,
    /// Struct responsible for doing checks for receipt. Ownership stays with manager allowing manager
    /// to update configuration ( like minimum timestamp ).
    receipt_auditor: ReceiptAuditor<CA, RCA>,
}

impl<
        EA: EscrowAdapter,
        RCA: ReceiptChecksAdapter,
        RSA: ReceiptStorageAdapter,
        RAVSA: RAVStorageAdapter,
    > Manager<EA, RCA, RSA, RAVSA>
{
    /// Creates new manager with provided `adapters`, any receipts received by this manager
    /// will complete all `required_checks` before being accepted or declined from RAV.
    /// `starting_min_timestamp` will be used as min timestamp until the first RAV request is created.
    ///
    pub fn new(
        domain_separator: Eip712Domain,
        escrow_adapter: EA,
        receipt_checks_adapter: RCA,
        rav_storage_adapter: RAVSA,
        receipt_storage_adapter: RSA,
        required_checks: Vec<ReceiptCheck>,
        starting_min_timestamp_ns: u64,
    ) -> Self {
        let receipt_auditor = ReceiptAuditor::new(
            domain_separator,
            escrow_adapter,
            receipt_checks_adapter,
            starting_min_timestamp_ns,
        );
        Self {
            rav_storage_adapter,
            receipt_storage_adapter,
            required_checks,
            receipt_auditor,
        }
    }

    /// Runs `initial_checks` on `signed_receipt` for initial verification, then stores received receipt.
    /// The provided `query_id` will be used as a key when chaecking query appraisal.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AdapterError`] if there are any errors while storing receipts
    ///
    /// Returns [`Error::InvalidStateForRequestedAction`] if the checks requested in `initial_checks` cannot be comleted due to: All other checks must be complete before `CheckAndReserveEscrow`
    ///
    /// Returns [`Error::InvalidCheckError`] if check in `initial_checks` is not in `required_checks` provided when manager was created
    ///
    pub async fn verify_and_store_receipt(
        &self,
        signed_receipt: SignedReceipt,
        query_id: u64,
        initial_checks: &[ReceiptCheck],
    ) -> std::result::Result<(), Error> {
        let mut received_receipt =
            ReceivedReceipt::new(signed_receipt, query_id, &self.required_checks);
        // The receipt id is needed before `perform_checks` can be called on received receipt
        // since it is needed for uniqueness check. Since the receipt_id is defined when it is stored
        // This function first stores it, then checks it, then updates what was stored.

        let receipt_id = self
            .receipt_storage_adapter
            .store_receipt(received_receipt.clone())
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?;

        received_receipt
            .perform_checks(initial_checks, receipt_id, &self.receipt_auditor)
            .await?;

        self.receipt_storage_adapter
            .update_receipt_by_id(receipt_id, received_receipt)
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?;
        Ok(())
    }

    /// Verify `signed_rav` matches all values on `expected_rav`, and that `signed_rav` has a valid signer.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AdapterError`] if there are any errors while storing RAV
    ///
    pub async fn verify_and_store_rav(
        &self,
        expected_rav: ReceiptAggregateVoucher,
        signed_rav: SignedRAV,
    ) -> std::result::Result<(), Error> {
        self.receipt_auditor
            .check_rav_signature(&signed_rav)
            .await?;

        if signed_rav.message != expected_rav {
            return Err(Error::InvalidReceivedRAV {
                received_rav: signed_rav.message,
                expected_rav,
            });
        }

        self.rav_storage_adapter
            .update_last_rav(signed_rav)
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?;

        Ok(())
    }

    /// Removes obsolete receipts from storage. Obsolete receipts are receipts that are older than the last RAV, and
    /// therefore already aggregated into the RAV.
    /// This function should be called after a new RAV is received to limit the number of receipts stored.
    /// No-op if there is no last RAV.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AdapterError`] if there are any errors while retrieving last RAV or removing receipts
    ///
    pub async fn remove_obsolete_receipts(&mut self) -> Result<(), Error> {
        match self.get_previous_rav().await? {
            Some(last_rav) => {
                self.receipt_storage_adapter
                    .remove_receipts_in_timestamp_range(..=last_rav.message.timestamp_ns)
                    .await
                    .map_err(|err| Error::AdapterError {
                        source_error: anyhow::Error::new(err),
                    })?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// Completes remaining checks on all receipts up to (current time - `timestamp_buffer_ns`). Returns them in
    /// two lists (valid receipts and invalid receipts) along with the expected RAV that should be received
    /// for aggregating list of valid receipts.
    ///
    /// Returns [`Error::AggregateOverflow`] if any receipt value causes aggregate value to overflow while generating expected RAV
    ///
    /// Returns [`Error::AdapterError`] if unable to fetch previous RAV or if unable to fetch previous receipts
    ///
    /// Returns [`Error::TimestampRangeError`] if the max timestamp of the previous RAV is greater than the min timestamp. Caused by timestamp buffer being too large, or requests coming too soon.
    ///
    pub async fn create_rav_request(&self, timestamp_buffer_ns: u64) -> Result<RAVRequest, Error> {
        let previous_rav = self.get_previous_rav().await?;
        let min_timestamp_ns = previous_rav
            .as_ref()
            .map(|rav| rav.message.timestamp_ns + 1)
            .unwrap_or(0);

        let (valid_receipts, invalid_receipts) = self
            .collect_receipts(timestamp_buffer_ns, min_timestamp_ns)
            .await?;

        let expected_rav = Self::generate_expected_rav(&valid_receipts, previous_rav.clone())?;

        self.receipt_auditor
            .update_min_timestamp_ns(expected_rav.timestamp_ns)
            .await;

        Ok(RAVRequest {
            valid_receipts,
            previous_rav,
            invalid_receipts,
            expected_rav,
        })
    }

    async fn get_previous_rav(&self) -> Result<Option<SignedRAV>, Error> {
        let previous_rav =
            self.rav_storage_adapter
                .last_rav()
                .await
                .map_err(|err| Error::AdapterError {
                    source_error: anyhow::Error::new(err),
                })?;
        Ok(previous_rav)
    }

    async fn collect_receipts(
        &self,
        timestamp_buffer_ns: u64,
        min_timestamp_ns: u64,
    ) -> Result<(Vec<SignedReceipt>, Vec<SignedReceipt>), Error> {
        let max_timestamp_ns = crate::get_current_timestamp_u64_ns()? - timestamp_buffer_ns;

        if min_timestamp_ns > max_timestamp_ns {
            return Err(Error::TimestampRangeError {
                min_timestamp_ns,
                max_timestamp_ns,
            });
        }
        let received_receipts = self
            .receipt_storage_adapter
            .retrieve_receipts_in_timestamp_range(min_timestamp_ns..max_timestamp_ns)
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?;

        let mut accepted_signed_receipts = Vec::<SignedReceipt>::new();
        let mut failed_signed_receipts = Vec::<SignedReceipt>::new();

        let mut received_receipts: Vec<ReceivedReceipt> =
            received_receipts.into_iter().map(|e| e.1).collect();

        for check in self.required_checks.iter() {
            ReceivedReceipt::perform_check_batch(
                &mut received_receipts,
                check,
                &self.receipt_auditor,
            )
            .await?;
        }

        for received_receipt in received_receipts {
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

#[cfg(test)]
#[path = "test/manager_test.rs"]
mod manager_test;
