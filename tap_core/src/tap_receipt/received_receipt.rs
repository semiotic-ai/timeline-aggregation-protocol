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

use strum_macros::{Display, EnumString};

use super::{Receipt, ReceiptCheck, ReceiptCheckResults};
use crate::eip_712_signed_message::EIP712SignedMessage;

#[derive(Eq, PartialEq, Debug, Clone, EnumString, Display)]
/// State of the contained receipt
pub enum ReceiptState {
    /// Initial state, received with no checks started
    Received,
    /// Checking in progress, no errors found
    Checking,
    /// At least one check resulted in an error
    Failed,
    /// All checks have completed with no errors found
    Accepted,
    /// Receipt was added to a RAV request
    IncludedInRAVRequest,
    /// Receipt was included in received RAV
    Complete,
}

#[derive(Eq, PartialEq, Debug, Clone)]

/// Status of receipt relating to RAV inclusion
pub enum RAVStatus {
    /// Has not been included in a RAV request or received RAV
    NotIncluded,
    /// Has been added to a RAV request, but not a received RAV (awaiting a response)
    IncludedInRequest,
    /// A RAV has been received that included this receipt
    IncludedInReceived,
}

#[derive(Debug, Clone)]
/// Wrapper class for metadata and state of a received receipt
pub struct ReceivedReceipt {
    /// An EIP712 signed receipt message
    pub(crate) signed_receipt: EIP712SignedMessage<Receipt>,
    /// A unique identifier for the query associated with the receipt
    pub(crate) query_id: u64,
    /// A list of checks to be completed for the receipt, along with their current result
    pub(crate) checks: ReceiptCheckResults,
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
        checks: ReceiptCheckResults,
    ) -> Self {
        let mut received_receipt = Self {
            signed_receipt,
            query_id,
            checks,
            rav_status: RAVStatus::NotIncluded,
            state: ReceiptState::Received,
        };
        received_receipt.update_state();
        received_receipt
    }

    /// Update RAV status, should be called when receipt is included in RAV request and when RAV request is received
    pub fn update_rav_status(&mut self, rav_status: RAVStatus) {
        self.rav_status = rav_status;
        self.update_state();
    }

    /// Update results of an receipt check, this is only valid if there are remaining checks to resolve
    pub fn update_check(
        &mut self,
        check: ReceiptCheck,
        result: Option<super::ReceiptResult<()>>,
    ) -> crate::Result<()> {
        if !self.checks.contains_key(&check) {
            return Err(crate::Error::InvalidCheckError {
                check_string: check.to_string(),
            });
        }
        if !(self.state == ReceiptState::Received || self.state == ReceiptState::Checking) {
            return Err(crate::Error::InvalidStateForRequestedAction {
                state: self.state.to_string(),
            });
        }
        self.checks.insert(check, result);
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
    pub fn incomplete_checks(&self) -> ReceiptCheckResults {
        self.checks
            .iter()
            .filter_map(|(check, result)| {
                if result.is_none() {
                    Some(((*check).clone(), None))
                } else {
                    None
                }
            })
            .collect()
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
    fn update_state(&mut self) {
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
        if self.any_check_resulted_in_error() {
            return ReceiptState::Failed;
        }
        if self.all_checks_passed() {
            return ReceiptState::Accepted;
        }
        if self.checking_is_in_progress() {
            return ReceiptState::Checking;
        }
        // Incase the function got called when checking was not started we can return to received state
        ReceiptState::Received
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

    pub fn check_is_complete(&self, check: ReceiptCheck) -> bool {
        match self.checks.get(&check){
            Some(Some(check_result_option)) => true,
            _ => false
        }
    }
}
