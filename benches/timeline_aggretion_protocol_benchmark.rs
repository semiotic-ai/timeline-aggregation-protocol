// Copyright 2023-, Semiotic AI, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module containing Receipt type used for providing and verifying a payment
//!
//! Receipts are used as single transaction promise of payment. A payment sender
//! creates a receipt and ECDSA signs it, then sends it to a payment receiver.
//! The payment receiver would verify the received receipt and store it to be
//! accumulated with other received receipts in the future.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ethereum_types::Address;
use k256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use std::str::FromStr;
use timeline_aggregation_protocol::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};

pub fn create_and_sign_receipt(
    allocation_id: Address,
    timestamp_ns: u64,
    value: u128,
    signing_key: &SigningKey,
) -> EIP712SignedMessage<Receipt> {
    EIP712SignedMessage::new(
        Receipt::new(allocation_id, timestamp_ns, value),
        signing_key,
    )
    .unwrap()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Arbitrary values wrapped in black box to avoid compiler optimizing them out
    let allocation_id =
        black_box(Address::from_str("0xabababababababababababababababababababab").unwrap());
    let value = black_box(12345u128);
    let timestamp_ns = black_box(12345u64);

    c.bench_function("Create Receipt", |b| {
        b.iter(|| create_and_sign_receipt(allocation_id, timestamp_ns, value, &signing_key))
    });

    let receipt = black_box(create_and_sign_receipt(
        allocation_id,
        timestamp_ns,
        value,
        &signing_key,
    ));

    c.bench_function("Validate Receipt", |b| {
        b.iter(|| receipt.check_signature(verifying_key))
    });

    let mut rav_group = c.benchmark_group("Create RAV with varying input sizes");

    for log_number_of_receipts in 10..30 {
        let receipts = black_box(
            (0..2 ^ log_number_of_receipts)
                .map(|_| {
                    EIP712SignedMessage::new(
                        Receipt::new(allocation_id, timestamp_ns, value),
                        &signing_key,
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>(),
        );

        rav_group.bench_function(
            &format!("Create RAV w/ 2^{} receipt's", log_number_of_receipts),
            |b| {
                b.iter(|| {
                    ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None)
                })
            },
        );
        let signed_rav = black_box(EIP712SignedMessage::new(
            ReceiptAggregateVoucher::aggregate_receipts(allocation_id, &receipts, None).unwrap(),
            &signing_key,
        ))
        .unwrap();
        rav_group.bench_function(
            &format!("Validate RAV w/ 2^{} receipt's", log_number_of_receipts),
            |b| b.iter(|| signed_rav.check_signature(verifying_key)),
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
