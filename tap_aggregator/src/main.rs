// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]

use anyhow::Result;
use clap::Parser;
use ethers_signers::{coins_bip39::English, MnemonicBuilder};
use tokio::signal::unix::{signal, SignalKind};

use log::{debug, info, warn};
use tap_aggregator::metrics;
use tap_aggregator::server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on for JSON-RPC requests.
    #[arg(long, default_value_t = 8080, env = "TAP_PORT")]
    port: u16,

    /// Gateway mnemonic to be used to sign Receipt Aggregate Vouchers.
    #[arg(long, env = "TAP_MNEMONIC")]
    mnemonic: String,

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
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(args.mnemonic.as_str())
        .build()?;

    // Start the JSON-RPC server.
    // This await is non-blocking
    let (handle, _) = server::run_server(
        args.port,
        wallet,
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
