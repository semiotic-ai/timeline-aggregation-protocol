// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Context implementations.
//!
//! Contexts are used to store and retrieve data from the TAP manager. They are used to store receipts, escrow, and other data that is required for the manager to function.
//! Currently, there's only one context implementation available, which is the `MemoryContext`. This context is used to store data in memory and is useful for testing and development purposes.
pub mod memory;
