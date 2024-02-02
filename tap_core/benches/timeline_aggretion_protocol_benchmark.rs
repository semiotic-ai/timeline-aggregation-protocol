// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt type used for providing and verifying a payment
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.

use std::str::FromStr;

use alloy_primitives::Address;
use alloy_sol_types::Eip712Domain;
use criterion::async_executor::AsyncStdExecutor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ethers::signers::{LocalWallet, Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;
use rand_core::OsRng;
use tap_core::tap_eip712_domain;
use tap_core::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};
use tokio::runtime::Runtime;

pub async fn create_and_sign_receipt(
    domain_separator: &Eip712Domain,
    allocation_id: Address,
    value: u128,
    wallet: &Wallet<SigningKey>,
) -> EIP712SignedMessage<Receipt> {
    EIP712SignedMessage::new(
        domain_separator,
        Receipt::new(allocation_id, value).unwrap(),
        wallet,
    )
    .await
    .unwrap()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let domain_seperator = tap_eip712_domain(1, Address::from([0x11u8; 20]));

    let async_runtime = Runtime::new().unwrap();

    let wallet = LocalWallet::new(&mut OsRng);
    let address: [u8; 20] = wallet.address().into();
    let address: Address = address.into();

    // Arbitrary values wrapped in black box to avoid compiler optimizing them out
    let allocation_id = Address::from_str("0xabababababababababababababababababababab").unwrap();
    let value = 12345u128;

    c.bench_function("Create Receipt", |b| {
        b.to_async(AsyncStdExecutor).iter(|| {
            create_and_sign_receipt(
                black_box(&domain_seperator),
                black_box(allocation_id),
                black_box(value),
                black_box(&wallet),
            )
        })
    });

    let receipt = async_runtime.block_on(create_and_sign_receipt(
        &domain_seperator,
        allocation_id,
        value,
        &wallet,
    ));

    c.bench_function("Validate Receipt", |b| {
        b.iter(|| {
            black_box(&receipt)
                .verify(black_box(&domain_seperator), black_box(address))
                .unwrap()
        })
    });

    let mut rav_group = c.benchmark_group("Create RAV with varying input sizes");

    for log_number_of_receipts in 10..30 {
        let receipts = (0..2 ^ log_number_of_receipts)
            .map(|_| {
                async_runtime.block_on(create_and_sign_receipt(
                    &domain_seperator,
                    allocation_id,
                    value,
                    &wallet,
                ))
            })
            .collect::<Vec<_>>();

        rav_group.bench_function(
            &format!("Create RAV w/ 2^{} receipt's", log_number_of_receipts),
            |b| {
                b.iter(|| {
                    ReceiptAggregateVoucher::aggregate_receipts(
                        black_box(allocation_id),
                        black_box(&receipts),
                        black_box(None),
                    )
                })
            },
        );

        let signed_rav = async_runtime
            .block_on(EIP712SignedMessage::new(
                &domain_seperator,
                ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None)
                    .unwrap(),
                &wallet,
            ))
            .unwrap();

        rav_group.bench_function(
            &format!("Validate RAV w/ 2^{} receipt's", log_number_of_receipts),
            |b| b.iter(|| black_box(&signed_rav).verify(&domain_seperator, black_box(address))),
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
