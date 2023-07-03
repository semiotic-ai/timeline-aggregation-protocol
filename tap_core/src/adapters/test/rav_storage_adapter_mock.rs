// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::{adapters::rav_storage_adapter::RAVStorageAdapter, tap_manager::SignedRAV};

pub struct RAVStorageAdapterMock {
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
