// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;
use ethers_signers::{coins_bip39::English, MnemonicBuilder};
use tokio::signal::unix::{signal, SignalKind};

use tap_aggregator::server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on for JSON-RPC requests.
    #[arg(short, long, default_value_t = 8080, env = "TAP_PORT")]
    port: u16,

    /// Gateway mnemonic to be used to sign Receipt Aggregate Vouchers.
    #[arg(short, long, env = "TAP_MNEMONIC")]
    mnemonic: String,

    /// Maximum request body size in bytes.
    /// Defaults to 10MB.
    #[arg(short, long, default_value_t = 10 * 1024 * 1024, env = "TAP_MAX_REQUEST_BODY_SIZE")]
    max_request_body_size: u32,

    /// Maximum response body size in bytes.
    /// Defaults to 100kB.
    #[arg(short, long, default_value_t = 100 * 1024, env = "TAP_MAX_RESPONSE_BODY_SIZE")]
    max_response_body_size: u32,

    /// Maximum number of concurrent connections.
    /// Defaults to 32.
    #[arg(short, long, default_value_t = 32, env = "TAP_MAX_CONNECTIONS")]
    max_connections: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Create a wallet from the mnemonic.
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(args.mnemonic.as_str())
        .build()?;

    // Start the JSON-RPC server.
    let (handle, _) = server::run_server(
        args.port,
        wallet,
        args.max_request_body_size,
        args.max_response_body_size,
        args.max_connections,
    )
    .await?;

    // Have tokio wait for SIGTERM or SIGINT.
    let mut signal_sigint = signal(SignalKind::interrupt())?;
    let mut signal_sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        _ = signal_sigint.recv() => println!("SIGINT"),
        _ = signal_sigterm.recv() => println!("SIGTERM"),
    }

    // If we're here, we've received a signal to exit.
    println!("Shutting down...");

    // Stop the server and wait for it to finish gracefully.
    handle.stop()?;
    handle.stopped().await;

    Ok(())
}
