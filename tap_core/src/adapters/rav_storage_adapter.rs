// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    eip_712_signed_message::EIP712SignedMessage, receipt_aggregate_voucher::ReceiptAggregateVoucher,
};

pub trait RAVStorageAdapter {
    /// User defined error type;
    type AdapterError: std::error::Error + std::fmt::Debug;

    fn store_rav(
        &mut self,
        rav: EIP712SignedMessage<ReceiptAggregateVoucher>,
    ) -> Result<u64, Self::AdapterError>;
    fn retrieve_rav_by_id(
        &self,
        rav_id: u64,
    ) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>, Self::AdapterError>;
    fn remove_rav_by_id(&mut self, rav_id: u64) -> Result<(), Self::AdapterError>;
}
