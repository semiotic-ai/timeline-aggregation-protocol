use std::collections::{HashMap, HashSet};

use ethereum_types::Address;

use crate::{
    adapters::receipt_checks_adapter::ReceiptChecksAdapter,
    eip_712_signed_message::EIP712SignedMessage,
    tap_receipt::{Receipt, ReceivedReceipt},
};

pub struct ReceiptChecksAdapterMock {
    receipt_storage: HashMap<u64, ReceivedReceipt>,
    query_appraisals: HashMap<u64, u128>,
    allocation_ids: HashSet<Address>,
    gateway_ids: HashSet<Address>,
}

impl ReceiptChecksAdapterMock {
    pub fn new(
        receipt_storage: HashMap<u64, ReceivedReceipt>,
        query_appraisals: HashMap<u64, u128>,
        allocation_ids: HashSet<Address>,
        gateway_ids: HashSet<Address>,
    ) -> Self {
        Self {
            receipt_storage,
            query_appraisals,
            allocation_ids,
            gateway_ids,
        }
    }
}

impl ReceiptChecksAdapter for ReceiptChecksAdapterMock {
    fn is_unique(&self, receipt: &EIP712SignedMessage<Receipt>) -> bool {
        self.receipt_storage
            .iter()
            .all(|(_, stored_receipt)| stored_receipt.signed_receipt.message != receipt.message)
    }

    fn is_valid_allocation_id(&self, allocation_id: Address) -> bool {
        self.allocation_ids.contains(&allocation_id)
    }

    fn is_valid_value(&self, value: u128, query_id: u64) -> bool {
        let appraised_value = self.query_appraisals.get(&query_id).unwrap();

        if value != *appraised_value {
            return false;
        }
        true
    }
    fn is_valid_gateway_id(&self, gateway_id: Address) -> bool {
        self.gateway_ids.contains(&gateway_id)
    }
}
