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

use alloy_sol_types::Eip712Domain;

use super::{Receipt, ReceiptError, ReceiptResult, SignedReceipt};
use crate::{
    manager::adapters::EscrowHandler, receipt::checks::ReceiptCheck,
    signed_message::EIP712SignedMessage,
};

#[derive(Debug, Clone)]
pub struct Checking;

#[derive(Debug, Clone)]
pub struct Failed {
    /// A list of checks to be completed for the receipt, along with their current result
    pub error: ReceiptError,
}

#[derive(Debug, Clone)]
pub struct AwaitingReserve;

#[derive(Debug, Clone)]
pub struct Reserved;

pub trait ReceiptState {}
impl ReceiptState for Checking {}
impl ReceiptState for AwaitingReserve {}
impl ReceiptState for Reserved {}
impl ReceiptState for Failed {}

pub type ResultReceipt<S> = std::result::Result<ReceiptWithState<S>, ReceiptWithState<Failed>>;

#[derive(Debug, Clone)]
/// Wrapper class for metadata and state of a received receipt
pub struct ReceiptWithState<S>
where
    S: ReceiptState,
{
    /// An EIP712 signed receipt message
    pub(crate) signed_receipt: EIP712SignedMessage<Receipt>,
    /// The current state of the receipt (e.g., received, checking, failed, accepted, etc.)
    pub(crate) _state: S,
}

impl ReceiptWithState<AwaitingReserve> {
    pub async fn check_and_reserve_escrow<E>(
        self,
        auditor: &E,
        domain_separator: &Eip712Domain,
    ) -> ResultReceipt<Reserved>
    where
        E: EscrowHandler,
    {
        match auditor
            .check_and_reserve_escrow(&self, domain_separator)
            .await
        {
            Ok(_) => Ok(self.perform_state_changes(Reserved)),
            Err(e) => Err(self.perform_state_error(e)),
        }
    }
}

impl ReceiptWithState<Checking> {
    pub fn new(signed_receipt: SignedReceipt) -> ReceiptWithState<Checking> {
        ReceiptWithState {
            signed_receipt,
            _state: Checking,
        }
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
    pub async fn perform_checks(&mut self, checks: &[ReceiptCheck]) -> ReceiptResult<()> {
        for check in checks {
            // return early on an error
            check
                .check(self)
                .await
                .map_err(|e| ReceiptError::CheckFailedToComplete(e.to_string()))?;
        }
        Ok(())
    }

    /// Completes all remaining checks and stores the results
    ///
    /// Returns `Err` only if unable to complete a check, returns `Ok` if no check failed to complete (*Important:* this is not the result of the check, just the result of _completing_ the check)
    ///
    pub async fn finalize_receipt_checks(
        mut self,
        checks: &[ReceiptCheck],
    ) -> ResultReceipt<AwaitingReserve> {
        let all_checks_passed = self.perform_checks(checks).await;

        if let Err(e) = all_checks_passed {
            Err(self.perform_state_error(e))
        } else {
            let checked = self.perform_state_changes(AwaitingReserve);
            Ok(checked)
        }
    }
}

impl<S> ReceiptWithState<S>
where
    S: ReceiptState,
{
    pub(super) fn perform_state_error(self, error: ReceiptError) -> ReceiptWithState<Failed> {
        ReceiptWithState {
            signed_receipt: self.signed_receipt,
            _state: Failed { error },
        }
    }

    fn perform_state_changes<T>(self, new_state: T) -> ReceiptWithState<T>
    where
        T: ReceiptState,
    {
        ReceiptWithState {
            signed_receipt: self.signed_receipt,
            _state: new_state,
        }
    }

    pub fn signed_receipt(&self) -> &EIP712SignedMessage<Receipt> {
        &self.signed_receipt
    }
}
