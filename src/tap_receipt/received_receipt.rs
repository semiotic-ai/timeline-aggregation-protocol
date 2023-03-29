// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{Receipt, ReceiptCheck, ReceiptCheckResults};
use crate::eip_712_signed_message::EIP712SignedMessage;
use strum_macros::{Display, EnumString};

#[derive(Eq, PartialEq, Debug, Clone, EnumString, Display)]
pub enum ReceiptState {
    Received,
    Checking,
    Failed,
    Accepted,
    IncludedInRAVRequest,
    Complete,
}

#[derive(Eq, PartialEq, Debug, Clone)]

pub enum RAVStatus {
    NotIncluded,
    IncludedInRequest,
    IncludedInReceived,
}

pub struct ReceivedReceipt {
    pub(crate) signed_receipt: EIP712SignedMessage<Receipt>,
    pub(crate) query_id: u64,
    pub(crate) checks: ReceiptCheckResults,
    pub(crate) rav_status: RAVStatus,
    pub(crate) state: ReceiptState,
}

impl ReceivedReceipt {
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

    pub fn update_rav_status(&mut self, rav_status: RAVStatus) {
        self.rav_status = rav_status;
        self.update_state();
    }

    pub fn update_check(
        &mut self,
        check: ReceiptCheck,
        result: Option<crate::Result<()>>,
    ) -> crate::Result<()> {
        assert!(self.checks.contains_key(&check));
        if !self.checks.contains_key(&check) {
            return Err(crate::Error::InvalidCheckError {
                check_string: check.to_string(),
            });
        }
        // TODO: return error if outside of valid states?
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
}
