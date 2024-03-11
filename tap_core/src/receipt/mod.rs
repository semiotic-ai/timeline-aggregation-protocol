// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod checks;
mod error;
mod receipt_sol;
mod received_receipt;

pub use error::ReceiptError;
pub use receipt_sol::Receipt;
pub use received_receipt::{
    AwaitingReserve, Checking, Failed, ReceiptState, ReceiptWithState, Reserved, ResultReceipt,
};

use crate::signed_message::EIP712SignedMessage;

pub type SignedReceipt = EIP712SignedMessage<Receipt>;
pub type ReceiptResult<T> = Result<T, ReceiptError>;
