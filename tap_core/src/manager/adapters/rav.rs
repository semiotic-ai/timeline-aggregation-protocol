// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;

use crate::rav::SignedRAV;

/// Stores the latest RAV in the storage.
///
/// # Example
///
/// For example code see [crate::manager::context::memory::RAVStorage]

#[async_trait]
pub trait RAVStore {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Updates the storage with the latest validated `SignedRAV`.
    ///
    /// This method should be implemented to store the most recent validated `SignedRAV` into your chosen storage system.
    /// Any errors that occur during this process should be captured and returned as an `AdapterError`.
    async fn update_last_rav(&self, rav: SignedRAV) -> Result<(), Self::AdapterError>;
}

/// Reads the RAV from storage
///
/// # Example
///
/// For example code see [crate::manager::context::memory::RAVStorage]

#[async_trait]
pub trait RAVRead {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Retrieves the latest `SignedRAV` from the storage.
    ///
    /// If no `SignedRAV` is available, this method should return `None`.
    async fn last_rav(&self) -> Result<Option<SignedRAV>, Self::AdapterError>;
}
