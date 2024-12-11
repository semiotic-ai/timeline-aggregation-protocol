fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running build.rs...");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by Cargo");
    println!("OUT_DIR: {}", out_dir); // This should print the output directory

    tonic_build::compile_protos("./proto/tap_aggregator.proto")?;
    Ok(())
}
