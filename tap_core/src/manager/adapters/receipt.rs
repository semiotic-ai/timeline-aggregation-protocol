// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeBounds;

use async_trait::async_trait;

use crate::receipt::{
    state::{Checking, ReceiptState},
    ReceiptWithState,
};

/// Stores receipts in the storage.
///
/// # Example
///
/// For example code see [crate::manager::context::memory::ReceiptStorage]
#[async_trait]
pub trait ReceiptStore {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Stores a new [`ReceiptWithState<Checking>`] into the storage.
    ///
    /// It returns a unique receipt_id associated with the stored receipt. Any errors that occur during
    /// this process should be captured and returned as an `AdapterError`.
    async fn store_receipt(
        &self,
        receipt: ReceiptWithState<Checking>,
    ) -> Result<u64, Self::AdapterError>;
}

/// Deletes receipts from storage.
///
/// # Example
///
/// For example code see [crate::manager::context::memory::ReceiptStorage]
#[async_trait]
pub trait ReceiptDelete {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;
    /// Removes all [`ReceiptWithState<Checking>`] within a specific timestamp range from the storage.
    ///
    /// This method should be implemented to remove all `ReceivedReceipts` within a specific timestamp
    /// range from your storage system. Any errors that occur during this process should be captured and
    /// returned as an `AdapterError`.
    async fn remove_receipts_in_timestamp_range<R: RangeBounds<u64> + std::marker::Send>(
        &self,
        timestamp_ns: R,
    ) -> Result<(), Self::AdapterError>;
}

/// Retrieves receipts from storage.
///
/// # Example
///
/// For example code see [crate::manager::context::memory::ReceiptStorage]
#[async_trait]
pub trait ReceiptRead {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from
    /// the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Retrieves all [`ReceiptWithState<Checking>`] within a specific timestamp range.
    ///
    /// If a limit is specified, the adapter should return at most that many receipts,
    /// while making sure that no receipts are left behind for any timestamp that
    /// is returned. Examples:
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
    ) -> Result<Vec<ReceiptWithState<Checking>>, Self::AdapterError>;
}

/// See [`ReceiptRead::retrieve_receipts_in_timestamp_range()`] for details.
///
/// WARNING: Will sort the receipts by timestamp using
/// [vec::sort_unstable](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort_unstable).
pub fn safe_truncate_receipts<T: ReceiptState>(
    receipts: &mut Vec<ReceiptWithState<T>>,
    limit: u64,
) {
    if receipts.len() <= limit as usize {
        return;
    } else if limit == 0 {
        receipts.clear();
        return;
    }

    receipts.sort_unstable_by_key(|rx_receipt| rx_receipt.signed_receipt().message.timestamp_ns);

    // This one will be the last timestamp in `receipts` after naive truncation
    let last_timestamp = receipts[limit as usize - 1]
        .signed_receipt()
        .message
        .timestamp_ns;
    // This one is the timestamp that comes just after the one above
    let after_last_timestamp = receipts[limit as usize]
        .signed_receipt()
        .message
        .timestamp_ns;

    receipts.truncate(limit as usize);

    if last_timestamp == after_last_timestamp {
        // If the last timestamp is the same as the one that came after it, we need to
        // remove all the receipts with the same timestamp as the last one, because
        // otherwise we would leave behind part of the receipts for that timestamp.
        receipts.retain(|rx_receipt| {
            rx_receipt.signed_receipt().message.timestamp_ns != last_timestamp
        });
    }
}
