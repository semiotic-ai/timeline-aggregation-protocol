// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing error and warning codes used by the TAP aggregator JSON-RPC API.
//!
//! The JSON-RPC spec allocates error codes in the range `[-32000, -32099]` for application errors. We chose to use that
//! range for both errors and warnings. We also chose error and warning codes that do not overlap, to reduce confusion.
//! As such, the ranges are:
//! - Errors: `[-32000, -32049]`, where `-32000` is reserved for all errors without a specific code.
//! - Warnings: `[-32050, -32099]`, where `-32050` is reserved for all warnings without a specific code.

/// JSON-RPC error codes specific to the TAP aggregator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonRpcErrorCode {
    /// -32000 -- Generic error.
    #[allow(dead_code)]
    Generic = -32000,
    /// -32001 -- Invalid API version.
    InvalidVersion = -32001,
    /// -32002 -- Error during receipt aggregation.
    Aggregation = -32002,
}

/// JSON-RPC warning codes
/// These are not part of the JSON-RPC spec, but are used to provide additional information to the
/// client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum JsonRpcWarningCode {
    /// -32050 -- Generic warning.
    #[allow(dead_code)]
    Generic = -32050,
    /// -32051 -- Requested API version is deprecated.
    DeprecatedVersion = -32051,
}
