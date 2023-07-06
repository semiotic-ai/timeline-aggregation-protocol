// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::tap_manager::SignedRAV;

pub trait RAVStorageAdapter {
    /// User defined error type;
    type AdapterError: std::error::Error + std::fmt::Debug + Send + Sync + 'static;

    fn update_last_rav(&mut self, rav: SignedRAV) -> Result<(), Self::AdapterError>;
    fn last_rav(&self) -> Result<Option<SignedRAV>, Self::AdapterError>;
}
