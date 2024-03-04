// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    adapters::rav_storage_adapter::{RAVRead, RAVStore},
    tap_manager::SignedRAV,
};

use super::executor_mock::AdapterErrorMock;

/// `RAVStorageAdapterMock` is a mock implementation of the `RAVStorageAdapter` trait.
///
/// It serves two main purposes:
///
/// 1. **Unit Testing**: The `RAVStorageAdapterMock` is primarily intended to be used for unit tests,
///    providing a way to simulate the behavior of a real `RAVStorageAdapter` without requiring a real
///    implementation. By using a mock implementation, you can create predictable behaviors and
///    responses, enabling isolated and focused testing of the logic that depends on the `RAVStorageAdapter` trait.
///
/// 2. **Example Implementation**: New users of the `RAVStorageAdapter` trait can look to
///    `RAVStorageAdapterMock` as a basic example of how to implement the trait.
///
/// Note: This mock implementation is not suitable for production use. Its methods simply manipulate a
/// local `RwLock<Option<SignedRAV>>`, and it provides no real error handling.
///
/// # Usage
///
/// To use `RAVStorageAdapterMock`, first create an `Arc<RwLock<Option<SignedRAV>>>`, then pass it to
/// `RAVStorageAdapterMock::new()`. Now, it can be used anywhere a `RAVStorageAdapter` is required.
///
/// ```rust
/// use std::sync::{Arc};
/// use tokio::sync::RwLock;
/// use tap_core::{tap_manager::SignedRAV, adapters::rav_storage_adapter_mock::RAVStorageAdapterMock};
///
/// let rav_storage: Arc<RwLock<Option<SignedRAV>>> = Arc::new(RwLock::new(None));
/// let adapter = RAVStorageAdapterMock::new(rav_storage);
/// ```
pub struct RAVStorageAdapterMock {
    /// local RAV store with rwlocks to allow sharing with other compenents as needed
    rav_storage: Arc<RwLock<Option<SignedRAV>>>,
}

impl RAVStorageAdapterMock {
    pub fn new(rav_storage: Arc<RwLock<Option<SignedRAV>>>) -> Self {
        RAVStorageAdapterMock { rav_storage }
    }
}

#[async_trait]
impl RAVStore for RAVStorageAdapterMock {
    type AdapterError = AdapterErrorMock;

    async fn update_last_rav(&self, rav: SignedRAV) -> Result<(), Self::AdapterError> {
        let mut rav_storage = self.rav_storage.write().await;
        *rav_storage = Some(rav);
        Ok(())
    }
}

#[async_trait]
impl RAVRead for RAVStorageAdapterMock {
    type AdapterError = AdapterErrorMock;

    async fn last_rav(&self) -> Result<Option<SignedRAV>, Self::AdapterError> {
        Ok(self.rav_storage.read().await.clone())
    }
}
