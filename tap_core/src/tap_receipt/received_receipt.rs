// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt with metadata for tracking receipt throught its lifecycle
//!
//! This module contains the `ReceivedReceipt` struct and associated enums `ReceiptState` and
//! `RAVStatus`. The `ReceivedReceipt` struct is a wrapper class for a signed receipt, which
//! includes metadata and state information for tracking the progress of a received receipt
//! throughout its lifecycle. `ReceiptState` represents the different states a receipt can be in,
//! while `RAVStatus` defines the status of a receipt with respect to its inclusion in RAV requests
//! and received RAVs.
//!
//! This module is useful for managing and tracking the state of received receipts, as well as
//! their progress through various checks and stages of inclusion in RAV requests and received RAVs.

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use super::{
    receipt_auditor::ReceiptAuditor, Receipt, ReceiptCheck, ReceiptCheckResults, ReceiptError,
};
use crate::{
    adapters::{escrow_adapter::EscrowAdapter, receipt_checks_adapter::ReceiptChecksAdapter},
    eip_712_signed_message::EIP712SignedMessage,
    Error, Result,
};

#[derive(Eq, PartialEq, Debug, Clone, EnumString, Display, Serialize, Deserialize)]
/// State of the contained receipt
pub enum ReceiptState {
    /// Initial state, received with no checks started
    Received,
    /// Checking in progress, no errors found
    Checking,
    /// Checks completed with at least one check resulting in an error
    Failed,
    /// Checks completed with all passed, awaiting escrow check and reserve
    AwaitingReserveEscrow,
    /// All checks completed with no errors found, escrow is reserved if requested by user
    Accepted,
    /// Receipt was added to a RAV request
    IncludedInRAVRequest,
    /// Receipt was included in received RAV
    Complete,
}

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]

/// Status of receipt relating to RAV inclusion
pub enum RAVStatus {
    /// Has not been included in a RAV request or received RAV
    NotIncluded,
    /// Has been added to a RAV request, but not a received RAV (awaiting a response)
    IncludedInRequest,
    /// A RAV has been received that included this receipt
    IncludedInReceived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Wrapper class for metadata and state of a received receipt
pub struct ReceivedReceipt {
    /// An EIP712 signed receipt message
    pub(crate) signed_receipt: EIP712SignedMessage<Receipt>,
    /// A unique identifier for the query associated with the receipt
    pub(crate) query_id: u64,
    /// A list of checks to be completed for the receipt, along with their current result
    pub(crate) checks: ReceiptCheckResults,
    /// Escrow check and reserve, which is performed only after all other checks are complete. `Ok` result means escrow was reserved
    pub(crate) escrow_reserved: Option<Option<std::result::Result<(), ReceiptError>>>,
    /// The current RAV status of the receipt (e.g., not included, included in a request, or included in a received RAV)
    pub(crate) rav_status: RAVStatus,
    /// The current state of the receipt (e.g., received, checking, failed, accepted, etc.)
    pub(crate) state: ReceiptState,
}

impl ReceivedReceipt {
    /// Initialize a new received receipt with provided signed receipt, query id, and checks
    pub fn new(
        signed_receipt: EIP712SignedMessage<Receipt>,
        query_id: u64,
        required_checks: &[ReceiptCheck],
    ) -> Self {
        let mut checks = Self::get_empty_required_checks_hashmap(required_checks);
        let escrow_reserved = checks.remove(&ReceiptCheck::CheckAndReserveEscrow);

        let mut received_receipt = Self {
            signed_receipt,
            query_id,
            checks,
            escrow_reserved,
            rav_status: RAVStatus::NotIncluded,
            state: ReceiptState::Received,
        };
        received_receipt.update_state();
        received_receipt
    }

    /// Completes a single *incomplete* check and stores the result, *if the check already has a result it is skipped.*
    ///
    /// Returns `Err` only if unable to complete the check, returns `Ok` if the check was completed (*Important:* this is not the result of the check, just the result of _completing_ the check)
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidStateForRequestedAction`] if the requested check cannot be comleted in the receipts current internal state. All other checks must be complete before `CheckAndReserveEscrow`.
    ///
    /// Returns [`Error::InvalidCheckError] if requested error in not a required check (list of required checks provided by user on construction)
    ///
    pub async fn perform_check<CA: EscrowAdapter, RCA: ReceiptChecksAdapter>(
        &mut self,
        check: &ReceiptCheck,
        receipt_id: u64,
        receipt_auditor: &ReceiptAuditor<CA, RCA>,
    ) -> crate::Result<()> {
        match self.state {
            ReceiptState::Checking | ReceiptState::Received => {
                // Cannot do escrow check and reserve until all other checks are complete
                if check == &ReceiptCheck::CheckAndReserveEscrow {
                    return Err(crate::Error::InvalidStateForRequestedAction {
                        state: self.state.to_string(),
                    });
                }
            }
            // All checks are valid in this state (although complete ones will be skipped)
            ReceiptState::AwaitingReserveEscrow => {}

            // If all checks are complete then checking is skipped
            ReceiptState::Accepted
            | ReceiptState::Complete
            | ReceiptState::IncludedInRAVRequest
            | ReceiptState::Failed => return Ok(()),
        }

        // All skipped checks return `Ok`
        let mut result = Ok(());
        // Only perform check if it is incomplete
        if !self.check_is_complete(check) {
            result = self.update_check(
                check,
                Some(
                    receipt_auditor
                        .check(check, &self.signed_receipt, self.query_id, receipt_id)
                        .await,
                ),
            );
        }
        self.update_state();
        result
    }

    pub async fn perform_check_batch<CA: EscrowAdapter, RCA: ReceiptChecksAdapter>(
        batch: &mut [Self],
        check: &ReceiptCheck,
        receipt_auditor: &ReceiptAuditor<CA, RCA>,
    ) -> Result<()> {
        let results = receipt_auditor.check_batch(check, batch).await;

        for (receipt, result) in batch.iter_mut().zip(results) {
            receipt.update_check(check, Some(result))?;
            receipt.update_state();
        }

        Ok(())
    }

    /// Completes a list of *incomplete* check and stores the result, if the check already has a result it is skipped
    ///
    /// Returns `Err` only if unable to complete a check, returns `Ok` if the checks were completed (*Important:* this is not the result of the check, just the result of _completing_ the check)
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidStateForRequestedAction`] if the requested check cannot be comleted in the receipts current internal state. All other checks must be complete before `CheckAndReserveEscrow`.
    ///
    /// Returns [`Error::InvalidCheckError] if requested error in not a required check (list of required checks provided by user on construction)
    ///
    pub async fn perform_checks<CA: EscrowAdapter, RCA: ReceiptChecksAdapter>(
        &mut self,
        checks: &[ReceiptCheck],
        receipt_id: u64,
        receipt_auditor: &ReceiptAuditor<CA, RCA>,
    ) -> Result<()> {
        let mut check_and_reserve_escrow_included = false;
        for check in checks {
            if *check == ReceiptCheck::CheckAndReserveEscrow {
                // if checks include check and reserve escrow it needs to be completed last
                check_and_reserve_escrow_included = true;
                continue;
            }
            self.perform_check(check, receipt_id, receipt_auditor)
                .await?;
        }
        if check_and_reserve_escrow_included && self.state != ReceiptState::Failed {
            // CheckAndReserveEscrow is only performed after all other checks have passed
            self.perform_check(
                &ReceiptCheck::CheckAndReserveEscrow,
                receipt_id,
                receipt_auditor,
            )
            .await?;
        }
        Ok(())
    }

    /// Completes all remaining checks and stores the results
    ///
    /// Returns `Err` only if unable to complete a check, returns `Ok` if no check failed to complete (*Important:* this is not the result of the check, just the result of _completing_ the check)
    ///
    pub async fn finalize_receipt_checks<CA: EscrowAdapter, RCA: ReceiptChecksAdapter>(
        &mut self,
        receipt_id: u64,
        receipt_auditor: &ReceiptAuditor<CA, RCA>,
    ) -> Result<()> {
        self.perform_checks(
            self.incomplete_checks().as_slice(),
            receipt_id,
            receipt_auditor,
        )
        .await
    }

    /// Update RAV status, should be called when receipt is included in RAV request and when RAV request is received
    pub fn update_rav_status(&mut self, rav_status: RAVStatus) {
        self.rav_status = rav_status;
        self.update_state();
    }

    pub(crate) fn update_check(
        &mut self,
        check: &ReceiptCheck,
        result: Option<super::ReceiptResult<()>>,
    ) -> Result<()> {
        if check == &ReceiptCheck::CheckAndReserveEscrow {
            return self.update_escrow_reserved_check(check, result);
        }

        if !self.checks.contains_key(check) {
            return Err(Error::InvalidCheckError {
                check_string: check.to_string(),
            });
        }

        self.checks.insert(check.clone(), result);
        Ok(())
    }

    fn update_escrow_reserved_check(
        &mut self,
        check: &ReceiptCheck,
        result: Option<super::ReceiptResult<()>>,
    ) -> Result<()> {
        if !(self.state == ReceiptState::AwaitingReserveEscrow) {
            return Err(Error::InvalidStateForRequestedAction {
                state: self.state.to_string(),
            });
        }

        if let Some(ref mut escrow_reserved_check) = self.escrow_reserved {
            *escrow_reserved_check = result;
        } else {
            return Err(crate::Error::InvalidCheckError {
                check_string: check.to_string(),
            });
        }

        self.update_state();
        Ok(())
    }

    pub fn signed_receipt(&self) -> EIP712SignedMessage<Receipt> {
        self.signed_receipt.clone()
    }

    pub fn query_id(&self) -> u64 {
        self.query_id
    }

    /// Returns all checks that have not been completed
    pub fn incomplete_checks(&self) -> Vec<ReceiptCheck> {
        let mut incomplete_checks: Vec<ReceiptCheck> = self
            .checks
            .iter()
            .filter_map(|(check, result)| {
                if result.is_none() {
                    Some((*check).clone())
                } else {
                    None
                }
            })
            .collect();
        if self.escrow_reserve_attempt_required() && !self.escrow_reserve_attempt_completed() {
            incomplete_checks.push(ReceiptCheck::CheckAndReserveEscrow);
        }
        incomplete_checks
    }

    /// Returns all checks that completed with errors
    pub fn completed_checks_with_errors(&self) -> ReceiptCheckResults {
        self.checks
            .iter()
            .filter_map(|(check, result)| {
                if let Some(unwrapped_result) = result {
                    if unwrapped_result.is_err() {
                        return Some(((*check).clone(), Some((*unwrapped_result).clone())));
                    }
                }
                None
            })
            .collect()
    }

    /// Updates receieved receipt state based on internal values, should be called anytime internal state changes
    pub(crate) fn update_state(&mut self) {
        let mut next_state = self.state.clone();
        match self.state {
            ReceiptState::Received => {
                if self.checking_is_started() {
                    next_state = self.get_state_of_checks();
                } else {
                    next_state = ReceiptState::Received;
                }
            }
            ReceiptState::Checking => {
                next_state = self.get_state_of_checks();
            }
            ReceiptState::AwaitingReserveEscrow => {
                next_state = self.get_state_of_escrow_reserve();
            }
            ReceiptState::Failed => {} // currently no next state from Failed
            ReceiptState::Accepted => {
                if self.rav_status == RAVStatus::IncludedInRequest {
                    next_state = ReceiptState::IncludedInRAVRequest;
                }
            }
            ReceiptState::IncludedInRAVRequest => {
                if self.rav_status == RAVStatus::IncludedInReceived {
                    next_state = ReceiptState::Complete;
                }
            }
            ReceiptState::Complete => {} // currently no next state from complete
        }
        self.state = next_state;
    }

    fn get_state_of_checks(&self) -> ReceiptState {
        if self.checking_is_completed() && self.any_check_resulted_in_error() {
            return ReceiptState::Failed;
        }
        if self.all_checks_passed() {
            return self.get_state_of_escrow_reserve();
        }
        if self.checking_is_in_progress() {
            return ReceiptState::Checking;
        }
        // Incase the function got called when checking was not started we can return to received state
        ReceiptState::Received
    }

    fn get_state_of_escrow_reserve(&self) -> ReceiptState {
        if !self.escrow_reserve_attempt_required() {
            return ReceiptState::Accepted;
        }
        if self.escrow_reserve_attempt_completed() {
            if let Some(Some(escrow_reserve_attempt_result)) = &self.escrow_reserved {
                if escrow_reserve_attempt_result.is_err() {
                    return ReceiptState::Failed;
                }
                if escrow_reserve_attempt_result.is_ok() {
                    return ReceiptState::Accepted;
                }
            }
        }

        ReceiptState::AwaitingReserveEscrow
    }

    pub(crate) fn escrow_reserve_attempt_completed(&self) -> bool {
        if let Some(escrow_reserve_attempt) = &self.escrow_reserved {
            return escrow_reserve_attempt.is_some();
        }
        false
    }

    pub(crate) fn escrow_reserve_attempt_required(&self) -> bool {
        self.escrow_reserved.is_some()
    }

    fn checking_is_in_progress(&self) -> bool {
        self.checking_is_started() && !self.checking_is_completed()
    }

    fn checking_is_started(&self) -> bool {
        self.checks.iter().any(|(_, status)| status.is_some())
    }

    fn checking_is_completed(&self) -> bool {
        !self.checks.iter().any(|(_, status)| status.is_none())
    }

    fn any_check_resulted_in_error(&self) -> bool {
        self.checks.iter().any(|(_, status)| match &status {
            Some(result) => result.is_err(),
            None => false,
        })
    }

    fn all_checks_passed(&self) -> bool {
        self.checking_is_completed() && !self.any_check_resulted_in_error()
    }

    /// returns true `check` has a result, otherwise false
    fn check_is_complete(&self, check: &ReceiptCheck) -> bool {
        matches!(self.checks.get(check), Some(Some(_)))
    }

    /// Returns true if all checks are complete and at least one failed
    pub fn is_failed(&self) -> bool {
        self.state == ReceiptState::Failed
    }

    /// Returns true if all checks are complete and all checks passed
    pub fn is_accepted(&self) -> bool {
        self.state == ReceiptState::Accepted
    }

    fn get_empty_required_checks_hashmap(required_checks: &[ReceiptCheck]) -> ReceiptCheckResults {
        required_checks
            .iter()
            .map(|check| (check.clone(), None))
            .collect()
    }
}
