// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use tap_aggregator::{
    grpc::v1::{tap_aggregator_client::TapAggregatorClient, RavRequest},
    jsonrpsee_helpers::JsonRpcResponse,
    server,
};
use tap_core::{signed_message::Eip712SignedMessage, tap_eip712_domain};
#[cfg(feature = "v2")]
use tap_graph::v2::{Receipt, ReceiptAggregateVoucher};
#[cfg(not(feature = "v2"))]
use tap_graph::{Receipt, ReceiptAggregateVoucher};
use thegraph_core::alloy::{
    primitives::{address, Address, U256},
    signers::local::PrivateKeySigner,
};
use tonic::codec::CompressionEncoding;

#[tokio::test]
async fn aggregation_test() {
    let domain_separator = tap_eip712_domain(1, Address::ZERO);

    let wallet = PrivateKeySigner::random();

    let max_request_body_size = 1024 * 100;
    let max_response_body_size = 1024 * 100;
    let max_concurrent_connections = 1;

    let accepted_addresses = HashSet::from([wallet.address()]);

    let (join_handle, local_addr) = server::run_server(
        0,
        wallet.clone(),
        accepted_addresses,
        domain_separator.clone(),
        max_request_body_size,
        max_response_body_size,
        max_concurrent_connections,
        None,
    )
    .await
    .unwrap();

    let endpoint = format!("http://127.0.0.1:{}", local_addr.port());

    let mut client = TapAggregatorClient::connect(endpoint.clone())
        .await
        .unwrap()
        .send_compressed(CompressionEncoding::Zstd);

    let allocation_id = address!("0xabababababababababababababababababababab");
    let payer = address!("0xabababababababababababababababababababab");
    let data_service = address!("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead");
    let service_provider = address!("0xbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef");

    // Use a fixed timestamp to ensure both v1 and v2 receipts have the same timestamps
    let fixed_timestamp = 1700000000000000000u64; // Fixed timestamp in nanoseconds

    // Create v1 receipts for gRPC v1 compatibility
    let mut v1_receipts = Vec::new();
    for (i, value) in (50..60).enumerate() {
        let mut receipt = tap_graph::Receipt::new(allocation_id, U256::from(value)).unwrap();
        receipt.timestamp_ns = fixed_timestamp + i as u64; // Ensure increasing timestamps
        v1_receipts.push(Eip712SignedMessage::new(&domain_separator, receipt, &wallet).unwrap());
    }

    let rav_request = RavRequest::new(v1_receipts, None);
    let res = client.aggregate_receipts(rav_request).await.unwrap();
    let signed_rav: tap_graph::SignedRav = res.into_inner().signed_rav().unwrap();

    // Create v2 receipts for JSON-RPC API with the same timestamps
    let mut v2_receipts = Vec::new();
    for (i, value) in (50..60).enumerate() {
        #[cfg(feature = "v2")]
        {
            let mut receipt = Receipt::new(
                allocation_id,
                payer,
                data_service,
                service_provider,
                U256::from(value),
            )
            .unwrap();
            receipt.timestamp_ns = fixed_timestamp + i as u64; // Same timestamps as v1
            v2_receipts
                .push(Eip712SignedMessage::new(&domain_separator, receipt, &wallet).unwrap());
        }
        #[cfg(not(feature = "v2"))]
        {
            let mut receipt = Receipt::new(allocation_id, value).unwrap();
            receipt.timestamp_ns = fixed_timestamp + i as u64;
            v2_receipts
                .push(Eip712SignedMessage::new(&domain_separator, receipt, &wallet).unwrap());
        }
    }

    let sender_aggregator = HttpClientBuilder::default().build(&endpoint).unwrap();

    let previous_rav: Option<tap_graph::SignedRav> = None;

    let response: JsonRpcResponse<Eip712SignedMessage<ReceiptAggregateVoucher>> = sender_aggregator
        .request(
            "aggregate_receipts",
            rpc_params!(
                "0.0", // TODO: Set the version in a smarter place.
                v2_receipts,
                previous_rav
            ),
        )
        .await
        .unwrap();
    let response = response.data;
    // Compare the core fields since the types might differ between v1 and v2
    assert_eq!(
        signed_rav.message.allocationId,
        response.message.allocationId
    );
    assert_eq!(signed_rav.message.timestampNs, response.message.timestampNs);
    assert_eq!(
        signed_rav.message.valueAggregate,
        response.message.valueAggregate
    );
    join_handle.abort();
}
