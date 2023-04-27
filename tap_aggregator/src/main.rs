use anyhow::Result;
use clap::Parser;
use coins_bip39::English;
use ethers_signers::MnemonicBuilder;
use tokio::signal::unix::{signal, SignalKind};

mod aggregator;
mod server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 8080, env = "TAP_PORT")]
    port: u16,
    #[arg(short, long, env = "TAP_MNEMONIC")]
    mnemonic: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Create a wallet from the mnemonic.
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(args.mnemonic.as_str())
        .build()?;

    // Start the JSON-RPC server.
    let (handle, _) = server::run_server(args.port, wallet.signer().clone()).await?;

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
