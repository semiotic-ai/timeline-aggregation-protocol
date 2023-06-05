// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use jsonrpsee::core::Serialize;
use serde::Deserialize;
use serde_json::value::Value;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonRpcWarning {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonRpcResponse<T: Serialize> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<JsonRpcWarning>>,
}

pub type JsonRpcError = jsonrpsee::types::ErrorObjectOwned;
pub type JsonRpcResult<T> = Result<JsonRpcResponse<T>, JsonRpcError>;

impl<T: Serialize> JsonRpcResponse<T> {
    /// Helper method that returns a JsonRpcResponse with the given data and no warnings.
    pub fn ok(data: T) -> Self {
        JsonRpcResponse {
            data,
            warnings: None,
        }
    }

    /// Helper method that returns a JsonRpcResponse with the given data and warnings.
    /// If the warnings vector is empty, no warning field is added to the JSON-RPC response.
    pub fn warn(data: T, warnings: Vec<JsonRpcWarning>) -> Self {
        JsonRpcResponse {
            data,
            warnings: if warnings.is_empty() {
                None
            } else {
                Some(warnings)
            },
        }
    }
}

impl JsonRpcWarning {
    pub fn new<S: Serialize>(code: i32, message: String, data: Option<S>) -> Self {
        JsonRpcWarning {
            code,
            message,
            data: data.and_then(|d| serde_json::to_value(&d).ok()),
        }
    }
}
