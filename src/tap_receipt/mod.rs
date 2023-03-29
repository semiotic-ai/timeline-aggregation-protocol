mod receipt;
mod received_receipt;
use std::collections::HashMap;

pub use receipt::Receipt;
pub use received_receipt::ReceivedReceipt;

use crate::Result;
use strum_macros::{Display, EnumString};

#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumString, Display)]
pub enum ReceiptCheck {
    CheckUnique,
    CheckAllocationId,
    CheckTimestamp,
    CheckValue,
    CheckSignature,
    CheckCollateralAvailable,
}

type ReceiptCheckResults = HashMap<ReceiptCheck, Option<Result<()>>>;
