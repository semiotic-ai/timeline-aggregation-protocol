use serde::{Deserialize, Serialize};

use super::SignedReceipt;
use crate::receipt_aggregate_voucher::ReceiptAggregateVoucher;

#[derive(Debug, Serialize, Deserialize, Clone)]

pub struct RAVRequest {
    pub valid_receipts: Vec<SignedReceipt>,
    pub invalid_receipts: Vec<SignedReceipt>,
    pub expected_rav: ReceiptAggregateVoucher,
}
