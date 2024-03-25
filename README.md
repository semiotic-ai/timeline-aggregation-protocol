# Timeline Aggregation Protocol (TAP)

## Overview

The TAP (Timeline Aggregation Protocol) facilitates a series of payments from a 
sender to a receiver (TAP Receipts), who aggregates these payments into a single 
payment (a Receipt Aggregate Voucher, or RAV). This aggregate payment can then be 
verified on-chain by a payment verifier, reducing the number of transactions and 
simplifying the payment process.

## Key Components

- **Sender:** Initiates the payment.
- **Receiver:** Receives the payment.
- **Signers:** Multiple signers authorized by the sender to sign receipts.
- **State Channel:** A one-way channel opened by the sender with the receiver 
for sending receipts.
- **Receipt:** A record of payment sent by the sender to the receiver.
- **ReceiptAggregateVoucher (RAV):** A signed message containing the aggregate 
value of the receipts.
- **tap_aggregator:** A service managed by the sender that aggregates receipts 
on the receiver's request into a signed RAV.
- **EscrowAccount:** An account created in the blockchain to hold funds for 
the sender-receiver pair.

## Security Measures

- The protocol uses asymmetric cryptography (ECDSA secp256k1) to sign and 
verify messages, ensuring the integrity of receipts and RAVs.

## Process

1. **Opening a State Channel:** A state channel is opened via a blockchain 
contract, creating an EscrowAccount for the sender-receiver pair.
2. **Sending Receipts:** The sender sends receipts to the receiver through the 
state channel.
3. **Storing Receipts:** The receiver stores the receipts and tracks the 
aggregate payment.
4. **Creating a RAV Request:** A RAV request consists of a list of receipts and, 
optionally, the previous RAV.
5. **Signing the RAV:** The receiver sends the RAV request to the tap_aggregator, 
which signs it into a new RAV.
6. **Tracking Aggregate Value:** The receiver tracks the aggregate value and 
new receipts since the last RAV.
7. **Requesting a New RAV:** The receiver sends new receipts and the last RAV 
to the tap_aggregator for a new RAV.
8. **Closing the State Channel:** When the allocation period ends, the receiver 
can send the last RAV to the blockchain and receive payment from the EscrowAccount.

## Performance Considerations

- The primary performance limitations are the time required to verify receipts 
and network limitations for sending requests to the tap_aggregator.

## Use Cases

- The TAP protocol is suitable for systems with sequential operations that are 
too expensive to redeem individually on-chain. By aggregating operations 
off-chain and redeeming them in one transaction, costs are drastically reduced.

## Compatibility

- The current implementation is for EVM-compatible blockchains, with most of the 
system being off-chain.

## Contributing

Contributions are welcome! Please submit a pull request or open an issue to 
discuss potential changes.
Also, make sure to follow the [Contributing Guide](CONTRIBUTING.md).