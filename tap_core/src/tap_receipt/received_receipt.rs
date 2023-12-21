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

use super::{receipt_auditor::ReceiptAuditor, Receipt, ReceiptCheckResults};
use crate::{
    adapters::{escrow_adapter::EscrowAdapter, receipt_storage_adapter::StoredReceipt},
    checks::{BoxedCheck, Check, CheckingChecks},
    eip_712_signed_message::EIP712SignedMessage,
    Error, Result,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct Checking {
    /// A list of checks to be completed for the receipt, along with their current result
    pub(crate) checks: ReceiptCheckResults,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Failed;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AwaitingReserve;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reserved;

pub trait ReceiptState {}
impl ReceiptState for Checking {}
impl ReceiptState for AwaitingReserve {}
impl ReceiptState for Reserved {}
impl ReceiptState for Failed {}

#[derive(Clone)]
pub enum ReceivedReceipt {
    AwaitingReserve(ReceiptWithState<AwaitingReserve>),
    Checking(ReceiptWithState<Checking>),
    Failed(ReceiptWithState<Failed>),
    Reserved(ReceiptWithState<Reserved>),
}

pub type ResultReceipt<S> = std::result::Result<ReceiptWithState<S>, ReceiptWithState<Failed>>;

pub struct ReceiptWithId<T>
where
    T: ReceiptState,
{
    pub(crate) receipt_id: u64,
    pub(crate) receipt: ReceiptWithState<T>,
}

impl<T> From<(u64, ReceiptWithState<T>)> for ReceiptWithId<T>
where
    T: ReceiptState,
{
    fn from((receipt_id, receipt): (u64, ReceiptWithState<T>)) -> ReceiptWithId<T> {
        Self {
            receipt_id,
            receipt,
        }
    }
}

pub struct SplittedReceiptWithState {
    pub(crate) awaiting_reserve_receipts: Vec<ReceiptWithState<AwaitingReserve>>,
    pub(crate) checking_receipts: Vec<ReceiptWithId<Checking>>,
    pub(crate) failed_receipts: Vec<ReceiptWithState<Failed>>,
    pub(crate) reserved_receipts: Vec<ReceiptWithState<Reserved>>,
}

impl From<ResultReceipt<AwaitingReserve>> for ReceivedReceipt {
    fn from(val: ResultReceipt<AwaitingReserve>) -> Self {
        match val {
            Ok(checked) => ReceivedReceipt::AwaitingReserve(checked),
            Err(failed) => ReceivedReceipt::Failed(failed),
        }
    }
}

impl From<ReceiptWithState<AwaitingReserve>> for ReceivedReceipt {
    fn from(val: ReceiptWithState<AwaitingReserve>) -> Self {
        ReceivedReceipt::AwaitingReserve(val)
    }
}

impl From<ReceiptWithState<Checking>> for ReceivedReceipt {
    fn from(val: ReceiptWithState<Checking>) -> Self {
        ReceivedReceipt::Checking(val)
    }
}

impl From<ReceiptWithState<Failed>> for ReceivedReceipt {
    fn from(val: ReceiptWithState<Failed>) -> Self {
        ReceivedReceipt::Failed(val)
    }
}

impl From<ReceiptWithState<Reserved>> for ReceivedReceipt {
    fn from(val: ReceiptWithState<Reserved>) -> Self {
        ReceivedReceipt::Reserved(val)
    }
}

impl From<Vec<StoredReceipt>> for SplittedReceiptWithState {
    fn from(value: Vec<StoredReceipt>) -> Self {
        let mut awaiting_reserve_receipts = Vec::new();
        let mut checking_receipts = Vec::new();
        let mut failed_receipts = Vec::new();
        let mut reserved_receipts = Vec::new();

        for stored_receipt in value {
            let StoredReceipt {
                receipt_id,
                receipt,
            } = stored_receipt;
            match receipt {
                ReceivedReceipt::AwaitingReserve(checked) => {
                    awaiting_reserve_receipts.push(checked)
                }
                ReceivedReceipt::Checking(checking) => {
                    checking_receipts.push((receipt_id, checking).into())
                }
                ReceivedReceipt::Failed(failed) => failed_receipts.push(failed),
                ReceivedReceipt::Reserved(reserved) => reserved_receipts.push(reserved),
            }
        }
        Self {
            awaiting_reserve_receipts,
            checking_receipts,
            failed_receipts,
            reserved_receipts,
        }
    }
}

impl ReceivedReceipt {
    /// Initialize a new received receipt with provided signed receipt, query id, and checks
    pub fn new(
        signed_receipt: EIP712SignedMessage<Receipt>,
        query_id: u64,
        required_checks: &[BoxedCheck],
    ) -> Self {
        let checks = ReceiptWithState::get_empty_required_checks_hashmap(required_checks);

        let received_receipt = ReceiptWithState {
            signed_receipt,
            query_id,
            state: Checking { checks },
        };
        received_receipt.into()
    }
    pub fn signed_receipt(&self) -> &EIP712SignedMessage<Receipt> {
        match self {
            ReceivedReceipt::AwaitingReserve(ReceiptWithState { signed_receipt, .. })
            | ReceivedReceipt::Checking(ReceiptWithState { signed_receipt, .. })
            | ReceivedReceipt::Failed(ReceiptWithState { signed_receipt, .. })
            | ReceivedReceipt::Reserved(ReceiptWithState { signed_receipt, .. }) => signed_receipt,
        }
    }

    pub fn query_id(&self) -> u64 {
        match self {
            ReceivedReceipt::AwaitingReserve(ReceiptWithState { query_id, .. })
            | ReceivedReceipt::Checking(ReceiptWithState { query_id, .. })
            | ReceivedReceipt::Failed(ReceiptWithState { query_id, .. })
            | ReceivedReceipt::Reserved(ReceiptWithState { query_id, .. }) => *query_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Wrapper class for metadata and state of a received receipt
pub struct ReceiptWithState<S>
where
    S: ReceiptState,
{
    /// An EIP712 signed receipt message
    pub(crate) signed_receipt: EIP712SignedMessage<Receipt>,
    /// A unique identifier for the query associated with the receipt
    pub(crate) query_id: u64,
    /// The current state of the receipt (e.g., received, checking, failed, accepted, etc.)
    pub(crate) state: S,
}

impl ReceiptWithState<AwaitingReserve> {
    pub async fn check_and_reserve_escrow<EA>(
        self,
        auditor: &ReceiptAuditor<EA>,
    ) -> ResultReceipt<Reserved>
    where
        EA: EscrowAdapter,
    {
        match auditor.check_and_reserve_escrow(&self).await {
            Ok(_) => Ok(self.perform_state_changes(Reserved)),
            Err(_) => Err(self.perform_state_changes(Failed)),
        }
    }
}

impl ReceiptWithState<Checking> {
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
    pub async fn perform_check(
        &mut self,
        check: &BoxedCheck,
    ) {
        // Only perform check if it is incomplete
        // Don't check if already failed
        if !self.check_is_complete(check) && !self.any_check_resulted_in_error() {
            let execute_check = self.state.checks.remove(check.typetag_name()).unwrap();
            let _ = self.update_check(check, execute_check.execute(&self).await);
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
    pub async fn perform_checks(
        &mut self,
        checks: &[BoxedCheck],
    ) {
        for check in checks {
            self.perform_check(check).await;
        }
    }

    /// Completes all remaining checks and stores the results
    ///
    /// Returns `Err` only if unable to complete a check, returns `Ok` if no check failed to complete (*Important:* this is not the result of the check, just the result of _completing_ the check)
    ///
    pub async fn finalize_receipt_checks(
        mut self,
    ) -> ResultReceipt<AwaitingReserve> {
        let incomplete_checks = self.incomplete_checks();

        self.perform_checks(incomplete_checks.as_slice())
            .await;

        if self.any_check_resulted_in_error() {
            let failed = self.perform_state_changes(Failed);
            Err(failed)
        } else {
            let checked = self.perform_state_changes(AwaitingReserve);
            Ok(checked)
        }
    }

    /// Returns all checks that completed with errors
    pub fn completed_checks_with_errors(&self) -> ReceiptCheckResults {
        self.state
            .checks
            .iter()
            .filter_map(|(check, result)| {
                if let CheckingChecks::Executed(unwrapped_result) = result {
                    if unwrapped_result.is_err() {
                        return Some(((*check).clone(), result.clone()));
                    }
                }
                None
            })
            .collect()
    }

    /// Returns all checks that have not been completed
    pub fn incomplete_checks(&self) -> Vec<BoxedCheck> {
        let incomplete_checks = self
            .state
            .checks
            .iter()
            .filter_map(|(check, result)| match result {
                CheckingChecks::Pending(check) => Some(check.clone()),
                CheckingChecks::Executed(_) => None,
            })
            .collect();
        incomplete_checks
    }

    pub(crate) fn update_check(
        &mut self,
        check: &BoxedCheck,
        result: CheckingChecks,
    ) -> Result<()> {
        if !self.state.checks.contains_key(check.typetag_name()) {
            return Err(Error::InvalidCheckError {
                check_string: check.typetag_name().to_string(),
            });
        }

        self.state.checks.insert(check.typetag_name(), result);
        Ok(())
    }

    /// returns true `check` has a result, otherwise false
    pub(crate) fn check_is_complete(&self, check: &BoxedCheck) -> bool {
        matches!(
            self.state.checks.get(check.typetag_name()),
            Some(CheckingChecks::Executed(_))
        )
    }

    fn any_check_resulted_in_error(&self) -> bool {
        self.state.checks.iter().any(|(_, status)| match &status {
            CheckingChecks::Executed(result) => result.is_err(),
            _ => false,
        })
    }

    pub fn checking_is_complete(&self) -> bool {
        self.state
            .checks
            .iter()
            .all(|(_, status)| matches!(status, CheckingChecks::Executed(_)))
    }

    fn get_empty_required_checks_hashmap(required_checks: &[BoxedCheck]) -> ReceiptCheckResults {
        required_checks
            .iter()
            .map(|check| (check.typetag_name(), CheckingChecks::Pending(check.clone())))
            .collect()
    }
}

impl<S> ReceiptWithState<S>
where
    S: ReceiptState,
{
    fn perform_state_changes<T>(self, new_state: T) -> ReceiptWithState<T>
    where
        T: ReceiptState,
    {
        ReceiptWithState {
            signed_receipt: self.signed_receipt,
            query_id: self.query_id,
            state: new_state,
        }
    }

    pub fn signed_receipt(&self) -> &EIP712SignedMessage<Receipt> {
        &self.signed_receipt
    }

    pub fn query_id(&self) -> u64 {
        self.query_id
    }
}
