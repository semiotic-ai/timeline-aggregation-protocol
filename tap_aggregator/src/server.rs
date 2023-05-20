// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use ethers_signers::LocalWallet;
use jsonrpsee::{
    proc_macros::rpc,
    server::ServerBuilder,
    {core::async_trait, server::ServerHandle},
};

use crate::aggregator::check_and_aggregate_receipts;
use tap_core::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};

/// The version of the TAP JSON-RPC API implemented by this server.
/// This version number is independent of the TAP software version. As such, we are
/// enabling the introduction of breaking changes to the TAP library interface without
/// necessarily introducing breaking changes to the JSON-RPC API (or vice versa).
const TAP_RPC_API_VERSIONS: &[&str] = &["0.0"];

/// List of RPC version numbers for which a deprecation warning has to be issued.
/// This is a very basic approach to deprecation warnings. The most important thing
/// is to have *some* process in place to warn users of breaking changes.
const TAP_RPC_API_VERSIONS_DEPRECATION_WARNING: &[&str] = &[];

#[rpc(server)]
pub trait Rpc {
    /// Returns the versions of the TAP JSON-RPC API implemented by this server.
    #[method(name = "api_versions")]
    async fn api_versions(&self) -> Result<Vec<String>, jsonrpsee::types::ErrorObjectOwned>;

    /// Checks if the given API version is deprecated and returns a warning message if so.
    /// If the API version is not deprecated, returns `None`.
    /// We expect clients to call this method occasionally to check if their API version
    /// is to be deprecated in the near future.
    #[method(name = "api_deprecation_check")]
    async fn api_version_deprecation_check(
        &self,
        api_version: String,
    ) -> Result<Option<String>, jsonrpsee::types::ErrorObjectOwned>;

    /// Aggregates the given receipts into a receipt aggregate voucher.
    /// Returns an error if the user expected API version is not supported.
    #[method(name = "aggregate_receipts")]
    async fn aggregate_receipts(
        &self,
        api_version: String,
        receipts: Vec<EIP712SignedMessage<Receipt>>,
        previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    ) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>, jsonrpsee::types::ErrorObjectOwned>;
}

struct RpcImpl {
    wallet: LocalWallet,
}

/// Helper method that checks if the given API version is supported.
/// Returns an error if the API version is not supported.
fn check_api_version(api_version: &str) -> Result<(), jsonrpsee::types::ErrorObjectOwned> {
    if !TAP_RPC_API_VERSIONS.contains(&api_version) {
        return Err(jsonrpsee::types::ErrorObject::owned(
            -32000,
            format!(
                "Unsupported API version: {}. Supported versions: {}",
                api_version,
                TAP_RPC_API_VERSIONS
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            None::<()>,
        ));
    }
    Ok(())
}

#[async_trait]
impl RpcServer for RpcImpl {
    async fn api_versions(&self) -> Result<Vec<String>, jsonrpsee::types::ErrorObjectOwned> {
        Ok(TAP_RPC_API_VERSIONS.iter().map(|s| s.to_string()).collect())
    }

    async fn api_version_deprecation_check(
        &self,
        api_version: String,
    ) -> Result<Option<String>, jsonrpsee::types::ErrorObjectOwned> {
        if TAP_RPC_API_VERSIONS_DEPRECATION_WARNING.contains(&api_version.as_str()) {
            Ok(Some(format!(
                "The API version {} will be deprecated. \
                Please check https://github.com/semiotic-ai/timeline_aggregation_protocol for more information.",
                api_version
            )))
        } else {
            Ok(None)
        }
    }

    async fn aggregate_receipts(
        &self,
        api_version: String,
        receipts: Vec<EIP712SignedMessage<Receipt>>,
        previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    ) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>, jsonrpsee::types::ErrorObjectOwned>
    {
        check_api_version(api_version.as_str())?; // check if the API version is supported

        let res = check_and_aggregate_receipts(&receipts, previous_rav, &self.wallet).await;
        // handle aggregation error
        match res {
            Ok(res) => Ok(res),
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
        let res: Result<Vec<String>, jsonrpsee::core::Error> = client
            .request("api_versions", rpc_params!(None::<()>))
            .await;

        handle.stop().unwrap();
        handle.stopped().await;

        res.unwrap();
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
        let remote_rav: EIP712SignedMessage<ReceiptAggregateVoucher> = client
            .request(
                "aggregate_receipts",
                rpc_params!(api_version, &receipts, None::<()>),
            )
            .await
            .unwrap();

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
        let rav: EIP712SignedMessage<ReceiptAggregateVoucher> = client
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

        assert!(rav.recover_signer().unwrap() == keys.1);

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
        let res: Result<EIP712SignedMessage<ReceiptAggregateVoucher>, jsonrpsee::core::Error> =
            client
                .request(
                    "aggregate_receipts",
                    rpc_params!(api_version, &receipts[..10], None::<()>),
                )
                .await;

        assert!(res.is_ok());

        // Create RAV through the JSON-RPC server.
        // Test with all 100 receipts
        let res: Result<EIP712SignedMessage<ReceiptAggregateVoucher>, jsonrpsee::core::Error> =
            client
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
