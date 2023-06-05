// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

/// JSON-RPC error codes specific to the TAP aggregator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonRpcErrorCode {
    /// -32001 -- Invalid API version.
    InvalidVersionError = -32001,
    /// -32002 -- Error during receipt aggregation.
    AggregationError = -32002,
}

/// JSON-RPC warning codes
/// These are not part of the JSON-RPC spec, but are used to provide additional information to the
/// client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonRpcWarningCode {
    /// -32101 -- Requested API version is deprecated.
    DeprecatedVersionWarning = -32101,
}
