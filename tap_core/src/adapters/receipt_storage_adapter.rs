// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeBounds;

use async_trait::async_trait;

use crate::tap_receipt::ReceivedReceipt;

/// `ReceiptStore` defines a trait for write storage adapters to manage `ReceivedReceipt` data.
///
/// This trait is designed to be implemented by users of this library who want to
/// customize the write storage behavior of `ReceivedReceipt` data. The error handling is also
/// customizable by defining an `AdapterError` type, which must implement both `Error`
/// and `Debug` from the standard library.
///
/// # Usage
///
/// The `store_receipt` method should be used to store a new `ReceivedReceipt` in the storage
/// managed by the adapter. It returns a unique receipt_id associated with the stored receipt.
/// Any errors during this operation should be captured and returned in the `AdapterError` format.
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

#[async_trait]
pub trait ReceiptStore {
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
    async fn store_receipt(&self, receipt: ReceivedReceipt) -> Result<u64, Self::AdapterError>;
}

#[async_trait]
pub trait ReceiptDelete {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;
    /// Removes all `ReceivedReceipts` within a specific timestamp range from the storage.
    ///
    /// This method should be implemented to remove all `ReceivedReceipts` within a specific timestamp
    /// range from your storage system. Any errors that occur during this process should be captured and
    /// returned as an `AdapterError`.
    async fn remove_receipts_in_timestamp_range<R: RangeBounds<u64> + std::marker::Send>(
        &self,
        timestamp_ns: R,
    ) -> Result<(), Self::AdapterError>;
}

/// `ReceiptRead` defines a trait for read storage adapters to manage `ReceivedReceipt` data.
///
/// This trait is designed to be implemented by users of this library who want to
/// customize the read storage behavior of `ReceivedReceipt` data. The error handling is also
/// customizable by defining an `AdapterError` type, which must implement both `Error`
/// and `Debug` from the standard library.
///
/// # Usage
///
/// The `retrieve_receipts_in_timestamp_range` method should be implemented to fetch all `ReceivedReceipts`
/// within a specific timestamp range from the storage. The returned receipts should be in the form of a vector
/// of tuples where each tuple contains the unique receipt_id and the corresponding `ReceivedReceipt`.
#[async_trait]
pub trait ReceiptRead {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Retrieves all `ReceivedReceipts` within a specific timestamp range.
    ///
    /// This method should be implemented to fetch all `ReceivedReceipts` within a specific timestamp range
    /// from your storage system. The returned receipts should be in the form of a vector of tuples where
    /// each tuple contains the unique receipt_id and the corresponding `ReceivedReceipt`.
    ///
    /// If a limit is specified, the adapter should return at most that many receipts, while making
    /// sure that that no receipts are left behind for any timestamp that is returned. Examples:
    ///
    /// - If the adapter has 10 receipts for timestamp 100, and 5 receipts for timestamp 200, and
    ///  the limit is 10, the adapter should return all 10 receipts for timestamp 100, and none for
    /// timestamp 200.
    /// - If the adapter has 5 receipts for timestamp 100, and 10 receipts for timestamp 200, and
    /// the limit is 10, the adapter should return all 5 receipts for timestamp 100, and none for
    /// timestamp 200. (because it would have to leave behind 5 receipts for timestamp 200, which
    /// is not allowed).
    ///
    /// You can use the [`safe_truncate_receipts()`] function to help with this, but feel free to
    /// implement a more efficient solution for your situation if you can.
    ///
    /// Any errors that occur during this process should be captured and returned as an `AdapterError`.
    async fn retrieve_receipts_in_timestamp_range<R: RangeBounds<u64> + std::marker::Send>(
        &self,
        timestamp_range_ns: R,
        limit: Option<u64>,
    ) -> Result<Vec<StoredReceipt>, Self::AdapterError>;
}

pub struct StoredReceipt {
    pub receipt_id: u64,
    pub receipt: ReceivedReceipt,
}

impl From<(u64, ReceivedReceipt)> for StoredReceipt {
    fn from((receipt_id, receipt): (u64, ReceivedReceipt)) -> Self {
        Self {
            receipt_id,
            receipt,
        }
    }
}
/// See [`ReceiptStorageAdapter::retrieve_receipts_in_timestamp_range()`] for details.
///
/// WARNING: Will sort the receipts by timestamp using
/// [vec::sort_unstable](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort_unstable).
pub fn safe_truncate_receipts(receipts: &mut Vec<(u64, ReceivedReceipt)>, limit: u64) {
    if receipts.len() <= limit as usize {
        return;
    } else if limit == 0 {
        receipts.clear();
        return;
    }

    receipts
        .sort_unstable_by_key(|(_, rx_receipt)| rx_receipt.signed_receipt().message.timestamp_ns);

    // This one will be the last timestamp in `receipts` after naive truncation
    let last_timestamp = receipts[limit as usize - 1]
        .1
        .signed_receipt()
        .message
        .timestamp_ns;
    // This one is the timestamp that comes just after the one above
    let after_last_timestamp = receipts[limit as usize]
        .1
        .signed_receipt()
        .message
        .timestamp_ns;

    receipts.truncate(limit as usize);

    if last_timestamp == after_last_timestamp {
        // If the last timestamp is the same as the one that came after it, we need to
        // remove all the receipts with the same timestamp as the last one, because
        // otherwise we would leave behind part of the receipts for that timestamp.
        receipts.retain(|(_, rx_receipt)| {
            rx_receipt.signed_receipt().message.timestamp_ns != last_timestamp
        });
    }
}
