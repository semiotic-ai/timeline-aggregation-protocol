// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

// These tests simulate a Sender sending query requests and receipts to one or two Indexers.
// The tests use a mock Indexer server running a tap_manager instance and a tap_aggregator to handle RAV requests.
// An Indexer checks and stores receipts. After receiving a specific number of receipts, the Indexer sends a RAV request to the aggregator.
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    net::{SocketAddr, TcpListener},
    str::FromStr,
    sync::Arc,
};

use alloy_primitives::Address;
use alloy_sol_types::Eip712Domain;
use anyhow::{Error, Result};
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
use jsonrpsee::{
    core::client::ClientT, http_client::HttpClientBuilder, rpc_params, server::ServerHandle,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rstest::*;
use tokio::sync::RwLock;

use tap_aggregator::{jsonrpsee_helpers, server as agg_server};
use tap_core::{
    adapters::executor_mock::{
        EscrowStorage, ExecutorMock, QueryAppraisals, RAVStorage, ReceiptStorage,
    },
    checks::{mock::get_full_list_of_checks, ReceiptCheck},
    eip_712_signed_message::EIP712SignedMessage,
    tap_eip712_domain,
    tap_manager::SignedRAV,
    tap_receipt::Receipt,
};

use crate::indexer_mock;

// Fixtures for sender aggregator server
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
    2
}

#[fixture]
fn aggregate_server_api_version() -> String {
    "0.0".to_string()
}

// Test parameters: num_queries is the number of unique "queries" available to a client
#[fixture]
fn num_queries() -> u64 {
    16
}

// Test parameter: num_batches is the number of batches of queries that will be sent to the aggregator
// Total queries sent to the aggregator = num_queries * num_batches
#[fixture]
fn num_batches() -> u64 {
    10
}

// The number of receipts collected before Indexer 1 sends a RAV request
#[fixture]
fn receipt_threshold_1(num_queries: u64, num_batches: u64) -> u64 {
    num_queries * num_batches / 4
}

// The number of receipts collected before Indexer 2 sends a RAV request
#[fixture]
fn receipt_threshold_2(num_queries: u64, num_batches: u64) -> u64 {
    num_queries * num_batches / 2
}

// The private key (LocalWallet) and public key (Address) of a Sender
#[fixture]
fn keys_sender() -> (LocalWallet, Address) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
    .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
    .build()
    .unwrap();
    // Alloy library does not have feature parity with ethers library (yet) This workaround is needed to get the address
    // to convert to an alloy Address. This will not be needed when the alloy library has wallet support.
    let address: [u8; 20] = wallet.address().into();

    (wallet, address.into())
}

// The private key (LocalWallet) and public key (Address) of a Sender. This key is used to test when the Sender's key differs from the Indexer's expectation.
#[fixture]
fn wrong_keys_sender() -> (LocalWallet, Address) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
        .phrase("devote force reopen galaxy humor virtual hobby chief grit nothing bag pulse")
        .build()
        .unwrap();
    // Alloy library does not have feature parity with ethers library (yet) This workaround is needed to get the address
    // to convert to an alloy Address. This will not be needed when the alloy library has wallet support.
    let address: [u8; 20] = wallet.address().into();

    (wallet, address.into())
}

// Allocation IDs are used to ensure receipts cannot be double-counted
#[fixture]
fn allocation_ids() -> Vec<Address> {
    vec![
        Address::from_str("0xabababababababababababababababababababab").unwrap(),
        Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead").unwrap(),
    ]
}

#[fixture]
fn sender_ids() -> Vec<Address> {
    vec![
        Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
        Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
        Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
        keys_sender().1,
    ]
}

// Domain separator is used to sign receipts/RAVs according to EIP-712
#[fixture]
fn domain_separator() -> Eip712Domain {
    tap_eip712_domain(1, Address::from([0x11u8; 20]))
}

// Query price will typically be set by the Indexer. It's assumed to be part of the Indexer service.
#[fixture]
#[once]
fn query_price() -> &'static [u128] {
    let seed: Vec<u8> = (0..32u8).collect(); // A seed of your choice
    let mut rng: StdRng = SeedableRng::from_seed(seed.try_into().unwrap());
    let mut v = Vec::new();

    for _ in 0..num_queries() {
        v.push(rng.gen::<u128>() % 100);
    }
    Box::leak(v.into_boxed_slice())
}

// Available escrow is set by a Sender. It's assumed the Indexer has way of knowing this value.
#[fixture]
fn available_escrow(query_price: &[u128], num_batches: u64) -> u128 {
    (num_batches as u128) * query_price.iter().sum::<u128>()
}

#[fixture]
fn receipt_storage() -> ReceiptStorage {
    Arc::new(RwLock::new(HashMap::new()))
}

#[fixture]
fn query_appraisals(query_price: &[u128]) -> QueryAppraisals {
    Arc::new(RwLock::new(
        query_price
            .iter()
            .enumerate()
            .map(|(i, p)| (i as u64, *p))
            .collect(),
    ))
}

#[fixture]
fn rav_storage() -> RAVStorage {
    Arc::new(RwLock::new(None))
}

#[fixture]
fn escrow_storage() -> EscrowStorage {
    Arc::new(RwLock::new(HashMap::new()))
}

struct ExecutorFixture {
    executor: ExecutorMock,
    checks: Vec<ReceiptCheck>,
}

#[fixture]
fn executor(
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
    sender_ids: Vec<Address>,
    receipt_storage: ReceiptStorage,
    query_appraisals: QueryAppraisals,
    rav_storage: RAVStorage,
    escrow_storage: EscrowStorage,
) -> ExecutorFixture {
    let executor = ExecutorMock::new(rav_storage, receipt_storage.clone(), escrow_storage.clone());

    let checks = get_full_list_of_checks(
        domain_separator,
        sender_ids.iter().cloned().collect(),
        Arc::new(RwLock::new(allocation_ids.iter().cloned().collect())),
        receipt_storage,
        query_appraisals,
    );

    ExecutorFixture { executor, checks }
}

#[fixture]
fn indexer_1_adapters(executor: ExecutorFixture) -> ExecutorFixture {
    executor
}

#[fixture]
fn indexer_2_adapters(executor: ExecutorFixture) -> ExecutorFixture {
    executor
}

// Helper fixture to generate a batch of receipts to be sent to the Indexer.
// Messages are formatted according to TAP spec and signed according to EIP-712.
#[fixture]
fn requests_1(
    keys_sender: (LocalWallet, Address),
    query_price: &[u128],
    num_batches: u64,
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let (sender_key, _) = keys_sender;
    // Create your Receipt here
    let requests = generate_requests(
        query_price,
        num_batches,
        &sender_key,
        allocation_ids[0],
        &domain_separator,
    )?;
    Ok(requests)
}

#[fixture]
fn requests_2(
    keys_sender: (LocalWallet, Address),
    query_price: &[u128],
    num_batches: u64,
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let (sender_key, _) = keys_sender;
    // Create your Receipt here
    let requests = generate_requests(
        query_price,
        num_batches,
        &sender_key,
        allocation_ids[1],
        &domain_separator,
    )?;
    Ok(requests)
}

#[fixture]
fn repeated_timestamp_request(
    keys_sender: (LocalWallet, Address),
    query_price: &[u128],
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    num_batches: u64,
    receipt_threshold_1: u64,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let (sender_key, _) = keys_sender;

    // Create signed receipts
    let mut requests = generate_requests(
        query_price,
        num_batches,
        &sender_key,
        allocation_ids[0],
        &domain_separator,
    )?;

    // Create a new receipt with the timestamp equal to the latest receipt in the first RAV request batch
    let repeat_timestamp = requests[receipt_threshold_1 as usize - 1]
        .0
        .message
        .timestamp_ns;
    let target_receipt = &requests[receipt_threshold_1 as usize].0.message;
    let repeat_receipt = Receipt {
        allocation_id: target_receipt.allocation_id,
        timestamp_ns: repeat_timestamp,
        nonce: target_receipt.nonce,
        value: target_receipt.value,
    };

    // Sign the new receipt and insert it in the second batch
    requests[receipt_threshold_1 as usize].0 =
        EIP712SignedMessage::new(&domain_separator, repeat_receipt, &sender_key)?;
    Ok(requests)
}

#[fixture]
fn repeated_timestamp_incremented_by_one_request(
    keys_sender: (LocalWallet, Address),
    query_price: &[u128],
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    num_batches: u64,
    receipt_threshold_1: u64,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let (sender_key, _) = keys_sender;
    // Create your Receipt here
    let mut requests = generate_requests(
        query_price,
        num_batches,
        &sender_key,
        allocation_ids[0],
        &domain_separator,
    )?;

    // Create a new receipt with the timestamp equal to the latest receipt timestamp+1 in the first RAV request batch
    let repeat_timestamp = requests[receipt_threshold_1 as usize - 1]
        .0
        .message
        .timestamp_ns
        + 1;
    let target_receipt = &requests[receipt_threshold_1 as usize].0.message;
    let repeat_receipt = Receipt {
        allocation_id: target_receipt.allocation_id,
        timestamp_ns: repeat_timestamp,
        nonce: target_receipt.nonce,
        value: target_receipt.value,
    };

    // Sign the new receipt and insert it in the second batch
    requests[receipt_threshold_1 as usize].0 =
        EIP712SignedMessage::new(&domain_separator, repeat_receipt, &sender_key)?;
    Ok(requests)
}

#[fixture]
fn wrong_requests(
    wrong_keys_sender: (LocalWallet, Address),
    query_price: &[u128],
    num_batches: u64,
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let (sender_key, _) = wrong_keys_sender;
    // Create your Receipt here
    // Create your Receipt here
    let requests = generate_requests(
        query_price,
        num_batches,
        &sender_key,
        allocation_ids[0],
        &domain_separator,
    )?;
    Ok(requests)
}

// Helper fixtures to start servers for tests
#[fixture]
async fn single_indexer_test_server(
    keys_sender: (LocalWallet, Address),
    domain_separator: Eip712Domain,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
    indexer_1_adapters: ExecutorFixture,
    available_escrow: u128,
    receipt_threshold_1: u64,
) -> Result<(ServerHandle, SocketAddr, ServerHandle, SocketAddr)> {
    let sender_id = keys_sender.1;
    let (sender_aggregator_handle, sender_aggregator_addr) = start_sender_aggregator(
        keys_sender,
        domain_separator.clone(),
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let ExecutorFixture { executor, checks } = indexer_1_adapters;
    let (indexer_handle, indexer_addr) = start_indexer_server(
        domain_separator.clone(),
        executor,
        sender_id,
        available_escrow,
        checks.clone(),
        checks,
        receipt_threshold_1,
        sender_aggregator_addr,
    )
    .await?;
    Ok((
        indexer_handle,
        indexer_addr,
        sender_aggregator_handle,
        sender_aggregator_addr,
    ))
}

#[fixture]
async fn two_indexers_test_servers(
    keys_sender: (LocalWallet, Address),
    domain_separator: Eip712Domain,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
    indexer_1_adapters: ExecutorFixture,
    indexer_2_adapters: ExecutorFixture,
    available_escrow: u128,
    receipt_threshold_1: u64,
) -> Result<(
    ServerHandle,
    SocketAddr,
    ServerHandle,
    SocketAddr,
    ServerHandle,
    SocketAddr,
)> {
    let sender_id = keys_sender.1;
    let (sender_aggregator_handle, sender_aggregator_addr) = start_sender_aggregator(
        keys_sender,
        domain_separator.clone(),
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let ExecutorFixture {
        executor: executor_1,
        checks: checks_1,
    } = indexer_1_adapters;

    let ExecutorFixture {
        executor: executor_2,
        checks: checks_2,
    } = indexer_2_adapters;

    let (indexer_handle, indexer_addr) = start_indexer_server(
        domain_separator.clone(),
        executor_1,
        sender_id,
        available_escrow,
        checks_1.clone(),
        checks_1,
        receipt_threshold_1,
        sender_aggregator_addr,
    )
    .await?;

    let (indexer_handle_2, indexer_addr_2) = start_indexer_server(
        domain_separator.clone(),
        executor_2,
        sender_id,
        available_escrow,
        checks_2.clone(),
        checks_2,
        receipt_threshold_1,
        sender_aggregator_addr,
    )
    .await?;

    Ok((
        indexer_handle,
        indexer_addr,
        indexer_handle_2,
        indexer_addr_2,
        sender_aggregator_handle,
        sender_aggregator_addr,
    ))
}

#[fixture]
async fn single_indexer_wrong_sender_test_server(
    wrong_keys_sender: (LocalWallet, Address),
    domain_separator: Eip712Domain,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
    indexer_1_adapters: ExecutorFixture,
    available_escrow: u128,
    receipt_threshold_1: u64,
) -> Result<(ServerHandle, SocketAddr, ServerHandle, SocketAddr)> {
    let sender_id = wrong_keys_sender.1;
    let (sender_aggregator_handle, sender_aggregator_addr) = start_sender_aggregator(
        wrong_keys_sender,
        domain_separator.clone(),
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let ExecutorFixture {
        executor, checks, ..
    } = indexer_1_adapters;

    let (indexer_handle, indexer_addr) = start_indexer_server(
        domain_separator.clone(),
        executor,
        sender_id,
        available_escrow,
        checks.clone(),
        checks,
        receipt_threshold_1,
        sender_aggregator_addr,
    )
    .await?;

    Ok((
        indexer_handle,
        indexer_addr,
        sender_aggregator_handle,
        sender_aggregator_addr,
    ))
}

#[rstest]
#[tokio::test]
async fn test_manager_one_indexer(
    #[future] single_indexer_test_server: Result<
        (ServerHandle, SocketAddr, ServerHandle, SocketAddr),
        Error,
    >,
    requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_server_handle, socket_addr, _sender_handle, _sender_addr) =
        single_indexer_test_server.await?;
    let indexer_1_address = "http://".to_string() + &socket_addr.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let requests = requests_1?;

    for (receipt_1, id) in requests {
        let result = client_1.request("request", (id, receipt_1)).await;

        match result {
            Ok(()) => {}
            Err(e) => panic!("Error making receipt request: {:?}", e),
        }
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_manager_two_indexers(
    #[future] two_indexers_test_servers: Result<
        (
            ServerHandle,
            SocketAddr,
            ServerHandle,
            SocketAddr,
            ServerHandle,
            SocketAddr,
        ),
        Error,
    >,
    requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    requests_2: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
) -> Result<()> {
    let (
        _server_handle_1,
        socket_addr_1,
        _server_handle_2,
        socket_addr_2,
        _sender_handle,
        _sender_addr,
    ) = two_indexers_test_servers.await?;

    let indexer_1_address = "http://".to_string() + &socket_addr_1.to_string();
    let indexer_2_address = "http://".to_string() + &socket_addr_2.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let client_2 = HttpClientBuilder::default().build(indexer_2_address)?;
    let requests_1 = requests_1?;
    let requests_2 = requests_2?;

    for ((receipt_1, id_1), (receipt_2, id_2)) in requests_1.iter().zip(requests_2) {
        let future_1 = client_1.request("request", (id_1, receipt_1));
        let future_2 = client_2.request("request", (id_2, receipt_2));
        match tokio::try_join!(future_1, future_2) {
            Ok(((), ())) => {}
            Err(e) => panic!("Error making receipt request: {:?}", e),
        }
    }
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_manager_wrong_aggregator_keys(
    #[future] single_indexer_wrong_sender_test_server: Result<
        (ServerHandle, SocketAddr, ServerHandle, SocketAddr),
        Error,
    >,
    requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    receipt_threshold_1: u64,
) -> Result<()> {
    let (_server_handle, socket_addr, _sender_handle, _sender_addr) =
        single_indexer_wrong_sender_test_server.await?;
    let indexer_1_address = "http://".to_string() + &socket_addr.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let requests = requests_1?;

    let mut counter = 1;
    for (receipt_1, id) in requests {
        let result: Result<(), jsonrpsee::core::Error> =
            client_1.request("request", (id, receipt_1)).await;
        // The rav request is being made with messages that have been signed with a key that differs from the sender aggregator's.
        // So the Sender Aggregator should send an error to the requesting Indexer.
        // And so the Indexer should then return an error to the clinet when a rav request is made.
        // A rav request is made when the number of receipts sent = receipt_threshold_1.
        // result should be an error when counter = multiple of receipt_threshold_1 and Ok otherwise.
        if (counter % receipt_threshold_1) == 0 {
            assert!(
                result.is_err(),
                "Sender Aggregator should have sent an error to the Indexer."
            );
        } else {
            assert!(
                result.is_ok(),
                "Error making receipt request: {:?}",
                result.unwrap_err()
            );
        }
        counter += 1;
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_manager_wrong_requestor_keys(
    #[future] single_indexer_test_server: Result<
        (ServerHandle, SocketAddr, ServerHandle, SocketAddr),
        Error,
    >,
    wrong_requests: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    receipt_threshold_1: u64,
) -> Result<()> {
    let (_server_handle, socket_addr, _sender_handle, _sender_addr) =
        single_indexer_test_server.await?;
    let indexer_1_address = "http://".to_string() + &socket_addr.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let requests = wrong_requests?;

    let mut counter = 1;
    for (receipt_1, id) in requests {
        let result: Result<(), jsonrpsee::core::Error> =
            client_1.request("request", (id, receipt_1)).await;
        // The receipts have been signed with a key that the Indexer is not expecting.
        // So the Indexer should return an error when a rav request is made, because they will not have any valid receipts for the request.
        // A rav request is made when the number of receipts sent = receipt_threshold_1.
        // result should be an error when counter = multiple of receipt_threshold_1 and Ok otherwise.
        if (counter % receipt_threshold_1) == 0 {
            assert!(result.is_err(), "Should have failed signature verification");
        } else {
            assert!(
                result.is_ok(),
                "Error making receipt request: {:?}",
                result.unwrap_err()
            );
        }
        counter += 1;
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_tap_manager_rav_timestamp_cuttoff(
    #[future] two_indexers_test_servers: Result<
        (
            ServerHandle,
            SocketAddr,
            ServerHandle,
            SocketAddr,
            ServerHandle,
            SocketAddr,
        ),
        Error,
    >,
    repeated_timestamp_request: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    repeated_timestamp_incremented_by_one_request: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    receipt_threshold_1: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test checks that tap_core is correctly filtering receipts by timestamp.
    let (
        server_handle_1,
        socket_addr_1,
        _server_handle_2,
        socket_addr_2,
        _sender_handle,
        _sender_addr,
    ) = two_indexers_test_servers.await?;

    let indexer_1_address = "http://".to_string() + &socket_addr_1.to_string();
    let indexer_2_address = "http://".to_string() + &socket_addr_2.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let client_2 = HttpClientBuilder::default().build(indexer_2_address)?;
    let requests = repeated_timestamp_request?;

    let mut counter = 1;
    for (receipt_1, id) in requests {
        let result: Result<(), jsonrpsee::core::Error> =
            client_1.request("request", (id, receipt_1)).await;

        // The first receipt in the second batch has the same timestamp as the last receipt in the first batch.
        // TAP manager should ignore this receipt when creating the second RAV request.
        // The indexer_mock will throw an error if the number of receipts in RAV request is less than the expected number.
        // An error is expected when requesting the second RAV.
        if counter == 2 * receipt_threshold_1 {
            assert!(result.is_err(), "Should have failed RAV request");
        } else {
            assert!(
                result.is_ok(),
                "Error making receipt request: {:?}",
                result.unwrap_err()
            );
        }
        counter += 1;
    }

    server_handle_1.stop()?;

    // Here the timestamp first receipt in the second batch is equal to timestamp + 1 of the last receipt in the first batch.
    // No errors are expected.
    let requests = repeated_timestamp_incremented_by_one_request?;
    for (receipt_1, id) in requests {
        let result = client_2.request("request", (id, receipt_1)).await;
        match result {
            Ok(()) => {}
            Err(e) => panic!("Error making receipt request: {:?}", e),
        }
    }
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_tap_aggregator_rav_timestamp_cuttoff(
    keys_sender: (LocalWallet, Address),
    domain_separator: Eip712Domain,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
    repeated_timestamp_request: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    repeated_timestamp_incremented_by_one_request: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    receipt_threshold_1: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test checks that tap_aggregator is correctly rejecting receipts with invalid timestamps
    let (sender_handle, sender_addr) = start_sender_aggregator(
        keys_sender,
        domain_separator,
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let client = HttpClientBuilder::default().build(format!("http://{}", sender_addr))?;

    // This is the first part of the test, two batches of receipts are sent to the aggregator.
    // The second batch has one receipt with the same timestamp as the latest receipt in the first batch.
    // The first RAV will have the same timestamp as one receipt in the second batch.
    // tap_aggregator should reject the second RAV request due to the repeated timestamp.
    let requests = repeated_timestamp_request?;
    let first_batch = &requests[0..receipt_threshold_1 as usize];
    let second_batch = &requests[receipt_threshold_1 as usize..2 * receipt_threshold_1 as usize];

    let receipts = first_batch
        .iter()
        .map(|(r, _)| r.clone())
        .collect::<Vec<_>>();
    let params = rpc_params!(&aggregate_server_api_version(), &receipts, None::<()>);
    let first_rav_response: jsonrpsee_helpers::JsonRpcResponse<SignedRAV> =
        client.request("aggregate_receipts", params).await?;

    let receipts = second_batch
        .iter()
        .map(|(r, _)| r.clone())
        .collect::<Vec<_>>();
    let params = rpc_params!(
        &aggregate_server_api_version(),
        &receipts,
        first_rav_response.data
    );
    let second_rav_response: Result<
        jsonrpsee_helpers::JsonRpcResponse<SignedRAV>,
        jsonrpsee::core::Error,
    > = client.request("aggregate_receipts", params).await;
    assert!(
        second_rav_response.is_err(),
        "Should have failed RAV request"
    );

    // This is the second part of the test, two batches of receipts are sent to the aggregator.
    // The second batch has one receipt with the timestamp = timestamp+1 of the latest receipt in the first batch.
    // tap_aggregator should accept the second RAV request.
    let requests = repeated_timestamp_incremented_by_one_request?;
    let first_batch = &requests[0..receipt_threshold_1 as usize];
    let second_batch = &requests[receipt_threshold_1 as usize..2 * receipt_threshold_1 as usize];

    let receipts = first_batch
        .iter()
        .map(|(r, _)| r.clone())
        .collect::<Vec<_>>();
    let params = rpc_params!(&aggregate_server_api_version(), &receipts, None::<()>);
    let first_rav_response: jsonrpsee_helpers::JsonRpcResponse<SignedRAV> =
        client.request("aggregate_receipts", params).await?;

    let receipts = second_batch
        .iter()
        .map(|(r, _)| r.clone())
        .collect::<Vec<_>>();
    let params = rpc_params!(
        &aggregate_server_api_version(),
        &receipts,
        first_rav_response.data
    );
    let second_rav_response: jsonrpsee_helpers::JsonRpcResponse<SignedRAV> =
        client.request("aggregate_receipts", params).await?;

    // Compute the expected aggregate value and check that it matches the latest RAV.
    let mut expected_value = 0;
    for (receipt, _) in first_batch.iter().chain(second_batch.iter()) {
        expected_value += receipt.message.value;
    }
    assert!(expected_value == second_rav_response.data.message.valueAggregate);

    sender_handle.stop()?;
    Ok(())
}

fn generate_requests(
    query_price: &[u128],
    num_batches: u64,
    sender_key: &LocalWallet,
    allocation_id: Address,
    domain_separator: &Eip712Domain,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let mut requests: Vec<(EIP712SignedMessage<Receipt>, u64)> = Vec::new();

    let mut counter = 0;
    for _ in 0..num_batches {
        for value in query_price {
            requests.push((
                EIP712SignedMessage::new(
                    domain_separator,
                    Receipt::new(allocation_id, *value)?,
                    sender_key,
                )?,
                counter,
            ));
            counter += 1;
        }
        counter = 0;
    }

    Ok(requests)
}

// Start-up a mock Indexer. Requires a Sender Aggregator to be running.
async fn start_indexer_server(
    domain_separator: Eip712Domain,
    mut executor: ExecutorMock,
    sender_id: Address,
    available_escrow: u128,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold: u64,
    agg_server_addr: SocketAddr,
) -> Result<(ServerHandle, SocketAddr)> {
    let http_port = {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.local_addr()?.port()
    };

    executor.increase_escrow(sender_id, available_escrow).await;
    let aggregate_server_address = "http://".to_string() + &agg_server_addr.to_string();

    let (server_handle, socket_addr) = indexer_mock::run_server(
        http_port,
        domain_separator,
        executor,
        initial_checks,
        required_checks,
        receipt_threshold,
        aggregate_server_address,
        aggregate_server_api_version(),
        sender_id,
    )
    .await?;

    Ok((server_handle, socket_addr))
}

// Start-up a Sender Aggregator.
async fn start_sender_aggregator(
    keys: (LocalWallet, Address),
    domain_separator: Eip712Domain,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
) -> Result<(ServerHandle, SocketAddr)> {
    let http_port = {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.local_addr()?.port()
    };

    let accepted_addresses = HashSet::from([keys.1]);

    let (server_handle, socket_addr) = agg_server::run_server(
        http_port,
        keys.0,
        accepted_addresses,
        domain_separator,
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;

    Ok((server_handle, socket_addr))
}
