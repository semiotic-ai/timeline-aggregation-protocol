// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use alloy_primitives::Address;
use async_trait::async_trait;

/// `EscrowAdapter` defines a trait for adapters to handle escrow related operations.
///
/// This trait is designed to be implemented by users of this library who want to
/// customize the management of local accounting for available escrow. The error handling is also
/// customizable by defining an `AdapterError` type, which must implement both `Error`
/// and `Debug` from the standard library.
///
/// # Usage
///
/// The `get_available_escrow` method should be used to retrieve the local accounting
///  amount of available escrow for a specified gateway. Any errors during this operation
/// should be captured and returned in the `AdapterError` format.
///
/// The `subtract_escrow` method is used to deduct a specified value from the local accounting
/// of available escrow of a specified gateway. Any errors during this operation should be captured
/// and returned as an `AdapterError`.
///
/// This trait is utilized by [crate::tap_manager], which relies on these
/// operations for managing escrow.
///
/// # Example
///
/// For example code see [crate::adapters::escrow_adapter_mock]

#[async_trait]
pub trait EscrowAdapter {
    /// Defines the user-specified error type.
    ///
    /// This error type should implement the `Error` and `Debug` traits from the standard library.
    /// Errors of this type are returned to the user when an operation fails.
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    /// Retrieves the local accounting amount of available escrow for a specified gateway.
    ///
    /// This method should be implemented to fetch the local accounting amount of available escrow for a
    /// specified gateway from your system. Any errors that occur during this process should
    /// be captured and returned as an `AdapterError`.
    async fn get_available_escrow(&self, gateway_id: Address) -> Result<u128, Self::AdapterError>;

    /// Deducts a specified value from the local accounting of available escrow for a specified gateway.
    ///
    /// This method should be implemented to deduct a specified value from the local accounting of
    /// available escrow of a specified gateway in your system. Any errors that occur during this
    /// process should be captured and returned as an `AdapterError`.
    async fn subtract_escrow(
        &self,
        gateway_id: Address,
        value: u128,
    ) -> Result<(), Self::AdapterError>;
}
