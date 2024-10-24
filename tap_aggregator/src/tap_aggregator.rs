use anyhow::anyhow;
use tap_core::signed_message::EIP712SignedMessage;

tonic::include_proto!("tap_aggregator.v1");

impl TryFrom<Receipt> for tap_core::receipt::Receipt {
    type Error = anyhow::Error;
    fn try_from(receipt: Receipt) -> Result<Self, Self::Error> {
        Ok(Self {
            allocation_id: receipt.allocation_id.as_slice().try_into()?,
            timestamp_ns: receipt.timestamp_ns,
            value: receipt.value.ok_or(anyhow!("Missing value"))?.into(),
            nonce: receipt.nonce,
        })
    }
}

impl TryFrom<SignedReceipt> for tap_core::receipt::SignedReceipt {
    type Error = anyhow::Error;
    fn try_from(receipt: SignedReceipt) -> Result<Self, Self::Error> {
        Ok(Self {
            signature: receipt.signature.as_slice().try_into()?,
            message: receipt
                .message
                .ok_or(anyhow!("Missing message"))?
                .try_into()?,
        })
    }
}

impl TryFrom<SignedRav> for EIP712SignedMessage<tap_core::rav::ReceiptAggregateVoucher> {
    type Error = anyhow::Error;
    fn try_from(voucher: SignedRav) -> Result<Self, Self::Error> {
        Ok(Self {
            signature: voucher.signature.as_slice().try_into()?,
            message: voucher
                .message
                .ok_or(anyhow!("Missing message"))?
                .try_into()?,
        })
    }
}

impl From<EIP712SignedMessage<tap_core::rav::ReceiptAggregateVoucher>> for SignedRav {
    fn from(voucher: EIP712SignedMessage<tap_core::rav::ReceiptAggregateVoucher>) -> Self {
        Self {
            signature: voucher.signature.as_bytes().to_vec(),
            message: Some(voucher.message.into()),
        }
    }
}

impl TryFrom<ReceiptAggregateVoucher> for tap_core::rav::ReceiptAggregateVoucher {
    type Error = anyhow::Error;
    fn try_from(voucher: ReceiptAggregateVoucher) -> Result<Self, Self::Error> {
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

impl From<tap_core::rav::ReceiptAggregateVoucher> for ReceiptAggregateVoucher {
    fn from(voucher: tap_core::rav::ReceiptAggregateVoucher) -> Self {
        Self {
            allocation_id: voucher.allocationId.to_vec(),
            timestamp_ns: voucher.timestampNs,
            value_aggregate: Some(voucher.valueAggregate.into()),
        }
    }
}

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