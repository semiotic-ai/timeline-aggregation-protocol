fmt:
    cargo +nightly fmt

clippy:
    cargo +nightly clippy --all-targets --all-features

build:
    cargo build

test:
    cargo nextest run
