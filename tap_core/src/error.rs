// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing EIP712 message and signature
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
        "Unexpected check: {check_string}, only checks provided in initial checklist are valid"
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
    #[error("Error from adapter: {source_error_message}")]
    AdapterError { source_error_message: String },
    #[error("Failed to produce rav request, no valid receipts")]
    NoValidReceiptsForRAVRequest,
}

pub type Result<T> = StdResult<T, Error>;
