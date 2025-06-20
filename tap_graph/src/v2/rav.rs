// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! # Receipt Aggregate Voucher v2

use std::cmp;

use serde::{Deserialize, Serialize};
use tap_eip712_message::Eip712SignedMessage;
use tap_receipt::{
    rav::{Aggregate, AggregationError},
    state::Checked,
    ReceiptWithState, WithValueAndTimestamp,
};
use thegraph_core::alloy::{
    primitives::{Address, Bytes, FixedBytes},
    sol,
};

use super::{Receipt, SignedReceipt};

/// EIP712 signed message for ReceiptAggregateVoucher
pub type SignedRav = Eip712SignedMessage<ReceiptAggregateVoucher>;

sol! {
    /// Holds information needed for promise of payment signed with ECDSA
    ///
    /// We use camelCase for field names to match the Ethereum ABI encoding
    #[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
    struct ReceiptAggregateVoucher {
        /// Unique collection id this RAV belongs to
        bytes32 collectionId;
        // The address of the payer the RAV was issued by
        address payer;
        // The address of the data service the RAV was issued to
        address dataService;
        // The address of the service provider the RAV was issued to
        address serviceProvider;
        // The RAV timestamp, indicating the latest TAP Receipt in the RAV
        uint64 timestampNs;
        // Total amount owed to the service provider since the beginning of the
        // payer-service provider relationship, including all debt that is already paid for.
        uint128 valueAggregate;
        // Arbitrary metadata to extend functionality if a data service requires it
        bytes metadata;
    }
}

impl ReceiptAggregateVoucher {
    /// Aggregates a batch of validated receipts with optional validated previous RAV,
    /// returning a new RAV if all provided items are valid or an error if not.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AggregateOverflow`] if any receipt value causes aggregate
    /// value to overflow
    pub fn aggregate_receipts(
        collection_id: FixedBytes<32>,
        payer: Address,
        data_service: Address,
        service_provider: Address,
        receipts: &[Eip712SignedMessage<Receipt>],
        previous_rav: Option<Eip712SignedMessage<Self>>,
    ) -> Result<Self, AggregationError> {
        //TODO(#29): When receipts in flight struct in created check that the state
        // of every receipt is OK with all checks complete (relies on #28)
        // If there is a previous RAV get initialize values from it, otherwise get default values
        let mut timestamp_max = 0u64;
        let mut value_aggregate = 0u128;

        if let Some(prev_rav) = previous_rav {
            timestamp_max = prev_rav.message.timestampNs;
            value_aggregate = prev_rav.message.valueAggregate;
        }

        for receipt in receipts {
            value_aggregate = value_aggregate
                .checked_add(receipt.message.value)
                .ok_or(AggregationError::AggregateOverflow)?;

            timestamp_max = cmp::max(timestamp_max, receipt.message.timestamp_ns)
        }

        Ok(Self {
            collectionId: collection_id,
            timestampNs: timestamp_max,
            valueAggregate: value_aggregate,
            payer,
            dataService: data_service,
            serviceProvider: service_provider,
            metadata: Bytes::new(),
        })
    }
}

impl Aggregate<SignedReceipt> for ReceiptAggregateVoucher {
    fn aggregate_receipts(
        receipts: &[ReceiptWithState<Checked, SignedReceipt>],
        previous_rav: Option<Eip712SignedMessage<Self>>,
    ) -> Result<Self, AggregationError> {
        if receipts.is_empty() {
            return Err(AggregationError::NoValidReceiptsForRavRequest);
        }
        let collection_id = receipts[0].signed_receipt().message.collection_id;
        let payer = receipts[0].signed_receipt().message.payer;
        let data_service = receipts[0].signed_receipt().message.data_service;
        let service_provider = receipts[0].signed_receipt().message.service_provider;
        let receipts = receipts
            .iter()
            .map(|rx_receipt| rx_receipt.signed_receipt().clone())
            .collect::<Vec<_>>();
        ReceiptAggregateVoucher::aggregate_receipts(
            collection_id,
            payer,
            data_service,
            service_provider,
            receipts.as_slice(),
            previous_rav,
        )
    }
}

impl WithValueAndTimestamp for ReceiptAggregateVoucher {
    fn value(&self) -> u128 {
        self.valueAggregate
    }

    fn timestamp_ns(&self) -> u64 {
        self.timestampNs
    }
}
