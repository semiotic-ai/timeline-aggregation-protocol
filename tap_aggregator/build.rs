// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running build.rs...");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by Cargo");
    println!("OUT_DIR: {}", out_dir); // This should print the output directory

    tonic_build::configure().compile_protos(
        &[
            "proto/uint128.proto",
            "proto/uint256.proto",
            "proto/tap_aggregator.proto",
            "proto/tap_aggregator_u256.proto",
            "proto/v2.proto",
            "proto/v2_u256.proto",
        ],
        &["proto"],
    )?;

    Ok(())
}
