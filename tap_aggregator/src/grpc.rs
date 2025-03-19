// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod uint128 {
    tonic::include_proto!("grpc.uint128");

    impl From<Uint128> for u128 {
        fn from(Uint128 { high, low }: Uint128) -> Self {
            ((high as u128) << 64) | low as u128
        }
    }

    impl From<u128> for Uint128 {
        fn from(value: u128) -> Self {
            let high = (value >> 64) as u64;
            let low = value as u64;
            Self { high, low }
        }
    }
}

pub mod v1 {
    use anyhow::anyhow;
    use tap_core::signed_message::Eip712SignedMessage;

    tonic::include_proto!("tap_aggregator.v1");

    impl TryFrom<self::Receipt> for tap_graph::Receipt {
        type Error = anyhow::Error;
        fn try_from(receipt: self::Receipt) -> Result<Self, Self::Error> {
            Ok(Self {
                allocation_id: receipt.allocation_id.as_slice().try_into()?,
                timestamp_ns: receipt.timestamp_ns,
                value: receipt.value.ok_or(anyhow!("Missing value"))?.into(),
                nonce: receipt.nonce,
            })
        }
    }

    impl TryFrom<self::SignedReceipt> for tap_graph::SignedReceipt {
        type Error = anyhow::Error;
        fn try_from(receipt: self::SignedReceipt) -> Result<Self, Self::Error> {
            Ok(Self {
                signature: receipt.signature.as_slice().try_into()?,
                message: receipt
                    .message
                    .ok_or(anyhow!("Missing message"))?
                    .try_into()?,
            })
        }
    }

    impl From<tap_graph::Receipt> for self::Receipt {
        fn from(value: tap_graph::Receipt) -> Self {
            Self {
                allocation_id: value.allocation_id.as_slice().to_vec(),
                timestamp_ns: value.timestamp_ns,
                nonce: value.nonce,
                value: Some(value.value.into()),
            }
        }
    }

    impl From<tap_graph::SignedReceipt> for self::SignedReceipt {
        fn from(value: tap_graph::SignedReceipt) -> Self {
            Self {
                message: Some(value.message.into()),
                signature: value.signature.as_bytes().to_vec(),
            }
        }
    }

    impl TryFrom<self::SignedRav> for Eip712SignedMessage<tap_graph::ReceiptAggregateVoucher> {
        type Error = anyhow::Error;
        fn try_from(voucher: self::SignedRav) -> Result<Self, Self::Error> {
            Ok(Self {
                signature: voucher.signature.as_slice().try_into()?,
                message: voucher
                    .message
                    .ok_or(anyhow!("Missing message"))?
                    .try_into()?,
            })
        }
    }

    impl From<Eip712SignedMessage<tap_graph::ReceiptAggregateVoucher>> for self::SignedRav {
        fn from(voucher: Eip712SignedMessage<tap_graph::ReceiptAggregateVoucher>) -> Self {
            Self {
                signature: voucher.signature.as_bytes().to_vec(),
                message: Some(voucher.message.into()),
            }
        }
    }

    impl TryFrom<self::ReceiptAggregateVoucher> for tap_graph::ReceiptAggregateVoucher {
        type Error = anyhow::Error;
        fn try_from(voucher: self::ReceiptAggregateVoucher) -> Result<Self, Self::Error> {
            Ok(Self {
                allocationId: voucher.allocation_id.as_slice().try_into()?,
                timestampNs: voucher.timestamp_ns,
                valueAggregate: voucher
                    .value_aggregate
                    .ok_or(anyhow!("Missing Value Aggregate"))?
                    .into(),
            })
        }
    }

    impl From<tap_graph::ReceiptAggregateVoucher> for self::ReceiptAggregateVoucher {
        fn from(voucher: tap_graph::ReceiptAggregateVoucher) -> Self {
            Self {
                allocation_id: voucher.allocationId.to_vec(),
                timestamp_ns: voucher.timestampNs,
                value_aggregate: Some(voucher.valueAggregate.into()),
            }
        }
    }

    impl self::RavRequest {
        pub fn new(
            receipts: Vec<tap_graph::SignedReceipt>,
            previous_rav: Option<tap_graph::SignedRav>,
        ) -> Self {
            Self {
                receipts: receipts.into_iter().map(Into::into).collect(),
                previous_rav: previous_rav.map(Into::into),
            }
        }
    }

    impl self::RavResponse {
        pub fn signed_rav(mut self) -> anyhow::Result<tap_graph::SignedRav> {
            let signed_rav: tap_graph::SignedRav = self
                .rav
                .take()
                .ok_or(anyhow!("Couldn't find rav"))?
                .try_into()?;
            Ok(signed_rav)
        }
    }
}

pub mod v2 {
    use anyhow::anyhow;
    use tap_core::signed_message::Eip712SignedMessage;
    use thegraph_core::alloy::primitives::Bytes;

    tonic::include_proto!("tap_aggregator.v2");

    impl TryFrom<self::Receipt> for tap_graph::v2::Receipt {
        type Error = anyhow::Error;
        fn try_from(receipt: self::Receipt) -> Result<Self, Self::Error> {
            Ok(Self {
                allocation_id: receipt.allocation_id.as_slice().try_into()?,
                timestamp_ns: receipt.timestamp_ns,
                value: receipt.value.ok_or(anyhow!("Missing value"))?.into(),
                nonce: receipt.nonce,
                payer: receipt.payer.as_slice().try_into()?,
                data_service: receipt.data_service.as_slice().try_into()?,
                service_provider: receipt.service_provider.as_slice().try_into()?,
            })
        }
    }

    impl TryFrom<self::SignedReceipt> for tap_graph::v2::SignedReceipt {
        type Error = anyhow::Error;
        fn try_from(receipt: self::SignedReceipt) -> Result<Self, Self::Error> {
            Ok(Self {
                signature: receipt.signature.as_slice().try_into()?,
                message: receipt
                    .message
                    .ok_or(anyhow!("Missing message"))?
                    .try_into()?,
            })
        }
    }

    impl From<tap_graph::v2::Receipt> for self::Receipt {
        fn from(value: tap_graph::v2::Receipt) -> Self {
            Self {
                allocation_id: value.allocation_id.as_slice().to_vec(),
                timestamp_ns: value.timestamp_ns,
                nonce: value.nonce,
                value: Some(value.value.into()),
                payer: value.payer.as_slice().to_vec(),
                data_service: value.data_service.as_slice().to_vec(),
                service_provider: value.service_provider.as_slice().to_vec(),
            }
        }
    }

    impl From<tap_graph::v2::SignedReceipt> for self::SignedReceipt {
        fn from(value: tap_graph::v2::SignedReceipt) -> Self {
            Self {
                message: Some(value.message.into()),
                signature: value.signature.as_bytes().to_vec(),
            }
        }
    }

    impl TryFrom<self::SignedRav> for Eip712SignedMessage<tap_graph::v2::ReceiptAggregateVoucher> {
        type Error = anyhow::Error;
        fn try_from(voucher: self::SignedRav) -> Result<Self, Self::Error> {
            Ok(Self {
                signature: voucher.signature.as_slice().try_into()?,
                message: voucher
                    .message
                    .ok_or(anyhow!("Missing message"))?
                    .try_into()?,
            })
        }
    }

    impl From<Eip712SignedMessage<tap_graph::v2::ReceiptAggregateVoucher>> for self::SignedRav {
        fn from(voucher: Eip712SignedMessage<tap_graph::v2::ReceiptAggregateVoucher>) -> Self {
            Self {
                signature: voucher.signature.as_bytes().to_vec(),
                message: Some(voucher.message.into()),
            }
        }
    }

    impl TryFrom<self::ReceiptAggregateVoucher> for tap_graph::v2::ReceiptAggregateVoucher {
        type Error = anyhow::Error;
        fn try_from(voucher: self::ReceiptAggregateVoucher) -> Result<Self, Self::Error> {
            Ok(Self {
                allocationId: voucher.allocation_id.as_slice().try_into()?,
                timestampNs: voucher.timestamp_ns,
                valueAggregate: voucher
                    .value_aggregate
                    .ok_or(anyhow!("Missing Value Aggregate"))?
                    .into(),
                payer: voucher.payer.as_slice().try_into()?,
                dataService: voucher.data_service.as_slice().try_into()?,
                serviceProvider: voucher.service_provider.as_slice().try_into()?,
                metadata: Bytes::copy_from_slice(voucher.metadata.as_slice()),
            })
        }
    }

    impl From<tap_graph::v2::ReceiptAggregateVoucher> for self::ReceiptAggregateVoucher {
        fn from(voucher: tap_graph::v2::ReceiptAggregateVoucher) -> Self {
            Self {
                allocation_id: voucher.allocationId.to_vec(),
                timestamp_ns: voucher.timestampNs,
                value_aggregate: Some(voucher.valueAggregate.into()),
                payer: voucher.payer.to_vec(),
                data_service: voucher.dataService.to_vec(),
                service_provider: voucher.serviceProvider.to_vec(),
                metadata: voucher.metadata.to_vec(),
            }
        }
    }

    impl self::RavRequest {
        pub fn new(
            receipts: Vec<tap_graph::v2::SignedReceipt>,
            previous_rav: Option<tap_graph::v2::SignedRav>,
        ) -> Self {
            Self {
                receipts: receipts.into_iter().map(Into::into).collect(),
                previous_rav: previous_rav.map(Into::into),
            }
        }
    }

    impl self::RavResponse {
        pub fn signed_rav(mut self) -> anyhow::Result<tap_graph::v2::SignedRav> {
            let signed_rav: tap_graph::v2::SignedRav = self
                .rav
                .take()
                .ok_or(anyhow!("Couldn't find rav"))?
                .try_into()?;
            Ok(signed_rav)
        }
    }
}
