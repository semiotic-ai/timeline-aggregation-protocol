use crate::{
    eip_712_signed_message::EIP712SignedMessage, receipt_aggregate_voucher::ReceiptAggregateVoucher,
};

pub trait RAVStorageAdapter<T> {
    fn store_rav(&mut self, rav: EIP712SignedMessage<ReceiptAggregateVoucher>) -> Result<u64, T>;
    fn retrieve_rav_by_id(
        &self,
        rav_id: u64,
    ) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>, T>;
    fn remove_rav_by_id(&mut self, rav_id: u64) -> Result<(), T>;
}
