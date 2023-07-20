// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{eip_712_signed_message::EIP712SignedMessage, tap_receipt::Receipt};
use ethereum_types::Address;

/// `ReceiptChecksAdapter` defines a trait for adapters to handle checks related to TAP receipts.
///
/// This trait is designed to be implemented by users of this library who want to
/// customize the checks done on TAP receipts. This includes ensuring the receipt is unique,
/// verifying the allocation ID, the value and the gateway ID.
///
/// # Usage
///
/// The `is_unique` method should be used to check if the given receipt is unique in the system.
///
/// The `is_valid_allocation_id` method should verify if the allocation ID is valid.
///
/// The `is_valid_value` method should confirm the value of the receipt is valid for the given query ID.
///
/// The `is_valid_gateway_id` method should confirm the gateway ID is valid.
///
/// This trait is utilized by [crate::tap_manager], which relies on these
/// operations for managing TAP receipts.
///
/// # Example
///
/// For example code see [crate::adapters::receipt_checks_adapter_mock]

pub trait ReceiptChecksAdapter {
    /// Checks if the given receipt is unique in the system.
    ///
    /// This method should be implemented to verify the uniqueness of a given receipt in your system. Keep in mind that
    /// the receipt likely will be in storage when this check is performed so the receipt id should be used to check
    /// for uniqueness.
    fn is_unique(&self, receipt: &EIP712SignedMessage<Receipt>, receipt_id: u64) -> bool;

    /// Verifies if the allocation ID is valid.
    ///
    /// This method should be implemented to validate the given allocation ID is a valid allocation for the indexer. Valid is defined as
    /// an allocation ID that is owned by the indexer and still available for redeeming.
    fn is_valid_allocation_id(&self, allocation_id: Address) -> bool;

    /// Confirms the value of the receipt is valid for the given query ID.
    ///
    /// This method should be implemented to confirm the validity of the given value for a specific query ID.
    fn is_valid_value(&self, value: u128, query_id: u64) -> bool;

    /// Confirms the gateway ID is valid.
    ///
    /// This method should be implemented to validate the given gateway ID is one associated with a gateway the indexer considers valid.
    /// The provided gateway ID is the address of the gateway that is recovered from the signature of the receipt.
    fn is_valid_gateway_id(&self, gateway_id: Address) -> bool;
}
