// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;

use crate::rav::SignedRAV;

/// `RAVStore` defines a trait for write storage adapters to handle `SignedRAV` data.
///
/// This trait is designed to be implemented by users of this library who want to
/// customize the write storage behavior of `SignedRAV` data. The error handling is also
/// customizable by defining an `StrategyError` type, which must implement both `Error`
/// and `Debug` from the standard library.
///
/// # Usage
///
/// The `update_last_rav` method should be used to update the last validated `SignedRAV`
/// in the storage managed by the adapter. Errors during this operation should be
/// captured and returned in the `StrategyError` format.
///
/// This trait is utilized by [crate::tap_manager], which relies on these
/// operations for working with `SignedRAV` data.
///
/// # Example
///
/// For example code see [crate::adapters::rav_storage_adapter_mock]

#[async_trait]
pub trait RAVStore {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type StrategyError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Updates the storage with the latest validated `SignedRAV`.
    ///
    /// This method should be implemented to store the most recent validated `SignedRAV` into your chosen storage system.
    /// Any errors that occur during this process should be captured and returned as an `StrategyError`.
    async fn update_last_rav(&self, rav: SignedRAV) -> Result<(), Self::StrategyError>;
}

/// `RAVRead` defines a trait for read storage adapters to handle `SignedRAV` data.
///
/// This trait is designed to be implemented by users of this library who want to
/// customize the read storage behavior of `SignedRAV` data. The error handling is also
/// customizable by defining an `StrategyError` type, which must implement both `Error`
/// and `Debug` from the standard library.
///
/// # Usage
///
/// The `last_rav` method is designed to fetch the latest `SignedRAV` from the storage.
/// If there is no `SignedRAV` available, it should return `None`. Any errors during
/// this operation should be captured and returned as an `StrategyError`.
///
/// This trait is utilized by [crate::tap_manager], which relies on these
/// operations for working with `SignedRAV` data.
///
/// # Example
///
/// For example code see [crate::adapters::rav_storage_adapter_mock]

#[async_trait]
pub trait RAVRead {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type StrategyError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Retrieves the latest `SignedRAV` from the storage.
    ///
    /// This method should be implemented to fetch the latest `SignedRAV` from your storage system.
    /// If no `SignedRAV` is available, this method should return `None`.
    /// Any errors that occur during this process should be captured and returned as an `StrategyError`.
    async fn last_rav(&self) -> Result<Option<SignedRAV>, Self::StrategyError>;
}
