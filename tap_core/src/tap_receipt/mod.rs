// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

mod receipt;
mod received_receipt;
use std::collections::HashMap;

use ethereum_types::Address;
pub use receipt::Receipt;
pub use received_receipt::ReceivedReceipt;
use strum_macros::{Display, EnumString};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ReceiptError {
    #[error("invalid allocation ID: {received_allocation_id} (valid {expected_allocation_ids})")]
    InvalidAllocationID {
        received_allocation_id: Address,
        expected_allocation_ids: String,
    },
    #[error("Signature check failed:\n{source_error_message}")]
    InvalidSignature { source_error_message: String },
    #[error("invalid timestamp: {received_timestamp} (expected range [{timestamp_min}, {timestamp_max}) )")]
    InvalidTimestamp {
        received_timestamp: u64,
        timestamp_min: u64,
        timestamp_max: u64,
    },
    #[error("Invalid Value: {received_value} (expected {expected_value})")]
    InvalidValue {
        received_value: u128,
        expected_value: u128,
    },
}

pub type ReceiptResult<T> = Result<T, ReceiptError>;
pub type ReceiptCheckResults = HashMap<ReceiptCheck, Option<ReceiptResult<()>>>;
#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumString, Display)]
pub enum ReceiptCheck {
    CheckUnique,
    CheckAllocationId,
    CheckTimestamp,
    CheckValue,
    CheckSignature,
    CheckCollateralAvailable,
}

pub fn get_full_list_of_checks() -> ReceiptCheckResults {
    let mut all_checks_list = ReceiptCheckResults::new();
    all_checks_list.insert(ReceiptCheck::CheckUnique, None);
    all_checks_list.insert(ReceiptCheck::CheckAllocationId, None);
    all_checks_list.insert(ReceiptCheck::CheckTimestamp, None);
    all_checks_list.insert(ReceiptCheck::CheckValue, None);
    all_checks_list.insert(ReceiptCheck::CheckSignature, None);
    all_checks_list.insert(ReceiptCheck::CheckCollateralAvailable, None);

    all_checks_list
}

#[cfg(test)]
pub mod tests;
