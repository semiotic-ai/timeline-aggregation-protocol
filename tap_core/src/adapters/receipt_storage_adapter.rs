// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::tap_receipt::ReceivedReceipt;

pub trait ReceiptStorageAdapter {
    /// User defined error type;
    type AdapterError: std::error::Error + std::fmt::Debug;

    fn store_receipt(&mut self, receipt: ReceivedReceipt) -> Result<u64, Self::AdapterError>;
    fn retrieve_receipt_by_id(
        &self,
        receipt_id: u64,
    ) -> Result<ReceivedReceipt, Self::AdapterError>;
    fn retrieve_receipts_by_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, Self::AdapterError>;
    fn retrieve_receipts_upto_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, Self::AdapterError>;
    fn update_receipt_by_id(
        &mut self,
        receipt_id: u64,
        receipt: ReceivedReceipt,
    ) -> Result<(), Self::AdapterError>;
    fn remove_receipt_by_id(&mut self, receipt_id: u64) -> Result<(), Self::AdapterError>;
    fn remove_receipts_by_ids(&mut self, receipt_ids: &[u64]) -> Result<(), Self::AdapterError>;
}
