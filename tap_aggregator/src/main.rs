// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]

use std::borrow::Cow;
use std::collections::HashSet;
use std::str::FromStr;

use alloy::dyn_abi::Eip712Domain;
use alloy::primitives::Address;
use alloy::primitives::FixedBytes;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use clap::Parser;
use ruint::aliases::U256;
use tokio::signal::unix::{signal, SignalKind};

use log::{debug, info};
use tap_aggregator::metrics;
use tap_aggregator::server;

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

    /// Domain verifying contract to be used for the EIP-712 domain separator.
    #[arg(long, env = "TAP_DOMAIN_VERIFYING_CONTRACT")]
    domain_verifying_contract: Option<Address>,

    /// Domain salt to be used for the EIP-712 domain separator.
    #[arg(long, env = "TAP_DOMAIN_SALT")]
    domain_salt: Option<String>,
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
    debug!("Settings: {:?}", args);

    // Start the metrics server.
    // We just let it gracelessly get killed at the end of main()
    tokio::spawn(metrics::run_server(args.metrics_port));

    // Create a wallet from the mnemonic.
    let wallet = PrivateKeySigner::from_str(&args.private_key)?;

    info!("Wallet address: {:#40x}", wallet.address());

    // Create the EIP-712 domain separator.
    let domain_separator = create_eip712_domain(&args)?;

    // Create HashSet of *all* allowed signers
    let mut accepted_addresses: HashSet<Address> = std::collections::HashSet::new();
    accepted_addresses.insert(wallet.address().0.into());
    if let Some(public_keys) = &args.public_keys {
        accepted_addresses.extend(public_keys.iter().cloned());
    }

    // Start the JSON-RPC server.
    // This await is non-blocking
    let (handle, _) = server::run_server(
        args.port,
        wallet,
        accepted_addresses,
        domain_separator,
        args.max_request_body_size,
        args.max_response_body_size,
        args.max_connections,
    )
    .await?;
    info!("Server started. Listening on port {}.", args.port);

    // Have tokio wait for SIGTERM or SIGINT.
    let mut signal_sigint = signal(SignalKind::interrupt())?;
    let mut signal_sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        _ = signal_sigint.recv() => debug!("Received SIGINT."),
        _ = signal_sigterm.recv() => debug!("Received SIGTERM."),
    }

    // If we're here, we've received a signal to exit.
    info!("Shutting down...");

    // Stop the server and wait for it to finish gracefully.
    handle.stop()?;
    handle.stopped().await;

    debug!("Goodbye!");
    Ok(())
}

fn create_eip712_domain(args: &Args) -> Result<Eip712Domain> {
    // Transfrom the args into the types expected by Eip712Domain::new().

    // Transform optional strings into optional Cow<str>.
    let name = args.domain_name.clone().map(Cow::Owned);
    let version = args.domain_version.clone().map(Cow::Owned);

    // Transform optional strings into optional U256.
    if args.domain_chain_id.is_some() {
        debug!("Parsing domain chain ID...");
    }
    let chain_id: Option<U256> = args
        .domain_chain_id
        .as_ref()
        .map(|s| s.parse())
        .transpose()?;

    if args.domain_salt.is_some() {
        debug!("Parsing domain salt...");
    }
    let salt: Option<FixedBytes<32>> = args.domain_salt.as_ref().map(|s| s.parse()).transpose()?;

    // Transform optional strings into optional Address.
    let verifying_contract: Option<Address> = args.domain_verifying_contract;

    // Create the EIP-712 domain separator.
    Ok(Eip712Domain::new(
        name,
        version,
        chain_id,
        verifying_contract,
        salt,
    ))
}
