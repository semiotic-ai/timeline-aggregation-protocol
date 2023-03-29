mod receipt;
mod received_receipt;
pub use receipt::Receipt;
pub use received_receipt::ReceivedReceipt;

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum ReceiptCheck {
    CheckUnique,
    CheckAllocationId,
    CheckTimestamp,
    CheckValue,
    CheckSignature,
    CheckCollateralAvailable,
}