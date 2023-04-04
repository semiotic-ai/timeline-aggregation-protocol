mod receipt;
mod received_receipt;
use std::collections::HashMap;

pub use receipt::Receipt;
pub use received_receipt::ReceivedReceipt;
use strum_macros::{Display, EnumString};

pub type ReceiptCheckResults = HashMap<ReceiptCheck, Option<crate::Result<()>>>;
#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumString, Display)]
pub enum ReceiptCheck {
    CheckUnique,
    CheckAllocationId,
    CheckTimestamp,
    CheckValue,
    CheckSignature,
    CheckCollateralAvailable,
}

pub fn get_full_list_of_checks() -> ReceiptCheckResults {
    let mut all_checks_list = ReceiptCheckResults::new();
    all_checks_list.insert(ReceiptCheck::CheckUnique, None);
    all_checks_list.insert(ReceiptCheck::CheckAllocationId, None);
    all_checks_list.insert(ReceiptCheck::CheckTimestamp, None);
    all_checks_list.insert(ReceiptCheck::CheckValue, None);
    all_checks_list.insert(ReceiptCheck::CheckSignature, None);
    all_checks_list.insert(ReceiptCheck::CheckCollateralAvailable, None);

    all_checks_list
}

#[cfg(test)]
pub mod tests;
