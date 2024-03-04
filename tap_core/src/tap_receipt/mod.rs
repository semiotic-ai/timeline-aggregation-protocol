// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

mod receipt;
mod receipt_auditor;
mod received_receipt;
use std::{collections::HashMap, sync::{Arc, RwLock}};

use alloy_primitives::Address;
pub use receipt::Receipt;
pub use receipt_auditor::ReceiptAuditor;
pub use received_receipt::{
    AwaitingReserve, CategorizedReceiptsWithState, Checking, Failed, ReceiptState, ReceiptWithId,
    ReceiptWithState, ReceivedReceipt, Reserved, ResultReceipt,
};

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use thiserror::Error;

use crate::checks::{CheckingChecks, ReceiptCheck};

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ReceiptError {
    #[error("invalid allocation ID: {received_allocation_id}")]
    InvalidAllocationID { received_allocation_id: Address },
    #[error("Signature check failed:\n{source_error_message}")]
    InvalidSignature { source_error_message: String },
    #[error("invalid timestamp: {received_timestamp} (expected min {timestamp_min})")]
    InvalidTimestamp {
        received_timestamp: u64,
        timestamp_min: u64,
    },
    #[error("Invalid Value: {received_value} ")]
    InvalidValue { received_value: u128 },
    #[error("Receipt is not unique")]
    NonUniqueReceipt,
    #[error("Attempt to collect escrow failed")]
    SubtractEscrowFailed,
    #[error("Issue encountered while performing check: {source_error_message}")]
    CheckFailedToComplete { source_error_message: String },
}

pub type ReceiptResult<T> = Result<T, ReceiptError>;
pub type ReceiptCheckResults = HashMap<&'static str, CheckingChecks>;
// #[derive(Hash, Eq, PartialEq, Debug, Clone, EnumString, Display, Serialize, Deserialize)]
// pub enum ReceiptCheck {
//     CheckUnique,
//     CheckAllocationId,
//     CheckTimestamp,
//     CheckValue,
//     CheckSignature,
// }

pub fn get_full_list_of_receipt_check_results() -> ReceiptCheckResults {
    let all_checks_list = ReceiptCheckResults::new();
    // all_checks_list.insert(ReceiptCheck::CheckUnique, None);
    // all_checks_list.insert(ReceiptCheck::CheckAllocationId, None);
    // all_checks_list.insert(ReceiptCheck::CheckTimestamp, None);
    // all_checks_list.insert(ReceiptCheck::CheckValue, None);
    // all_checks_list.insert(ReceiptCheck::CheckSignature, None);

    all_checks_list
}

pub fn get_full_list_of_checks() -> Vec<ReceiptCheck> {
    vec![
        // ReceiptCheck::CheckUnique,
        // ReceiptCheck::CheckAllocationId,
        // ReceiptCheck::CheckTimestamp,
        // ReceiptCheck::CheckValue,
        // ReceiptCheck::CheckSignature,
    ]
}

#[cfg(test)]
pub mod tests;
