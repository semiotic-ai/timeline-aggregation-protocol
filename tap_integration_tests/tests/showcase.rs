// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

// These tests simulate a Gateway sending query requests and receipts to one or two Indexers.
// The tests use a mock Indexer server running a tap_manager instance and a tap_aggregator to handle RAV requests.
// An Indexer checks and stores receipts. After receiving a specific number of receipts, the Indexer sends a RAV request to the aggregator.
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    iter::FromIterator,
    net::{SocketAddr, TcpListener},
    str::FromStr,
    sync::{Arc, RwLock},
};

use anyhow::{Error, Result};
use ethers::{
    signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer},
    types::{Address, H160},
};
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, server::ServerHandle};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rstest::*;

use tap_aggregator::server as agg_server;
use tap_core::{
    adapters::{
        collateral_adapter_mock::CollateralAdapterMock,
        rav_storage_adapter_mock::RAVStorageAdapterMock,
        receipt_checks_adapter_mock::ReceiptChecksAdapterMock,
        receipt_storage_adapter_mock::ReceiptStorageAdapterMock,
    },
    eip_712_signed_message::EIP712SignedMessage,
    tap_receipt::ReceiptCheck,
    tap_receipt::{Receipt, ReceivedReceipt},
};

use crate::indexer_mock;

// Fixtures for gateway aggregator server
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
    num_queries * num_batches
}

// The private key (LocalWallet) and public key (Address) of a Gateway
#[fixture]
fn keys_gateway() -> (LocalWallet, Address) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
    .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
    .build()
    .unwrap();
    let address = wallet.address();
    (wallet, address)
}

// The private key (LocalWallet) and public key (Address) of a Gateway. This key is used to test when the Gateway's key differs from the Indexer's expectation.
#[fixture]
fn wrong_keys_gateway() -> (LocalWallet, Address) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
        .phrase("devote force reopen galaxy humor virtual hobby chief grit nothing bag pulse")
        .build()
        .unwrap();
    let address = wallet.address();
    (wallet, address)
}

// Allocation IDs are used to ensure receipts cannot be double-counted
#[fixture]
fn allocation_ids() -> Vec<Address> {
    vec![
        Address::from_str("0xabababababababababababababababababababab").unwrap(),
        Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead").unwrap(),
    ]
}

// Query price will typically be set by the Indexer. It's assumed to be part of the Indexer service.
#[fixture]
fn query_price() -> Vec<u128> {
    let seed: Vec<u8> = (0..32u8).collect(); // A seed of your choice
    let mut rng: StdRng = SeedableRng::from_seed(seed.try_into().unwrap());
    let mut v = Vec::new();

    for _ in 0..num_queries() {
        v.push(rng.gen::<u128>() % 100);
    }
    v
}

// Available collateral is set by a Gateway. It's assumed the Indexer has way of knowing this value.
#[fixture]
fn available_collateral(query_price: Vec<u128>, num_batches: u64) -> u128 {
    (num_batches as u128) * query_price.into_iter().sum::<u128>()
}

// The collateral adapter, a storage struct that the Indexer uses to track the available collateral for each Gateway
#[fixture]
fn collateral_adapter() -> CollateralAdapterMock {
    CollateralAdapterMock::new(Arc::new(RwLock::new(HashMap::new())))
}

#[fixture]
fn receipt_storage() -> Arc<RwLock<HashMap<u64, ReceivedReceipt>>> {
    Arc::new(RwLock::new(HashMap::new()))
}
// A storage struct used by the Indexer to store Receipts, all recieved receipts are stored here. There are flags which indicate the validity of the receipt.
#[fixture]
fn receipt_storage_adapter(
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
) -> ReceiptStorageAdapterMock {
    ReceiptStorageAdapterMock::new(receipt_storage)
}

// This adapter is used by the Indexer to check the validity of the receipt.
#[fixture]
fn receipt_checks_adapter(
    keys_gateway: (LocalWallet, Address),
    query_price: Vec<u128>,
    allocation_ids: Vec<Address>,
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
) -> ReceiptChecksAdapterMock {
    let (_, gateway_address) = keys_gateway;
    let query_appraisals: HashMap<_, _> = (0u64..).zip(query_price).collect();
    let query_appraisals_storage = Arc::new(RwLock::new(query_appraisals));
    let allocation_ids: Arc<RwLock<HashSet<H160>>> =
        Arc::new(RwLock::new(HashSet::from_iter(allocation_ids)));
    let gateway_ids: Arc<RwLock<HashSet<H160>>> =
        Arc::new(RwLock::new(HashSet::from([gateway_address])));

    ReceiptChecksAdapterMock::new(
        receipt_storage,
        query_appraisals_storage,
        allocation_ids,
        gateway_ids,
    )
}

// A structure for storing received RAVs.
#[fixture]
fn rav_storage_adapter() -> RAVStorageAdapterMock {
    RAVStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new())))
}

// These are the checks that the Indexer will perform when requesting a RAV.
// Testing with all checks enabled.
#[fixture]
fn required_checks() -> Vec<ReceiptCheck> {
    vec![
        ReceiptCheck::CheckAllocationId,
        ReceiptCheck::CheckSignature,
        ReceiptCheck::CheckTimestamp,
        ReceiptCheck::CheckUnique,
        ReceiptCheck::CheckValue,
        ReceiptCheck::CheckAndReserveCollateral,
    ]
}

// These are the checks that the Indexer will perform for each received receipt, i.e. before requesting a RAV.
// Testing with all checks enabled.
#[fixture]
fn initial_checks() -> Vec<ReceiptCheck> {
    vec![
        ReceiptCheck::CheckAllocationId,
        ReceiptCheck::CheckSignature,
        ReceiptCheck::CheckTimestamp,
        ReceiptCheck::CheckUnique,
        ReceiptCheck::CheckValue,
        ReceiptCheck::CheckAndReserveCollateral,
    ]
}

#[fixture]
fn indexer_1_adapters(
    collateral_adapter: CollateralAdapterMock,
    receipt_storage_adapter: ReceiptStorageAdapterMock,
    receipt_checks_adapter: ReceiptChecksAdapterMock,
    rav_storage_adapter: RAVStorageAdapterMock,
) -> (
    CollateralAdapterMock,
    ReceiptStorageAdapterMock,
    ReceiptChecksAdapterMock,
    RAVStorageAdapterMock,
) {
    (
        collateral_adapter,
        receipt_storage_adapter,
        receipt_checks_adapter,
        rav_storage_adapter,
    )
}

#[fixture]
fn indexer_2_adapters(
    collateral_adapter: CollateralAdapterMock,
    receipt_storage_adapter: ReceiptStorageAdapterMock,
    receipt_checks_adapter: ReceiptChecksAdapterMock,
    rav_storage_adapter: RAVStorageAdapterMock,
) -> (
    CollateralAdapterMock,
    ReceiptStorageAdapterMock,
    ReceiptChecksAdapterMock,
    RAVStorageAdapterMock,
) {
    (
        collateral_adapter,
        receipt_storage_adapter,
        receipt_checks_adapter,
        rav_storage_adapter,
    )
}

// Helper fixture to generate a batch of receipts to be sent to the Indexer.
// Messages are formatted according to TAP spec and signed according to EIP-712.
#[fixture]
async fn requests_1(
    keys_gateway: (LocalWallet, Address),
    query_price: Vec<u128>,
    num_batches: u64,
    allocation_ids: Vec<H160>,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let (gateway_key, _) = keys_gateway;
    // Create your Receipt here
    let requests =
        generate_requests(query_price, num_batches, &gateway_key, allocation_ids[0]).await?;
    Ok(requests)
}

#[fixture]
async fn requests_2(
    keys_gateway: (LocalWallet, Address),
    query_price: Vec<u128>,
    num_batches: u64,
    allocation_ids: Vec<H160>,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let (gateway_key, _) = keys_gateway;
    // Create your Receipt here
    let requests =
        generate_requests(query_price, num_batches, &gateway_key, allocation_ids[1]).await?;
    Ok(requests)
}

#[fixture]
async fn wrong_requests(
    wrong_keys_gateway: (LocalWallet, Address),
    query_price: Vec<u128>,
    num_batches: u64,
    allocation_ids: Vec<H160>,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let (gateway_key, _) = wrong_keys_gateway;
    // Create your Receipt here
    // Create your Receipt here
    let requests =
        generate_requests(query_price, num_batches, &gateway_key, allocation_ids[0]).await?;
    Ok(requests)
}

// Helper fixtures to start servers for tests
#[fixture]
async fn single_indexer_test_server(
    keys_gateway: (LocalWallet, Address),
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
    indexer_1_adapters: (
        CollateralAdapterMock,
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        RAVStorageAdapterMock,
    ),
    available_collateral: u128,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold_1: u64,
) -> Result<(ServerHandle, SocketAddr, ServerHandle, SocketAddr)> {
    let gateway_id = keys_gateway.1;
    let (gateway_aggregator_handle, gateway_aggregator_addr) = start_gateway_aggregator(
        keys_gateway,
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let (collateral_adapter, receipt_storage_adapter, receipt_checks_adapter, rav_storage_adapter) =
        indexer_1_adapters;
    let (indexer_handle, indexer_addr) = start_indexer_server(
        collateral_adapter,
        receipt_storage_adapter,
        receipt_checks_adapter,
        rav_storage_adapter,
        gateway_id,
        available_collateral,
        initial_checks,
        required_checks,
        receipt_threshold_1,
        gateway_aggregator_addr,
    )
    .await?;
    Ok((
        indexer_handle,
        indexer_addr,
        gateway_aggregator_handle,
        gateway_aggregator_addr,
    ))
}

#[fixture]
async fn two_indexers_test_servers(
    keys_gateway: (LocalWallet, Address),
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
    indexer_1_adapters: (
        CollateralAdapterMock,
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        RAVStorageAdapterMock,
    ),
    indexer_2_adapters: (
        CollateralAdapterMock,
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        RAVStorageAdapterMock,
    ),
    available_collateral: u128,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold_1: u64,
) -> Result<(
    ServerHandle,
    SocketAddr,
    ServerHandle,
    SocketAddr,
    ServerHandle,
    SocketAddr,
)> {
    let gateway_id = keys_gateway.1;
    let (gateway_aggregator_handle, gateway_aggregator_addr) = start_gateway_aggregator(
        keys_gateway,
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let (
        collateral_adapter_1,
        receipt_storage_adapter_1,
        receipt_checks_adapter_1,
        rav_storage_adapter_1,
    ) = indexer_1_adapters;
    let (
        collateral_adapter_2,
        receipt_storage_adapter_2,
        receipt_checks_adapter_2,
        rav_storage_adapter_2,
    ) = indexer_2_adapters;

    let (indexer_handle, indexer_addr) = start_indexer_server(
        collateral_adapter_1,
        receipt_storage_adapter_1,
        receipt_checks_adapter_1,
        rav_storage_adapter_1,
        gateway_id,
        available_collateral,
        initial_checks.clone(),
        required_checks.clone(),
        receipt_threshold_1,
        gateway_aggregator_addr,
    )
    .await?;

    let (indexer_handle_2, indexer_addr_2) = start_indexer_server(
        collateral_adapter_2,
        receipt_storage_adapter_2,
        receipt_checks_adapter_2,
        rav_storage_adapter_2,
        gateway_id,
        available_collateral,
        initial_checks,
        required_checks,
        receipt_threshold_1,
        gateway_aggregator_addr,
    )
    .await?;

    Ok((
        indexer_handle,
        indexer_addr,
        indexer_handle_2,
        indexer_addr_2,
        gateway_aggregator_handle,
        gateway_aggregator_addr,
    ))
}

#[fixture]
async fn single_indexer_wrong_gateway_test_server(
    wrong_keys_gateway: (LocalWallet, Address),
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
    indexer_1_adapters: (
        CollateralAdapterMock,
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        RAVStorageAdapterMock,
    ),
    available_collateral: u128,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold_1: u64,
) -> Result<(ServerHandle, SocketAddr, ServerHandle, SocketAddr)> {
    let gateway_id = wrong_keys_gateway.1;
    let (gateway_aggregator_handle, gateway_aggregator_addr) = start_gateway_aggregator(
        wrong_keys_gateway,
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let (collateral_adapter, receipt_storage_adapter, receipt_checks_adapter, rav_storage_adapter) =
        indexer_1_adapters;

    let (indexer_handle, indexer_addr) = start_indexer_server(
        collateral_adapter,
        receipt_storage_adapter,
        receipt_checks_adapter,
        rav_storage_adapter,
        gateway_id,
        available_collateral,
        initial_checks,
        required_checks,
        receipt_threshold_1,
        gateway_aggregator_addr,
    )
    .await?;

    Ok((
        indexer_handle,
        indexer_addr,
        gateway_aggregator_handle,
        gateway_aggregator_addr,
    ))
}

#[rstest]
#[tokio::test]
async fn test_manager_one_indexer(
    #[future] single_indexer_test_server: Result<
        (ServerHandle, SocketAddr, ServerHandle, SocketAddr),
        Error,
    >,
    #[future] requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_server_handle, socket_addr, _gateway_handle, _gateway_addr) =
        single_indexer_test_server.await?;
    let indexer_1_address = "http://".to_string() + &socket_addr.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let requests = requests_1.await?;

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
    #[future] requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    #[future] requests_2: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
) -> Result<()> {
    let (
        _server_handle_1,
        socket_addr_1,
        _server_handle_2,
        socket_addr_2,
        _gateway_handle,
        _gateway_addr,
    ) = two_indexers_test_servers.await?;

    let indexer_1_address = "http://".to_string() + &socket_addr_1.to_string();
    let indexer_2_address = "http://".to_string() + &socket_addr_2.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let client_2 = HttpClientBuilder::default().build(indexer_2_address)?;
    let requests_1 = requests_1.await?;
    let requests_2 = requests_2.await?;

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
    #[future] single_indexer_wrong_gateway_test_server: Result<
        (ServerHandle, SocketAddr, ServerHandle, SocketAddr),
        Error,
    >,
    #[future] requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    receipt_threshold_1: u64,
) -> Result<()> {
    let (_server_handle, socket_addr, _gateway_handle, _gateway_addr) =
        single_indexer_wrong_gateway_test_server.await?;
    let indexer_1_address = "http://".to_string() + &socket_addr.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let requests = requests_1.await?;

    let mut counter = 1;
    for (receipt_1, id) in requests {
        let result = client_1.request("request", (id, receipt_1)).await;
        // The rav request is being made with messages that have been signed with a key that differs from the gateway aggregator's.
        // So the Gateway Aggregator should send an error to the requesting Indexer.
        // And so the Indexer should then return an error to the clinet when a rav request is made.
        // A rav request is made when the number of receipts sent = receipt_threshold_1.
        // result should be an error when counter = multiple of receipt_threshold_1 and Ok otherwise.
        if (counter % receipt_threshold_1) == 0 {
            match result {
                Ok(()) => panic!("Gateway Aggregator should have sent an error to the Indexer."),
                Err(_) => {}
            }
        } else {
            match result {
                Ok(()) => {}
                Err(e) => panic!("Error making receipt request: {:?}", e),
            }
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
    #[future] wrong_requests: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    receipt_threshold_1: u64,
) -> Result<()> {
    let (_server_handle, socket_addr, _gateway_handle, _gateway_addr) =
        single_indexer_test_server.await?;
    let indexer_1_address = "http://".to_string() + &socket_addr.to_string();
    let client_1 = HttpClientBuilder::default().build(indexer_1_address)?;
    let requests = wrong_requests.await?;

    let mut counter = 1;
    for (receipt_1, id) in requests {
        let result = client_1.request("request", (id, receipt_1)).await;
        // The receipts have been signed with a key that the Indexer is not expecting.
        // So the Indexer should return an error when a rav request is made, because they will not have any valid receipts for the request.
        // A rav request is made when the number of receipts sent = receipt_threshold_1.
        // result should be an error when counter = multiple of receipt_threshold_1 and Ok otherwise.
        if (counter % receipt_threshold_1) == 0 {
            match result {
                Ok(()) => panic!("Should have failed signature verification"),
                Err(_) => {}
            }
        } else {
            match result {
                Ok(()) => {}
                Err(e) => panic!("Error making receipt request: {:?}", e),
            }
        }
        counter += 1;
    }

    Ok(())
}

async fn generate_requests(
    query_price: Vec<u128>,
    num_batches: u64,
    gateway_key: &LocalWallet,
    allocation_id: H160,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    let mut requests: Vec<(EIP712SignedMessage<Receipt>, u64)> = Vec::new();

    let mut counter = 0;
    for _ in 0..num_batches {
        for value in &query_price {
            requests.push((
                EIP712SignedMessage::new(Receipt::new(allocation_id, *value)?, gateway_key).await?,
                counter,
            ));
            counter += 1;
        }
        counter = 0;
    }

    Ok(requests)
}

// Start-up a mock Indexer. Requires a Gateway Aggregator to be running.
async fn start_indexer_server(
    mut collateral_adapter: CollateralAdapterMock,
    receipt_storage_adapter: ReceiptStorageAdapterMock,
    receipt_checks_adapter: ReceiptChecksAdapterMock,
    rav_storage_adapter: RAVStorageAdapterMock,
    gateway_id: Address,
    available_collateral: u128,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold: u64,
    agg_server_addr: SocketAddr,
) -> Result<(ServerHandle, SocketAddr)> {
    let http_port = {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.local_addr()?.port()
    };

    collateral_adapter.increase_collateral(gateway_id, available_collateral);
    let aggregate_server_address = "http://".to_string() + &agg_server_addr.to_string();

    let (server_handle, socket_addr) = indexer_mock::run_server(
        http_port,
        collateral_adapter,
        receipt_checks_adapter,
        receipt_storage_adapter,
        rav_storage_adapter,
        initial_checks,
        required_checks,
        receipt_threshold,
        aggregate_server_address,
        aggregate_server_api_version(),
    )
    .await?;

    Ok((server_handle, socket_addr))
}

// Start-up a Gateway Aggregator.
async fn start_gateway_aggregator(
    keys: (LocalWallet, Address),
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
) -> Result<(ServerHandle, SocketAddr)> {
    let http_port = {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.local_addr()?.port()
    };

    let (server_handle, socket_addr) = agg_server::run_server(
        http_port,
        keys.0,
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;

    Ok((server_handle, socket_addr))
}
