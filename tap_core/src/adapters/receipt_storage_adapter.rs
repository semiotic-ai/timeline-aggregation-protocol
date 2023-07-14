// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeBounds;

use crate::tap_receipt::ReceivedReceipt;

/// `ReceiptStorageAdapter` defines a trait for storage adapters to manage `ReceivedReceipt` data.
///
/// This trait is designed to be implemented by users of this library who want to
/// customize the storage behavior of `ReceivedReceipt` data. The error handling is also
/// customizable by defining an `AdapterError` type, which must implement both `Error`
/// and `Debug` from the standard library.
///
/// # Usage
///
/// The `store_receipt` method should be used to store a new `ReceivedReceipt` in the storage
/// managed by the adapter. It returns a unique receipt_id associated with the stored receipt.
/// Any errors during this operation should be captured and returned in the `AdapterError` format.
///
/// The `retrieve_receipts_in_timestamp_range` method should be implemented to fetch all `ReceivedReceipts`
/// within a specific timestamp range from the storage. The returned receipts should be in the form of a vector
/// of tuples where each tuple contains the unique receipt_id and the corresponding `ReceivedReceipt`.
///
/// The `update_receipt_by_id` method is designed to update a specific `ReceivedReceipt` identified by a unique
/// receipt_id. Any errors during this operation should be captured and returned as an `AdapterError`.
///
/// The `remove_receipts_in_timestamp_range` method is used to remove all `ReceivedReceipts` within a specific
/// timestamp range from the storage. Any errors during this operation should be captured and returned as an `AdapterError`.
///
/// This trait is utilized by [crate::tap_manager], which relies on these
/// operations for working with `ReceivedReceipt` data.
///
/// # Example
///
/// For example code see [crate::adapters::receipt_storage_adapter_mock]

pub trait ReceiptStorageAdapter {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Stores a new `ReceivedReceipt` into the storage.
    ///
    /// This method should be implemented to store a new `ReceivedReceipt` into your chosen storage system.
    /// It returns a unique receipt_id associated with the stored receipt. Any errors that occur during
    /// this process should be captured and returned as an `AdapterError`.
    fn store_receipt(&mut self, receipt: ReceivedReceipt) -> Result<u64, Self::AdapterError>;

    /// Retrieves all `ReceivedReceipts` within a specific timestamp range.
    ///
    /// This method should be implemented to fetch all `ReceivedReceipts` within a specific timestamp range
    /// from your storage system. The returned receipts should be in the form of a vector of tuples where
    /// each tuple contains the unique receipt_id and the corresponding `ReceivedReceipt`.
    /// Any errors that occur during this process should be captured and returned as an `AdapterError`.
    fn retrieve_receipts_in_timestamp_range<R: RangeBounds<u64>>(
        &self,
        timestamp_range_ns: R,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, Self::AdapterError>;

    /// Updates a specific `ReceivedReceipt` identified by a unique receipt_id.
    ///
    /// This method should be implemented to update a specific `ReceivedReceipt` identified by a unique
    /// receipt_id in your storage system. Any errors that occur during this process should be captured
    /// and returned as an `AdapterError`.
    fn update_receipt_by_id(
        &mut self,
        receipt_id: u64,
        receipt: ReceivedReceipt,
    ) -> Result<(), Self::AdapterError>;

    /// Removes all `ReceivedReceipts` within a specific timestamp range from the storage.
    ///
    /// This method should be implemented to remove all `ReceivedReceipts` within a specific timestamp
    /// range from your storage system. Any errors that occur during this process should be captured and
    /// returned as an `AdapterError`.
    fn remove_receipts_in_timestamp_range<R: RangeBounds<u64>>(
        &mut self,
        timestamp_ns: R,
    ) -> Result<(), Self::AdapterError>;
}
