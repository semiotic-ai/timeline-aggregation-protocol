use ethereum_types::Address;
use k256::{
    ecdsa::{SigningKey, Signature, signature::Signer, VerifyingKey, signature::Verifier}
};

use serde::{Serialize, Deserialize};
use crate::{Result, Error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Receipt{
    allocation_id: Address,
    timestamp: u64,
    nonce: u64,
    value: u64,
    signature: Signature
}

impl Receipt{
    pub fn new(allocation_id: Address, timestamp: u64, nonce: u64, value: u64, signing_key: &SigningKey) -> Receipt {
        Receipt {
            allocation_id: allocation_id,
            timestamp: timestamp,
            nonce: nonce,
            value: value,
            signature: signing_key.sign(&Self::get_message_bytes(allocation_id, timestamp, nonce, value))
        }
    }

    pub fn is_valid(
        self: &Self,
        verifying_key: VerifyingKey,
        allocation_id: Address,
        min_timestamp: u64
    ) -> Result<()>{
        if self.allocation_id != allocation_id {
            return Err(Error::InvalidAllocationID {
                received_allocation_id: self.allocation_id,
                expected_allocation_id: allocation_id
            });
        }
        if self.timestamp < min_timestamp {
            return Err(Error::InvalidTimestamp {
                received_timestamp: self.timestamp,
                min_timestamp: min_timestamp
            });
        }
        verifying_key
        .verify(
            &Self::get_message_bytes(
                self.allocation_id,
                self.timestamp,
                self.nonce,
                self.value
            ),
            &self.signature
        )?;
        Ok(())
    }

    pub fn get_value(self: &Self) -> u64{
        self.value
    }

    pub fn get_allocation_id(self: &Self) -> Address{
        self.allocation_id
    }

    pub fn get_timestamp(self: &Self) -> u64{
        self.timestamp
    }

    fn get_message_bytes(allocation_id: Address, timestamp: u64, nonce: u64, value: u64) -> Vec<u8>{
        allocation_id.as_bytes().iter().copied()
            .chain(timestamp.to_be_bytes())
            .chain(nonce.to_be_bytes())
            .chain(value.to_be_bytes())
            .collect()
    }
}