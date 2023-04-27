use ethereum_types::Address;

use crate::{eip_712_signed_message::EIP712SignedMessage, tap_receipt::Receipt};

pub trait ReceiptChecksAdapter {
    fn is_unique(&self, receipt: &EIP712SignedMessage<Receipt>) -> bool;
    fn is_valid_allocation_id(&self, allocation_id: Address) -> bool;
    fn is_valid_value(&self, value: u128, query_id: u64) -> bool;
    fn is_valid_gateway_id(&self, gateway_id: Address) -> bool;
}
