// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use alloy_sol_types::Eip712Domain;
use anyhow::{Error, Result};
use jsonrpsee::{
    core::{async_trait, client::ClientT},
    http_client::{HttpClient, HttpClientBuilder},
    proc_macros::rpc,
    rpc_params,
    server::{ServerBuilder, ServerHandle},
};

use tap_aggregator::jsonrpsee_helpers;
use tap_core::{
    adapters::{
        escrow_adapter::EscrowAdapter,
        rav_storage_adapter::{RAVRead, RAVStore},
        receipt_checks_adapter::ReceiptChecksAdapter,
        receipt_storage_adapter::{ReceiptRead, ReceiptStore},
    },
    tap_manager::{Manager, SignedRAV, SignedReceipt},
    tap_receipt::ReceiptCheck,
    Error as TapCoreError,
};
/// Rpc trait represents a JSON-RPC server that has a single async method `request`.
/// This method is designed to handle incoming JSON-RPC requests.
#[rpc(server)]
pub trait Rpc {
    // This async method is designed to handle incoming JSON-RPC requests.
    #[method(name = "request")]
    async fn request(
        &self,
        request_id: u64,        // Unique identifier for the request
        receipt: SignedReceipt, // Signed receipt associated with the request
    ) -> Result<(), jsonrpsee::types::ErrorObjectOwned>; // The result of the request, a JSON-RPC error if it fails
}

/// RpcManager is a struct that implements the `Rpc` trait and it represents a JSON-RPC server manager.
/// It includes a manager, initial_checks, receipt_count, threshold and aggregator_client.
/// Manager holds an Arc to an instance of a generic `Manager` object which is shared and can be accessed by multiple threads.
/// initial_checks is a list of checks that needs to be performed for every incoming request.
/// receipt_count is a thread-safe counter that increments with each receipt verified and stored.
/// threshold is a limit to which receipt_count can increment, after reaching which RAV request is triggered.
/// aggregator_client is an HTTP client used for making JSON-RPC requests to another server.
pub struct RpcManager<
    EA: EscrowAdapter + Send + Sync + 'static, // An instance of EscrowAdapter, marked as thread-safe with Send and given 'static lifetime
    RCA: ReceiptChecksAdapter + Send + Sync + 'static, // An instance of ReceiptChecksAdapter
    RSA: ReceiptStore + ReceiptRead + Send + Sync + 'static, // An instance of ReceiptStorageAdapter
    RAVSA: RAVStore + RAVRead + Send + Sync + 'static,
> {
    manager: Arc<Manager<EA, RCA, RSA, RAVSA>>, // Manager object reference counted with an Arc
    initial_checks: Vec<ReceiptCheck>, // Vector of initial checks to be performed on each request
    receipt_count: Arc<AtomicU64>,     // Thread-safe atomic counter for receipts
    threshold: u64,                    // The count at which a RAV request will be triggered
    aggregator_client: (HttpClient, String), // HTTP client for sending requests to the aggregator server
}

/// Implementation for `RpcManager`, includes the constructor and the `request` method.
/// Constructor initializes a new instance of `RpcManager`.
/// `request` method handles incoming JSON-RPC requests and it verifies and stores the receipt from the request.
impl<
        EA: EscrowAdapter + Send + Sync + 'static,
        RCA: ReceiptChecksAdapter + Send + Sync + 'static,
        RSA: ReceiptStore + ReceiptRead + Send + Sync + 'static,
        RAVSA: RAVStore + RAVRead + Send + Sync + 'static,
    > RpcManager<EA, RCA, RSA, RAVSA>
{
    pub fn new(
        domain_separator: Eip712Domain,
        escrow_adapter: EA,
        receipt_checks_adapter: RCA,
        receipt_storage_adapter: RSA,
        rav_storage_adapter: RAVSA,
        initial_checks: Vec<ReceiptCheck>,
        required_checks: Vec<ReceiptCheck>,
        threshold: u64,
        aggregate_server_address: String,
        aggregate_server_api_version: String,
    ) -> Result<Self> {
        Ok(Self {
            manager: Arc::new(Manager::<EA, RCA, RSA, RAVSA>::new(
                domain_separator,
                escrow_adapter,
                receipt_checks_adapter,
                rav_storage_adapter,
                receipt_storage_adapter,
                required_checks,
                get_current_timestamp_u64_ns()?,
            )),
            initial_checks,
            receipt_count: Arc::new(AtomicU64::new(0)),
            threshold,
            aggregator_client: (
                HttpClientBuilder::default().build(aggregate_server_address)?,
                aggregate_server_api_version,
            ),
        })
    }
}

#[async_trait]
impl<
        CA: EscrowAdapter + Send + Sync + 'static,
        RCA: ReceiptChecksAdapter + Send + Sync + 'static,
        RSA: ReceiptStore + ReceiptRead + Send + Sync + 'static,
        RAVSA: RAVStore + RAVRead + Send + Sync + 'static,
    > RpcServer for RpcManager<CA, RCA, RSA, RAVSA>
{
    async fn request(
        &self,
        request_id: u64,
        receipt: SignedReceipt,
    ) -> Result<(), jsonrpsee::types::ErrorObjectOwned> {
        let verify_result = match self
            .manager
            .verify_and_store_receipt(receipt, request_id, self.initial_checks.as_slice())
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(to_rpc_error(
                Box::new(e),
                "Failed to verify and store receipt",
            )),
        };

        // Increment the receipt count
        self.receipt_count.fetch_add(1, Ordering::Relaxed);
        let rav_request_valid = if self.receipt_count.load(Ordering::SeqCst) >= self.threshold {
            // Reset the counter after reaching the threshold
            self.receipt_count.store(0, Ordering::SeqCst);

            // Create the aggregate_receipts request params
            let time_stamp_buffer = 0;
            match request_rav(
                &self.manager,
                time_stamp_buffer,
                &self.aggregator_client,
                self.threshold as usize,
            )
            .await
            {
                Ok(_) => Ok(()),
                Err(e) => Err(to_rpc_error(e.into(), "Failed to request rav")),
            }
        } else {
            Ok(())
        };

        // Combine the results
        match (verify_result, rav_request_valid) {
            (Ok(_), Ok(_)) => Ok(()),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }
}

/// run_server function initializes and starts a JSON-RPC server that handles incoming requests.
pub async fn run_server<
    CA: EscrowAdapter + Send + Sync + 'static,
    RCA: ReceiptChecksAdapter + Send + Sync + 'static,
    RSA: ReceiptStore + ReceiptRead + Send + Sync + 'static,
    RAVSA: RAVStore + RAVRead + Send + Sync + 'static,
>(
    port: u16,                            // Port on which the server will listen
    domain_separator: Eip712Domain,       // EIP712 domain separator
    escrow_adapter: CA,                   // EscrowAdapter instance
    receipt_checks_adapter: RCA,          // ReceiptChecksAdapter instance
    receipt_storage_adapter: RSA,         // ReceiptStorageAdapter instance
    rav_storage_adapter: RAVSA,           // RAVStorageAdapter instance
    initial_checks: Vec<ReceiptCheck>, // Vector of initial checks to be performed on each request
    required_checks: Vec<ReceiptCheck>, // Vector of required checks to be performed on each request
    threshold: u64,                    // The count at which a RAV request will be triggered
    aggregate_server_address: String,  // Address of the aggregator server
    aggregate_server_api_version: String, // API version of the aggregator server
) -> Result<(ServerHandle, std::net::SocketAddr)> {
    // Setting up the JSON RPC server
    println!("Starting server...");
    let server = ServerBuilder::new()
        .http_only()
        .build(format!("127.0.0.1:{}", port))
        .await?;
    let addr = server.local_addr()?;
    println!("Listening on: {}", addr);
    let rpc_manager = RpcManager::new(
        domain_separator,
        escrow_adapter,
        receipt_checks_adapter,
        receipt_storage_adapter,
        rav_storage_adapter,
        initial_checks,
        required_checks,
        threshold,
        aggregate_server_address,
        aggregate_server_api_version,
    )?;

    let handle = server.start(rpc_manager.into_rpc())?;
    Ok((handle, addr))
}

// request_rav function creates a request for aggregate receipts (RAV), sends it to another server and verifies the result.
async fn request_rav<
    CA: EscrowAdapter + Send + Sync + 'static,
    RCA: ReceiptChecksAdapter + Send + Sync + 'static,
    RSA: ReceiptStore + ReceiptRead + Send + Sync + 'static,
    RAVSA: RAVStore + RAVRead + Send + Sync + 'static,
>(
    manager: &Arc<Manager<CA, RCA, RSA, RAVSA>>,
    time_stamp_buffer: u64, // Buffer for timestamping, see tap_core for details
    aggregator_client: &(HttpClient, String), // HttpClient for making requests to the tap_aggregator server
    threshold: usize,
) -> Result<()> {
    // Create the aggregate_receipts request params
    let rav_request = manager.create_rav_request(time_stamp_buffer, None).await?;

    // To-do: Need to add previous RAV, when tap_manager supports replacing receipts
    let params = rpc_params!(
        &aggregator_client.1,
        &rav_request.valid_receipts,
        rav_request.previous_rav
    );

    // Call the aggregate_receipts method on the other server
    let remote_rav_result: jsonrpsee_helpers::JsonRpcResponse<SignedRAV> = aggregator_client
        .0
        .request("aggregate_receipts", params)
        .await?;
    manager
        .verify_and_store_rav(rav_request.expected_rav, remote_rav_result.data)
        .await?;

    // For these tests, we expect every receipt to be valid, i.e. there should be no invalid receipts, nor any missing receipts (less than the expected threshold).
    // If there is throw an error.
    match rav_request.invalid_receipts.is_empty() && (rav_request.valid_receipts.len() == threshold)
    {
        true => Ok(()),
        false => Err(Error::msg("Invalid receipts found")),
    }?;
    Ok(())
}

// get_current_timestamp_u64_ns function returns current system time since UNIX_EPOCH as a 64-bit unsigned integer.
fn get_current_timestamp_u64_ns() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| TapCoreError::InvalidSystemTime {
            source_error_message: err.to_string(),
        })?
        .as_nanos() as u64)
}

fn to_rpc_error(e: Box<dyn std::error::Error>, msg: &str) -> jsonrpsee::types::ErrorObjectOwned {
    jsonrpsee::types::ErrorObject::owned(-32000, format!("{} - {}", e, msg), None::<()>)
}
