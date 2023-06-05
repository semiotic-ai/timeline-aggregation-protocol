use anyhow::Result;
use ethers::signers::coins_bip39::English;
use ethers::signers::{LocalWallet, MnemonicBuilder, Signer};
use ethers::types::{Address, H160};
use futures::Future;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::server::ServerHandle;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rstest::*;
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::iter::FromIterator;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tap_aggregator::server as agg_server;
use tap_core::eip_712_signed_message::EIP712SignedMessage;
use tap_core::tap_receipt::Receipt;
use tap_core::{
    adapters::{
        collateral_adapter_mock::CollateralAdapterMock,
        rav_storage_adapter_mock::RAVStorageAdapterMock,
        receipt_checks_adapter_mock::ReceiptChecksAdapterMock,
        receipt_storage_adapter_mock::ReceiptStorageAdapterMock,
    },
    tap_receipt::ReceiptCheck,
};
use tokio::join;

pub mod server;

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
fn wrong_keys() -> (LocalWallet, Address) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
        .phrase("devote force reopen galaxy humor virtual hobby chief grit nothing bag pulse")
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
    2
}

#[fixture]
fn num_queries() -> usize {
    16
}

#[fixture]
fn collateral_adapter() -> CollateralAdapterMock {
    CollateralAdapterMock::new(Arc::new(RwLock::new(HashMap::new())))
}

#[fixture]
fn receipt_storage_adapter() -> ReceiptStorageAdapterMock {
    ReceiptStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new())))
}

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

#[fixture]
fn receipt_checks_adapter(query_price: Vec<u128>) -> ReceiptChecksAdapterMock {
    // Setup receipt storage
    let receipt_storage = Arc::new(RwLock::new(HashMap::new()));

    let query_prices = query_price;
    // Setup query appraisals
    let query_appraisals = (0..num_queries() as u64)
        .zip(query_prices)
        .into_iter()
        .map(|(id, price)| (id, price))
        .collect::<HashMap<_, _>>();

    let query_appraisals_storage = Arc::new(RwLock::new(query_appraisals));

    // Setup receipt checks adapter
    let allocation_ids: Arc<RwLock<HashSet<H160>>> =
        Arc::new(RwLock::new(HashSet::from_iter(allocation_ids())));
    let gateway_ids: Arc<RwLock<HashSet<H160>>> = Arc::new(RwLock::new(HashSet::from([keys().1])));
    ReceiptChecksAdapterMock::new(
        receipt_storage.clone(),
        query_appraisals_storage.clone(),
        allocation_ids.clone(),
        gateway_ids.clone(),
    )
}

#[fixture]
fn rav_storage_adapter() -> RAVStorageAdapterMock {
    RAVStorageAdapterMock::new(Arc::new(RwLock::new(HashMap::new())))
}

#[fixture]
fn indexer_1_adapters() -> (
    CollateralAdapterMock,
    ReceiptStorageAdapterMock,
    ReceiptChecksAdapterMock,
    RAVStorageAdapterMock,
) {
    (
        collateral_adapter(),
        receipt_storage_adapter(),
        receipt_checks_adapter(query_price()),
        rav_storage_adapter(),
    )
}

#[fixture]
fn indexer_2_adapters() -> (
    CollateralAdapterMock,
    ReceiptStorageAdapterMock,
    ReceiptChecksAdapterMock,
    RAVStorageAdapterMock,
) {
    (
        collateral_adapter(),
        receipt_storage_adapter(),
        receipt_checks_adapter(query_price()),
        rav_storage_adapter(),
    )
}

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
async fn signed_requests_1(
    keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    num_batches: u64,
    allocation_ids: Vec<H160>,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    // Create your Receipt here
    let values = query_price.clone();
    let mut receipts = Vec::new();
    let mut req_ids = Vec::new();
    for _ in 0..num_batches {
        let mut counter = 0u64;
        // Sign receipt
        for value in values.clone() {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value)?, &keys.0)
                    .await
                    .expect("Failed to sign receipt"),
            );
            req_ids.push(counter);
            counter += 1;
        }
    }

    let req = receipts
        .into_iter()
        .zip(req_ids.clone())
        .collect::<Vec<_>>();
    Ok(req)
}

#[fixture]
async fn signed_requests_2(
    keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    num_batches: u64,
    allocation_ids: Vec<H160>,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    // Create your Receipt here
    let values = query_price.clone();
    let mut receipts = Vec::new();
    let mut req_ids = Vec::new();
    for _ in 0..num_batches {
        let mut counter = 0u64;
        // Sign receipt
        for value in values.clone() {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[1], value)?, &keys.0)
                    .await
                    .expect("Failed to sign receipt"),
            );
            req_ids.push(counter);
            counter += 1;
        }
    }

    let req = receipts
        .into_iter()
        .zip(req_ids.clone())
        .collect::<Vec<_>>();
    Ok(req)
}

#[fixture]
async fn wrong_signed_requests(
    wrong_keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    num_batches: u64,
    allocation_ids: Vec<H160>,
) -> Result<Vec<(EIP712SignedMessage<Receipt>, u64)>> {
    // Create your Receipt here
    let values = query_price.clone();
    let mut receipts = Vec::new();
    let mut req_ids = Vec::new();
    for _ in 0..num_batches {
        let mut counter = 0u64;
        // Sign receipt
        for value in values.clone() {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value)?, &wrong_keys.0)
                    .await
                    .expect("Failed to sign receipt"),
            );
            req_ids.push(counter);
            counter += 1;
        }
    }

    let req = receipts
        .into_iter()
        .zip(req_ids.clone())
        .collect::<Vec<_>>();
    Ok(req)
}

#[fixture]
fn receipt_threshold_1() -> u64 {
    800
}

#[fixture]
fn receipt_threshold_2() -> u64 {
    400
}

#[fixture]
fn num_batches() -> u64 {
    100
}

#[rstest]
#[tokio::test]
async fn test_manager_one_indexer(
    indexer_1_adapters: (
        CollateralAdapterMock,
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        RAVStorageAdapterMock,
    ),
    keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold_1: u64,
    #[future] signed_requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let (_agg_server_handle, agg_server_addr) = start_aggregate_server(
        keys.clone(),
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let (_server_handle, socket_addr) = start_indexer_server(
        indexer_1_adapters.0,
        indexer_1_adapters.1,
        indexer_1_adapters.2,
        indexer_1_adapters.3,
        keys,
        query_price,
        initial_checks,
        required_checks,
        receipt_threshold_1,
        agg_server_addr,
    )
    .await?;

    // Setup client
    let client_1 =
        HttpClientBuilder::default().build("http://".to_owned() + &socket_addr.to_string())?;

    let req = signed_requests_1.await?;
    for (receipt_1, id) in req.clone() {
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
    keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold_1: u64,
    receipt_threshold_2: u64,
    #[future] signed_requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    #[future] signed_requests_2: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_agg_server_handle, agg_server_addr) = start_aggregate_server(
        keys.clone(),
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let (_server_handle_1, socket_addr_1) = start_indexer_server(
        indexer_1_adapters.0,
        indexer_1_adapters.1,
        indexer_1_adapters.2,
        indexer_1_adapters.3,
        keys.clone(),
        query_price.clone(),
        initial_checks.clone(),
        required_checks.clone(),
        receipt_threshold_1,
        agg_server_addr,
    )
    .await?;
    let (_server_handle_2, socket_addr_2) = start_indexer_server(
        indexer_2_adapters.0,
        indexer_2_adapters.1,
        indexer_2_adapters.2,
        indexer_2_adapters.3,
        keys,
        query_price,
        initial_checks,
        required_checks,
        receipt_threshold_2,
        agg_server_addr,
    )
    .await?;

    // Setup client
    let client_1 =
        HttpClientBuilder::default().build("http://".to_owned() + &socket_addr_1.to_string())?;

    let client_2 =
        HttpClientBuilder::default().build("http://".to_owned() + &socket_addr_2.to_string())?;

    let req = signed_requests_1
        .await?
        .into_iter()
        .zip(signed_requests_2.await?)
        .collect::<Vec<_>>();

    for ((receipt_1, id_1), (receipt_2, id_2)) in req.clone() {
        let future_1: std::pin::Pin<
            Box<dyn Future<Output = Result<(), jsonrpsee::core::Error>> + Send>,
        > = client_1.request("request", (id_1, receipt_1));
        let future_2: std::pin::Pin<
            Box<dyn Future<Output = Result<(), jsonrpsee::core::Error>> + Send>,
        > = client_2.request("request", (id_2, receipt_2));
        let result = join!(future_1, future_2);
        assert_eq!(result.0.is_ok(), result.1.is_ok());
    }
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_manager_one_indexer_wrong_aggregator_keys(
    indexer_1_adapters: (
        CollateralAdapterMock,
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        RAVStorageAdapterMock,
    ),
    keys: (LocalWallet, Address),
    wrong_keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold_1: u64,
    #[future] signed_requests_1: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_agg_server_handle, agg_server_addr) = start_aggregate_server(
        wrong_keys,
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let (_server_handle, socket_addr) = start_indexer_server(
        indexer_1_adapters.0,
        indexer_1_adapters.1,
        indexer_1_adapters.2,
        indexer_1_adapters.3,
        keys,
        query_price,
        initial_checks,
        required_checks,
        receipt_threshold_1,
        agg_server_addr,
    )
    .await?;

    // Setup client
    let client_1 =
        HttpClientBuilder::default().build("http://".to_owned() + &socket_addr.to_string())?;

    let req = signed_requests_1.await?;
    let mut counter = 1;
    for (receipt_1, id) in req.clone() {
        let result = client_1.request("request", (id, receipt_1)).await;
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

#[rstest]
#[tokio::test]
async fn test_manager_one_indexer_wrong_requestor_keys(
    indexer_1_adapters: (
        CollateralAdapterMock,
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        RAVStorageAdapterMock,
    ),
    keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold_1: u64,
    #[future] wrong_signed_requests: Result<Vec<(EIP712SignedMessage<Receipt>, u64)>>,
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_agg_server_handle, agg_server_addr) = start_aggregate_server(
        keys.clone(),
        http_request_size_limit,
        http_response_size_limit,
        http_max_concurrent_connections,
    )
    .await?;
    let (_server_handle, socket_addr) = start_indexer_server(
        indexer_1_adapters.0,
        indexer_1_adapters.1,
        indexer_1_adapters.2,
        indexer_1_adapters.3,
        keys,
        query_price,
        initial_checks,
        required_checks,
        receipt_threshold_1,
        agg_server_addr,
    )
    .await?;

    // Setup client
    let client_1 =
        HttpClientBuilder::default().build("http://".to_owned() + &socket_addr.to_string())?;

    let req = wrong_signed_requests.await?;
    let mut counter = 1;
    for (receipt_1, id) in req.clone() {
        let result = client_1.request("request", (id, receipt_1)).await;
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

async fn start_indexer_server(
    mut collateral_adapter: CollateralAdapterMock,
    receipt_storage_adapter: ReceiptStorageAdapterMock,
    receipt_checks_adapter: ReceiptChecksAdapterMock,
    rav_storage_adapter: RAVStorageAdapterMock,
    keys: (LocalWallet, Address),
    query_price: Vec<u128>,
    initial_checks: Vec<ReceiptCheck>,
    required_checks: Vec<ReceiptCheck>,
    receipt_threshold: u64,
    agg_server_addr: SocketAddr,
) -> Result<(ServerHandle, SocketAddr)> {
    let http_port = {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.local_addr()?.port()
    };
    let gateway_id = keys.1;
    let value: u128 = query_price.clone().into_iter().sum();
    collateral_adapter.increase_collateral(gateway_id, value);
    let aggregate_server_address = "http://".to_string() + &agg_server_addr.to_string();
    let (server_handle, socket_addr) = server::run_server(
        http_port,
        collateral_adapter,
        receipt_checks_adapter,
        receipt_storage_adapter,
        rav_storage_adapter,
        initial_checks,
        required_checks,
        receipt_threshold,
        aggregate_server_address,
    )
    .await?;
    Ok((server_handle, socket_addr))
}

async fn start_aggregate_server(
    keys: (LocalWallet, Address),
    http_request_size_limit: u32,
    http_response_size_limit: u32,
    http_max_concurrent_connections: u32,
) -> Result<(ServerHandle, SocketAddr)> {
    let http_port = {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.local_addr()?.port()
    };
    // Start tap_aggregate server
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
