// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use anyhow::Result;
use ethers_signers::LocalWallet;
use jsonrpsee::{
    proc_macros::rpc,
    server::ServerBuilder,
    {core::async_trait, server::ServerHandle},
};

use crate::aggregator::check_and_aggregate_receipts;
use crate::api_versioning::{
    tap_rpc_api_versions_info, TapRpcApiVersion, TapRpcApiVersionsInfo,
    TAP_RPC_API_VERSIONS_DEPRECATED,
};
use crate::jsonrpsee_helpers::{JsonRpcError, JsonRpcResponse, JsonRpcResult, JsonRpcWarning};
use tap_core::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};

/// Generates the `RpcServer` trait that is used to define the JSON-RPC API.
///
/// Note that because of the way the `rpc` macro works, we cannot document the RpcServer trait here.
/// (So even this very docstring will not appear in the generated documentation...)
/// As a result, we document the JSON-RPC API in the `tap_aggregator/README.md` file.
/// Do not forget to update the documentation there if you make any changes to the JSON-RPC API.
#[rpc(server)]
pub trait Rpc {
    /// Returns the versions of the TAP JSON-RPC API implemented by this server.
    #[method(name = "api_versions")]
    async fn api_versions(&self) -> JsonRpcResult<TapRpcApiVersionsInfo>;

    /// Aggregates the given receipts into a receipt aggregate voucher.
    /// Returns an error if the user expected API version is not supported.
    #[method(name = "aggregate_receipts")]
    async fn aggregate_receipts(
        &self,
        api_version: String,
        receipts: Vec<EIP712SignedMessage<Receipt>>,
        previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    ) -> JsonRpcResult<EIP712SignedMessage<ReceiptAggregateVoucher>>;
}

struct RpcImpl {
    wallet: LocalWallet,
}

/// Helper method that checks if the given API version is supported.
/// Returns an error if the API version is not supported.
fn parse_api_version(api_version: &str) -> Result<TapRpcApiVersion, JsonRpcError> {
    TapRpcApiVersion::from_str(api_version).map_err(|_| {
        jsonrpsee::types::ErrorObject::owned(
            -32001,
            format!("Unsupported API version: \"{}\".", api_version),
            Some(tap_rpc_api_versions_info()),
        )
    })
}

/// Helper method that checks if the given API version has a deprecation warning.
/// Returns a warning if the API version is deprecated.
fn check_api_version_deprecation(api_version: &TapRpcApiVersion) -> Option<JsonRpcWarning> {
    if TAP_RPC_API_VERSIONS_DEPRECATED.contains(api_version) {
        Some(JsonRpcWarning::new(
            -32002,
            format!(
                "The API version {} will be deprecated. \
                Please check https://github.com/semiotic-ai/timeline_aggregation_protocol for more information.",
                api_version
            ),
            Some(tap_rpc_api_versions_info()),
        ))
    } else {
        None
    }
}

#[async_trait]
impl RpcServer for RpcImpl {
    async fn api_versions(&self) -> JsonRpcResult<TapRpcApiVersionsInfo> {
        Ok(JsonRpcResponse::ok(tap_rpc_api_versions_info()))
    }

    async fn aggregate_receipts(
        &self,
        api_version: String,
        receipts: Vec<EIP712SignedMessage<Receipt>>,
        previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    ) -> JsonRpcResult<EIP712SignedMessage<ReceiptAggregateVoucher>> {
        // Return an error if the API version is not supported.
        let api_version = parse_api_version(api_version.as_str())?;

        // Add a warning if the API version is to be deprecated.
        let mut warnings: Vec<JsonRpcWarning> = Vec::new();
        if let Some(w) = check_api_version_deprecation(&api_version) {
            warnings.push(w);
        }

        let res = match api_version {
            TapRpcApiVersion::V0_0 => {
                check_and_aggregate_receipts(&receipts, previous_rav, &self.wallet).await
            }
        };

        // Handle aggregation error
        match res {
            Ok(res) => Ok(JsonRpcResponse::warn(res, warnings)),
            Err(e) => Err(jsonrpsee::types::ErrorObject::owned(
                -32000,
                e.to_string(),
                None::<()>,
            )),
        }
    }
}

pub async fn run_server(
    port: u16,
    wallet: LocalWallet,
    max_request_body_size: u32,
    max_response_body_size: u32,
    max_concurrent_connections: u32,
) -> Result<(ServerHandle, std::net::SocketAddr)> {
    // Setting up the JSON RPC server
    println!("Starting server...");
    let server = ServerBuilder::new()
        .max_request_body_size(max_request_body_size)
        .max_response_body_size(max_response_body_size)
        .max_connections(max_concurrent_connections)
        .http_only()
        .build(format!("127.0.0.1:{}", port))
        .await?;
    let addr = server.local_addr()?;
    println!("Listening on: {}", addr);
    let rpc_impl = RpcImpl { wallet };
    let handle = server.start(rpc_impl.into_rpc())?;
    Ok((handle, addr))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ethers_core::types::Address;
    use ethers_signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
    use rstest::*;

    use crate::server;
    use tap_core::{
        eip_712_signed_message::EIP712SignedMessage,
        receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
    };

    #[fixture]
    fn keys() -> (LocalWallet, Address) {
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
        let address = wallet.address();
        (wallet, address)
    }

    #[fixture]
    fn allocation_ids() -> Vec<Address> {
        vec![
            Address::from_str("0xabababababababababababababababababababab").unwrap(),
            Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead").unwrap(),
            Address::from_str("0xbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef").unwrap(),
            Address::from_str("0x1234567890abcdef1234567890abcdef12345678").unwrap(),
        ]
    }

    #[fixture]
    fn http_request_size_limit() -> u32 {
        100 * 1024
    }

    #[fixture]
    fn http_response_size_limit() -> u32 {
        100 * 1024
    }

    #[fixture]
    fn http_max_concurrent_connections() -> u32 {
        1
    }

    #[rstest]
    #[tokio::test]
    async fn protocol_version(
        keys: (LocalWallet, Address),
        http_request_size_limit: u32,
        http_response_size_limit: u32,
        http_max_concurrent_connections: u32,
    ) {
        // Start the JSON-RPC server.
        let (handle, local_addr) = server::run_server(
            0,
            keys.0,
            http_request_size_limit,
            http_response_size_limit,
            http_max_concurrent_connections,
        )
        .await
        .unwrap();

        // Start the JSON-RPC client.
        let client = HttpClientBuilder::default()
            .build(format!("http://127.0.0.1:{}", local_addr.port()))
            .unwrap();
        let res: server::JsonRpcResponse<server::TapRpcApiVersionsInfo> = client
            .request("api_versions", rpc_params!(None::<()>))
            .await
            .unwrap();

        println!("{:?}", res);

        handle.stop().unwrap();
        handle.stopped().await;
    }

    #[rstest]
    #[case::basic_rav_test (vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts (vec![0,0,0,0])]
    #[tokio::test]
    async fn signed_rav_is_valid_with_no_previous_rav(
        keys: (LocalWallet, Address),
        http_request_size_limit: u32,
        http_response_size_limit: u32,
        http_max_concurrent_connections: u32,
        allocation_ids: Vec<Address>,
        #[case] values: Vec<u128>,
        #[values("0.0")] api_version: &str,
    ) {
        // Start the JSON-RPC server.
        let (handle, local_addr) = server::run_server(
            0,
            keys.0.clone(),
            http_request_size_limit,
            http_response_size_limit,
            http_max_concurrent_connections,
        )
        .await
        .unwrap();

        // Start the JSON-RPC client.
        let client = HttpClientBuilder::default()
            .build(format!("http://127.0.0.1:{}", local_addr.port()))
            .unwrap();

        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .await
                    .unwrap(),
            );
        }

        // Skipping receipts validation in this test, aggregate_receipts assumes receipts are valid.
        // Create RAV through the JSON-RPC server.
        let res: server::JsonRpcResponse<EIP712SignedMessage<ReceiptAggregateVoucher>> = client
            .request(
                "aggregate_receipts",
                rpc_params!(api_version, &receipts, None::<()>),
            )
            .await
            .unwrap();

        let remote_rav = res.data;

        let local_rav =
            ReceiptAggregateVoucher::aggregate_receipts(allocation_ids[0], &receipts, None)
                .unwrap();

        assert!(remote_rav.message.allocation_id == local_rav.allocation_id);
        assert!(remote_rav.message.timestamp_ns == local_rav.timestamp_ns);
        assert!(remote_rav.message.value_aggregate == local_rav.value_aggregate);

        assert!(remote_rav.recover_signer().unwrap() == keys.1);

        handle.stop().unwrap();
        handle.stopped().await;
    }

    #[rstest]
    #[case::basic_rav_test (vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts (vec![0,0,0,0])]
    #[tokio::test]
    async fn signed_rav_is_valid_with_previous_rav(
        keys: (LocalWallet, Address),
        http_request_size_limit: u32,
        http_response_size_limit: u32,
        http_max_concurrent_connections: u32,
        allocation_ids: Vec<Address>,
        #[case] values: Vec<u128>,
        #[values("0.0")] api_version: &str,
    ) {
        // Start the JSON-RPC server.
        let (handle, local_addr) = server::run_server(
            0,
            keys.0.clone(),
            http_request_size_limit,
            http_response_size_limit,
            http_max_concurrent_connections,
        )
        .await
        .unwrap();

        // Start the JSON-RPC client.
        let client = HttpClientBuilder::default()
            .build(format!("http://127.0.0.1:{}", local_addr.port()))
            .unwrap();

        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .await
                    .unwrap(),
            );
        }

        // Create previous RAV from first half of receipts locally
        let prev_rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[0..receipts.len() / 2],
            None,
        )
        .unwrap();
        let signed_prev_rav = EIP712SignedMessage::new(prev_rav, &keys.0).await.unwrap();

        // Create new RAV from last half of receipts and prev_rav through the JSON-RPC server
        let res: server::JsonRpcResponse<EIP712SignedMessage<ReceiptAggregateVoucher>> = client
            .request(
                "aggregate_receipts",
                rpc_params!(
                    api_version,
                    &receipts[receipts.len() / 2..receipts.len()],
                    Some(signed_prev_rav)
                ),
            )
            .await
            .unwrap();

        let rav = res.data;

        assert!(rav.recover_signer().unwrap() == keys.1);

        handle.stop().unwrap();
        handle.stopped().await;
    }

    #[rstest]
    #[tokio::test]
    async fn invalid_api_version(
        keys: (LocalWallet, Address),
        http_request_size_limit: u32,
        http_response_size_limit: u32,
        http_max_concurrent_connections: u32,
        allocation_ids: Vec<Address>,
    ) {
        // Start the JSON-RPC server.
        let (handle, local_addr) = server::run_server(
            0,
            keys.0.clone(),
            http_request_size_limit,
            http_response_size_limit,
            http_max_concurrent_connections,
        )
        .await
        .unwrap();

        // Start the JSON-RPC client.
        let client = HttpClientBuilder::default()
            .build(format!("http://127.0.0.1:{}", local_addr.port()))
            .unwrap();

        // Create receipts
        let receipts =
            vec![
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                    .await
                    .unwrap(),
            ];

        // Skipping receipts validation in this test, aggregate_receipts assumes receipts are valid.
        // Create RAV through the JSON-RPC server.
        let res: Result<
            server::JsonRpcResponse<EIP712SignedMessage<ReceiptAggregateVoucher>>,
            jsonrpsee::core::Error,
        > = client
            .request(
                "aggregate_receipts",
                rpc_params!("invalid version string", &receipts, None::<()>),
            )
            .await;

        println!("{:#?}", res);

        assert!(res.is_err());

        // Make sure the JSON-RPC error is "invalid version"
        assert!(res
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Unsupported API version"));

        // Check the API versions returned by the server
        match res.expect_err("Expected an error") {
            jsonrpsee::core::Error::Call(err) => {
                let versions: server::TapRpcApiVersionsInfo =
                    serde_json::from_str(err.data().unwrap().get()).unwrap();
                assert!(versions
                    .versions_supported
                    .contains(&server::TapRpcApiVersion::V0_0));
            }
            _ => panic!("Expected data in error"),
        }

        handle.stop().unwrap();
        handle.stopped().await;
    }

    #[rstest]
    #[tokio::test]
    async fn request_size_limit(
        keys: (LocalWallet, Address),
        http_response_size_limit: u32,
        http_max_concurrent_connections: u32,
        allocation_ids: Vec<Address>,
        #[values("0.0")] api_version: &str,
    ) {
        // Set the request size limit to 10 kB to easily trigger the HTTP 413 error.
        let small_request_size_limit = 10 * 1024;

        // Start the JSON-RPC server.
        let (handle, local_addr) = server::run_server(
            0,
            keys.0.clone(),
            small_request_size_limit,
            http_response_size_limit,
            http_max_concurrent_connections,
        )
        .await
        .unwrap();

        // Start the JSON-RPC client.
        let client = HttpClientBuilder::default()
            .build(format!("http://127.0.0.1:{}", local_addr.port()))
            .unwrap();

        // Create 100 receipts
        let mut receipts = Vec::new();
        for _ in 1..100 {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], 42).unwrap(), &keys.0)
                    .await
                    .unwrap(),
            );
        }

        // Skipping receipts validation in this test, aggregate_receipts assumes receipts are valid.
        // Create RAV through the JSON-RPC server.
        // Test with only 10 receipts
        let res: Result<
            server::JsonRpcResponse<EIP712SignedMessage<ReceiptAggregateVoucher>>,
            jsonrpsee::core::Error,
        > = client
            .request(
                "aggregate_receipts",
                rpc_params!(api_version, &receipts[..10], None::<()>),
            )
            .await;

        assert!(res.is_ok());

        // Create RAV through the JSON-RPC server.
        // Test with all 100 receipts
        let res: Result<
            server::JsonRpcResponse<EIP712SignedMessage<ReceiptAggregateVoucher>>,
            jsonrpsee::core::Error,
        > = client
            .request(
                "aggregate_receipts",
                rpc_params!(api_version, &receipts, None::<()>),
            )
            .await;

        assert!(res.is_err());
        // Make sure the error is a HTTP 413 Content Too Large
        assert!(res.unwrap_err().to_string().contains("413"));

        handle.stop().unwrap();
        handle.stopped().await;
    }
}
