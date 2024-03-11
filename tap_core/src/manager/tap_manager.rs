// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy_sol_types::Eip712Domain;

use super::adapters::{EscrowHandler, RAVRead, RAVStore, ReceiptDelete, ReceiptRead, ReceiptStore};
use crate::{
    rav::{RAVRequest, ReceiptAggregateVoucher, SignedRAV},
    receipt::{
        checks::{BatchTimestampCheck, CheckBatch, Checks, UniqueCheck},
        Failed, ReceiptWithState, Reserved, SignedReceipt,
    },
    Error,
};

pub struct Manager<E> {
    /// Context that implements adapters
    context: E,

    /// Checks that must be completed for each receipt before being confirmed or denied for rav request
    checks: Checks,

    /// Struct responsible for doing checks for receipt. Ownership stays with manager allowing manager
    /// to update configuration ( like minimum timestamp ).
    domain_separator: Eip712Domain,
}

impl<E> Manager<E> {
    /// Creates new manager with provided `adapters`, any receipts received by this manager
    /// will complete all `required_checks` before being accepted or declined from RAV.
    /// `starting_min_timestamp` will be used as min timestamp until the first RAV request is created.
    ///
    pub fn new(domain_separator: Eip712Domain, context: E, checks: impl Into<Checks>) -> Self {
        Self {
            context,
            domain_separator,
            checks: checks.into(),
        }
    }
}

impl<E> Manager<E>
where
    E: RAVStore + EscrowHandler,
{
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
        self.context
            .check_rav_signature(&signed_rav, &self.domain_separator)
            .await?;

        if signed_rav.message != expected_rav {
            return Err(Error::InvalidReceivedRAV {
                received_rav: signed_rav.message,
                expected_rav,
            });
        }

        self.context
            .update_last_rav(signed_rav)
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?;

        Ok(())
    }
}

impl<E> Manager<E>
where
    E: RAVRead,
{
    async fn get_previous_rav(&self) -> Result<Option<SignedRAV>, Error> {
        let previous_rav = self
            .context
            .last_rav()
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?;
        Ok(previous_rav)
    }
}

impl<E> Manager<E>
where
    E: ReceiptRead + EscrowHandler,
{
    async fn collect_receipts(
        &self,
        timestamp_buffer_ns: u64,
        min_timestamp_ns: u64,
        limit: Option<u64>,
    ) -> Result<
        (
            Vec<ReceiptWithState<Reserved>>,
            Vec<ReceiptWithState<Failed>>,
        ),
        Error,
    > {
        let max_timestamp_ns = crate::get_current_timestamp_u64_ns()? - timestamp_buffer_ns;

        if min_timestamp_ns > max_timestamp_ns {
            return Err(Error::TimestampRangeError {
                min_timestamp_ns,
                max_timestamp_ns,
            });
        }
        let checking_receipts = self
            .context
            .retrieve_receipts_in_timestamp_range(min_timestamp_ns..max_timestamp_ns, limit)
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?;

        let mut awaiting_reserve_receipts = vec![];
        let mut failed_receipts = vec![];
        let mut reserved_receipts = vec![];

        // check for timestamp
        let (checking_receipts, already_failed) =
            BatchTimestampCheck(min_timestamp_ns).check_batch(checking_receipts);
        failed_receipts.extend(already_failed);

        // check for uniqueness
        let (checking_receipts, already_failed) = UniqueCheck.check_batch(checking_receipts);
        failed_receipts.extend(already_failed);

        for receipt in checking_receipts.into_iter() {
            let receipt = receipt.finalize_receipt_checks(&self.checks).await;

            match receipt {
                Ok(checked) => awaiting_reserve_receipts.push(checked),
                Err(failed) => failed_receipts.push(failed),
            }
        }
        for checked in awaiting_reserve_receipts {
            match checked
                .check_and_reserve_escrow(&self.context, &self.domain_separator)
                .await
            {
                Ok(reserved) => reserved_receipts.push(reserved),
                Err(failed) => failed_receipts.push(failed),
            }
        }

        Ok((reserved_receipts, failed_receipts))
    }
}

impl<E> Manager<E>
where
    E: ReceiptRead + RAVRead + EscrowHandler,
{
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
    pub async fn create_rav_request(
        &self,
        timestamp_buffer_ns: u64,
        receipts_limit: Option<u64>,
    ) -> Result<RAVRequest, Error> {
        let previous_rav = self.get_previous_rav().await?;
        let min_timestamp_ns = previous_rav
            .as_ref()
            .map(|rav| rav.message.timestampNs + 1)
            .unwrap_or(0);

        let (valid_receipts, invalid_receipts) = self
            .collect_receipts(timestamp_buffer_ns, min_timestamp_ns, receipts_limit)
            .await?;

        let expected_rav = Self::generate_expected_rav(&valid_receipts, previous_rav.clone())?;

        let valid_receipts = valid_receipts
            .into_iter()
            .map(|rx_receipt| rx_receipt.signed_receipt)
            .collect::<Vec<_>>();

        Ok(RAVRequest {
            valid_receipts,
            previous_rav,
            invalid_receipts,
            expected_rav,
        })
    }

    fn generate_expected_rav(
        receipts: &[ReceiptWithState<Reserved>],
        previous_rav: Option<SignedRAV>,
    ) -> Result<ReceiptAggregateVoucher, Error> {
        if receipts.is_empty() {
            return Err(Error::NoValidReceiptsForRAVRequest);
        }
        let allocation_id = receipts[0].signed_receipt().message.allocation_id;
        let receipts = receipts
            .iter()
            .map(|rx_receipt| rx_receipt.signed_receipt().clone())
            .collect::<Vec<_>>();
        ReceiptAggregateVoucher::aggregate_receipts(
            allocation_id,
            receipts.as_slice(),
            previous_rav,
        )
    }
}

impl<E> Manager<E>
where
    E: ReceiptDelete + RAVRead,
{
    /// Removes obsolete receipts from storage. Obsolete receipts are receipts that are older than the last RAV, and
    /// therefore already aggregated into the RAV.
    /// This function should be called after a new RAV is received to limit the number of receipts stored.
    /// No-op if there is no last RAV.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AdapterError`] if there are any errors while retrieving last RAV or removing receipts
    ///
    pub async fn remove_obsolete_receipts(&self) -> Result<(), Error> {
        match self.get_previous_rav().await? {
            Some(last_rav) => {
                self.context
                    .remove_receipts_in_timestamp_range(..=last_rav.message.timestampNs)
                    .await
                    .map_err(|err| Error::AdapterError {
                        source_error: anyhow::Error::new(err),
                    })?;
                Ok(())
            }
            None => Ok(()),
        }
    }
}

impl<E> Manager<E>
where
    E: ReceiptStore,
{
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
    ) -> std::result::Result<(), Error> {
        let mut received_receipt = ReceiptWithState::new(signed_receipt);

        // perform checks
        received_receipt.perform_checks(&self.checks).await?;

        // store the receipt
        self.context
            .store_receipt(received_receipt)
            .await
            .map_err(|err| Error::AdapterError {
                source_error: anyhow::Error::new(err),
            })?;
        Ok(())
    }
}
