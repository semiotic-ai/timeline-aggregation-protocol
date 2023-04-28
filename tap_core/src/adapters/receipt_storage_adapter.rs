use crate::tap_receipt::ReceivedReceipt;

pub trait ReceiptStorageAdapter<T> {
    fn store_receipt(&mut self, receipt: ReceivedReceipt) -> Result<u64, T>;
    fn retrieve_receipt_by_id(&self, receipt_id: u64) -> Result<ReceivedReceipt, T>;
    fn retrieve_receipts_by_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, T>;
    fn retrieve_receipts_upto_timestamp(
        &self,
        timestamp_ns: u64,
    ) -> Result<Vec<(u64, ReceivedReceipt)>, T>;
    fn remove_receipt_by_id(&mut self, receipt_id: u64) -> Result<(), T>;
    fn remove_receipts_by_ids(&mut self, receipt_ids: &[u64]) -> Result<(), T>;
}
