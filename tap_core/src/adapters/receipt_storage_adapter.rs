// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeBounds;

use crate::tap_receipt::ReceivedReceipt;

pub trait ReceiptStorageAdapter {
    /// User defined error type;
    type AdapterError: std::error::Error + std::fmt::Debug;

    fn store_receipt(&mut self, receipt: ReceivedReceipt) -> Result<u64, Self::AdapterError>;
    fn retrieve_receipts_in_timestamp_range<R: RangeBounds<u64>>(
        &self,
        timestamp_range_ns: R,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, Self::AdapterError>;
    fn update_receipt_by_id(
        &mut self,
        receipt_id: u64,
        receipt: ReceivedReceipt,
    ) -> Result<(), Self::AdapterError>;
    fn remove_receipts_in_timestamp_range<R: RangeBounds<u64>>(
        &mut self,
        timestamp_ns: R,
    ) -> Result<(), Self::AdapterError>;
}
