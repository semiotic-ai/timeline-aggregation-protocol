// Copyright 2024 The Graph Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Result;

use crate::protocol_mode::ProtocolMode;

/// Validate that a batch of v1 receipts is valid for legacy processing
pub fn validate_v1_receipt_batch<T>(receipts: &[T]) -> Result<ProtocolMode> {
    if receipts.is_empty() {
        return Err(anyhow::anyhow!("Cannot aggregate empty receipt batch"));
    }
    // All v1 receipts use legacy mode
    Ok(ProtocolMode::Legacy)
}

/// Validate that a batch of v2 receipts is valid for horizon processing
#[cfg(feature = "v2")]
pub fn validate_v2_receipt_batch<T>(receipts: &[T]) -> Result<ProtocolMode> {
    if receipts.is_empty() {
        return Err(anyhow::anyhow!("Cannot aggregate empty receipt batch"));
    }
    // All v2 receipts use horizon mode
    Ok(ProtocolMode::Horizon)
}

#[cfg(test)]
mod tests {
    use tap_graph::Receipt;

    use super::*;

    #[test]
    fn test_validate_v1_batch() {
        use thegraph_core::alloy::primitives::Address;
        let receipt = Receipt::new(Address::ZERO, 100).unwrap();
        let receipts = vec![receipt];
        assert_eq!(
            validate_v1_receipt_batch(&receipts).unwrap(),
            ProtocolMode::Legacy
        );
    }

    #[test]
    fn test_validate_v1_empty_batch_fails() {
        let receipts: Vec<Receipt> = vec![];
        assert!(validate_v1_receipt_batch(&receipts).is_err());
    }

    #[cfg(feature = "v2")]
    #[test]
    fn test_validate_v2_batch() {
        use tap_graph::v2;
        use thegraph_core::alloy::primitives::{Address, FixedBytes};
        let receipt = v2::Receipt::new(
            FixedBytes::ZERO,
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            100,
        )
        .unwrap();
        let receipts = vec![receipt];
        assert_eq!(
            validate_v2_receipt_batch(&receipts).unwrap(),
            ProtocolMode::Horizon
        );
    }

    #[cfg(feature = "v2")]
    #[test]
    fn test_validate_v2_empty_batch_fails() {
        use tap_graph::v2;
        let receipts: Vec<v2::Receipt> = vec![];
        assert!(validate_v2_receipt_batch(&receipts).is_err());
    }
}
