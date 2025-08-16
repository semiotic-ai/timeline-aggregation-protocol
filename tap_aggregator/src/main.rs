// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]

use std::{collections::HashSet, str::FromStr};

use anyhow::Result;
use clap::Parser;
use log::{debug, info};
use tap_aggregator::{metrics, server};
use thegraph_core::alloy::{primitives::Address, signers::local::PrivateKeySigner};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on for JSON-RPC requests.
    /// Defaults to 8080.
    #[arg(long, default_value_t = 8080, env = "TAP_PORT")]
    port: u16,

    /// Signer private key for signing Receipt Aggregate Vouchers, as a hex string.
    #[arg(long, env = "TAP_PRIVATE_KEY")]
    private_key: String,

    /// Signer public keys. Not the counterpart of the signer private key. Signers that are allowed
    /// for the incoming receipts / RAV to aggregate. Useful when needing to accept receipts that
    /// were signed with a different key (e.g. a recent key rotation, or receipts coming from a
    /// different gateway / aggregator that use a different signing key).
    /// Expects a comma-separated list of Ethereum addresses.
    #[arg(long, env = "TAP_PUBLIC_KEYS")]
    public_keys: Option<Vec<Address>>,

    /// Maximum request body size in bytes.
    /// Defaults to 10MB.
    #[arg(long, default_value_t = 10 * 1024 * 1024, env = "TAP_MAX_REQUEST_BODY_SIZE")]
    max_request_body_size: u32,

    /// Maximum response body size in bytes.
    /// Defaults to 100kB.
    #[arg(long, default_value_t = 100 * 1024, env = "TAP_MAX_RESPONSE_BODY_SIZE")]
    max_response_body_size: u32,

    /// Maximum number of concurrent connections.
    /// Defaults to 32.
    #[arg(long, default_value_t = 32, env = "TAP_MAX_CONNECTIONS")]
    max_connections: u32,

    /// Metrics server port.
    /// Defaults to 5000.
    #[arg(long, default_value_t = 5000, env = "TAP_METRICS_PORT")]
    metrics_port: u16,

    /// Domain name to be used for the EIP-712 domain separator.
    #[arg(long, env = "TAP_DOMAIN_NAME")]
    domain_name: Option<String>,

    /// Domain version to be used for the EIP-712 domain separator.
    #[arg(long, env = "TAP_DOMAIN_VERSION")]
    domain_version: Option<String>,

    /// Domain chain ID to be used for the EIP-712 domain separator.
    #[arg(long, env = "TAP_DOMAIN_CHAIN_ID")]
    domain_chain_id: Option<String>,

    /// Domain verifying contract for V1 receipts (TAPVerifier).
    /// Default: 0xC9a43158891282A2B1475592D5719c001986Aaec
    #[arg(long, env = "TAP_DOMAIN_VERIFYING_CONTRACT_V1")]
    domain_verifying_contract_v1: Option<Address>,

    /// Domain verifying contract for V2 receipts (GraphTallyCollector).
    /// Default: 0xB0D4afd8879eD9F52b28595d31B441D079B2Ca07
    #[arg(long, env = "TAP_DOMAIN_VERIFYING_CONTRACT_V2")]
    domain_verifying_contract_v2: Option<Address>,

    /// Domain salt to be used for the EIP-712 domain separator.
    #[arg(long, env = "TAP_DOMAIN_SALT")]
    domain_salt: Option<String>,

    #[arg(long, env = "TAP_KAFKA_CONFIG")]
    kafka_config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger.
    // Set the log level by setting the RUST_LOG environment variable.
    // We prefer using tracing_subscriber as the logging backend because jsonrpsee
    // uses it, and it shows jsonrpsee log spans in the logs (to see client IP, etc).
    // See https://github.com/paritytech/jsonrpsee/pull/922 for more info.
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    debug!("Settings: {args:?}");

    // Start the metrics server.
    // We just let it gracelessly get killed at the end of main()
    tokio::spawn(metrics::run_server(args.metrics_port));

    // Create a wallet from the mnemonic.
    let wallet = PrivateKeySigner::from_str(&args.private_key)?;

    info!("Wallet address: {:#40x}", wallet.address());

    // Create the domain configuration
    let domain_config = create_domain_config(&args)?;

    // Create HashSet of *all* allowed signers
    let mut accepted_addresses: HashSet<Address> = std::collections::HashSet::new();
    accepted_addresses.insert(wallet.address().0.into());
    if let Some(public_keys) = &args.public_keys {
        accepted_addresses.extend(public_keys.iter().cloned());
    }

    let kafka = match args.kafka_config {
        None => None,
        Some(config) => {
            let mut client = rdkafka::ClientConfig::new();
            for (key, value) in config.split(';').filter_map(|s| s.split_once('=')) {
                client.set(key, value);
            }
            Some(client.create()?)
        }
    };

    // Start the JSON-RPC server.
    // This await is non-blocking
    let (handle, _) = server::run_server(
        args.port,
        wallet,
        accepted_addresses,
        domain_config,
        args.max_request_body_size,
        args.max_response_body_size,
        args.max_connections,
        kafka,
    )
    .await?;
    info!("Server started. Listening on port {}.", args.port);

    let _ = handle.await;

    // If we're here, we've received a signal to exit.
    info!("Shutting down...");
    Ok(())
}

fn create_domain_config(args: &Args) -> Result<server::DomainConfig> {
    let chain_id: Option<u64> = args
        .domain_chain_id
        .as_ref()
        .map(|s| s.parse())
        .transpose()?;

    let chain_id = chain_id.unwrap_or(1);

    // Use custom addresses if provided, otherwise use defaults
    if args.domain_verifying_contract_v1.is_some() || args.domain_verifying_contract_v2.is_some() {
        let v1_contract = args.domain_verifying_contract_v1.unwrap_or_else(|| {
            Address::from_str("0xC9a43158891282A2B1475592D5719c001986Aaec").unwrap()
        });
        let v2_contract = args.domain_verifying_contract_v2.unwrap_or_else(|| {
            Address::from_str("0xB0D4afd8879eD9F52b28595d31B441D079B2Ca07").unwrap()
        });

        Ok(server::DomainConfig::custom(
            chain_id,
            v1_contract,
            v2_contract,
        ))
    } else {
        server::DomainConfig::new(chain_id)
            .map_err(|e| anyhow::anyhow!("Failed to create domain config: {}", e))
    }
}
