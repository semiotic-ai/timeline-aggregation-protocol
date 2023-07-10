// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::{adapters::rav_storage_adapter::RAVStorageAdapter, tap_manager::SignedRAV};

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
/// use std::sync::{Arc, RwLock};
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

#[derive(Debug, Error)]
pub enum AdpaterErrorMock {
    #[error("something went wrong: {error}")]
    AdapterError { error: String },
}

impl RAVStorageAdapter for RAVStorageAdapterMock {
    type AdapterError = AdpaterErrorMock;

    fn update_last_rav(&mut self, rav: SignedRAV) -> Result<(), Self::AdapterError> {
        let mut rav_storage = self.rav_storage.write().unwrap();
        *rav_storage = Some(rav);
        Ok(())
    }
    fn last_rav(&self) -> Result<Option<SignedRAV>, Self::AdapterError> {
        Ok(self.rav_storage.read().unwrap().clone())
    }
}
