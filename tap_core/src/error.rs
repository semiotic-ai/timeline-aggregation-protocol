// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Error type and Result typedef
//!

use crate::receipt_aggregate_voucher::ReceiptAggregateVoucher;
use ethers::signers::WalletError;
use ethers_core::{abi::Address, types::SignatureError};
use std::result::Result as StdResult;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Aggregating receipt results in overflow")]
    AggregateOverflow,
    #[error("Failed to encode to EIP712 hash:\n{source_error_message}")]
    EIP712EncodeError { source_error_message: String },
    #[error(
        "Unexpected check: \"{check_string}\". Only checks provided in initial checklist are valid"
    )]
    InvalidCheckError { check_string: String },
    #[error("The requested action is invalid for current receipt state: {state}")]
    InvalidStateForRequestedAction { state: String },
    #[error("Failed to get current system time: {source_error_message} ")]
    InvalidSystemTime { source_error_message: String },
    #[error(transparent)]
    WalletError(#[from] WalletError),
    #[error(transparent)]
    SignatureError(#[from] SignatureError),
    #[error("Recovered gateway address invalid{address}")]
    InvalidRecoveredSigner { address: Address },
    #[error("Received RAV does not match expexted RAV")]
    InvalidReceivedRAV {
        received_rav: ReceiptAggregateVoucher,
        expected_rav: ReceiptAggregateVoucher,
    },
    #[error("Error from adapter.\n Caused by: {source_error}")]
    AdapterError { source_error: anyhow::Error },
    #[error("Failed to produce rav request, no valid receipts")]
    NoValidReceiptsForRAVRequest,
    #[error("Previous RAV allocation id ({prev_id}) doesn't match the allocation id from the new receipt ({new_id}).")]
    RavAllocationIdMismatch { prev_id: String, new_id: String },
    #[error("All receipts should have the same allocation id, but they don't")]
    RavAllocationIdNotUniform,
    #[error("Duplicate receipt signature: {0}")]
    DuplicateReceiptSignature(String),
    #[error(
        "Receipt timestamp ({receipt_ts}) is less or equal than previous rav timestamp ({rav_ts})"
    )]
    ReceiptTimestampLowerThanRav { rav_ts: u64, receipt_ts: u64 },
    #[error("Timestamp range error: min_timestamp_ns: {min_timestamp_ns}, max_timestamp_ns: {max_timestamp_ns}. Adjust timestamp buffer.")]
    TimestampRangeError {
        min_timestamp_ns: u64,
        max_timestamp_ns: u64,
    },
}

pub type Result<T> = StdResult<T, Error>;
