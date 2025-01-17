// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy::sol_types::SolStruct;
use async_trait::async_trait;

use crate::signed_message::EIP712SignedMessage;

/// Stores the latest RAV in the storage.
///
/// # Example
///
/// For example code see [crate::manager::context::memory::RAVStorage]

#[async_trait]
pub trait RAVStore<T>
where
    T: SolStruct,
{
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from
    /// the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Updates the storage with the latest validated `SignedRAV`.
    ///
    /// This method should be implemented to store the most recent validated
    /// `SignedRAV` into your chosen storage system. Any errors that occur
    /// during this process should be captured and returned as an `AdapterError`.
    async fn update_last_rav(&self, rav: EIP712SignedMessage<T>) -> Result<(), Self::AdapterError>;
}

/// Reads the RAV from storage
///
/// # Example
///
/// For example code see [crate::manager::context::memory::RAVStorage]

#[async_trait]
pub trait RAVRead<T>
where
    T: SolStruct,
{
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from
    /// the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Retrieves the latest `SignedRAV` from the storage.
    ///
    /// If no `SignedRAV` is available, this method should return `None`.
    async fn last_rav(&self) -> Result<Option<EIP712SignedMessage<T>>, Self::AdapterError>;
}
