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

use alloy::dyn_abi::Eip712Domain;

use super::{Receipt, ReceiptError, ReceiptResult, SignedReceipt};
use crate::receipt::state::{AwaitingReserve, Checking, Failed, ReceiptState, Reserved};
use crate::{
    manager::adapters::EscrowHandler, receipt::checks::ReceiptCheck,
    signed_message::EIP712SignedMessage,
};

pub type ResultReceipt<S> = std::result::Result<ReceiptWithState<S>, ReceiptWithState<Failed>>;

/// Typestate pattern for tracking the state of a receipt
///
/// - The [ `ReceiptState` ] trait represents the different states a receipt
/// can be in.
/// - The [ `Checking` ] state is used to represent a receipt that is currently
/// being checked.
/// - The [ `Failed` ] state is used to represent a receipt that has failed a
/// check or validation.
/// - The [ `AwaitingReserve` ] state is used to represent a receipt that has
/// passed all checks and is
/// awaiting escrow reservation.
/// - The [ `Reserved` ] state is used to represent a receipt that has
/// successfully reserved escrow.
#[derive(Debug, Clone)]
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
    /// Perform the checks implemented by the context and reserve escrow if
    /// all checks pass
    ///
    /// Returns a [`ReceiptWithState<Reserved>`] if successful, otherwise
    /// returns a [`ReceiptWithState<Failed>`]
    pub async fn check_and_reserve_escrow<E>(
        self,
        context: &E,
        domain_separator: &Eip712Domain,
    ) -> ResultReceipt<Reserved>
    where
        E: EscrowHandler,
    {
        match context
            .check_and_reserve_escrow(&self, domain_separator)
            .await
        {
            Ok(_) => Ok(self.perform_state_changes(Reserved)),
            Err(e) => Err(self.perform_state_error(e)),
        }
    }
}

impl ReceiptWithState<Checking> {
    /// Creates a new `ReceiptWithState` in the `Checking` state
    pub fn new(signed_receipt: SignedReceipt) -> ReceiptWithState<Checking> {
        ReceiptWithState {
            signed_receipt,
            _state: Checking,
        }
    }

    /// Performs a list of checks on the receipt
    ///
    /// # Errors
    ///
    /// Returns [`ReceiptError::CheckFailedToComplete`] if the requested check
    /// cannot be comleted in the receipts current internal state.
    /// All other checks must be complete before `CheckAndReserveEscrow`.
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

    /// Completes all checks and transitions the receipt to the next state
    ///
    /// Returns `Err` with a [`ReceiptWithState<Failed>`] in case of error,
    /// returns `Ok` with a [`ReceiptWithState<AwaitingReserve>`] in case of success.
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

    /// Returns the signed receipt
    pub fn signed_receipt(&self) -> &EIP712SignedMessage<Receipt> {
        &self.signed_receipt
    }
}
