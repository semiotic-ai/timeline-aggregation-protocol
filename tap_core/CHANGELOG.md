# Changelog

## [0.5.1](https://github.com/semiotic-ai/timeline-aggregation-protocol/compare/tap_core-v0.5.0...tap_core-v0.5.1) (2023-08-24)


### Bug Fixes

* Update alloy to v0.3.2 ([715a04c](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/715a04c1d9b9622e0bb846830f8dd782062901c7))

## [0.5.0](https://github.com/semiotic-ai/timeline-aggregation-protocol/compare/tap_core-v0.4.0...tap_core-v0.5.0) (2023-08-17)


### ⚠ BREAKING CHANGES

* **tap-core:** receipts, RAVs, and eip712-signed-message now use alloy primitive address

### Features

* **tap-core-bench:** update to work with alloy changes ([43c5e6c](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/43c5e6c24218b9c5138cabec50262715f6b34124))
* **tap-core:** changes eip712 and address components to alloy library ([a99af2f](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/a99af2f8d64ad2d1a4d175de5e7789254fb2c36f))

## [0.4.0](https://github.com/semiotic-ai/timeline-aggregation-protocol/compare/tap_core-v0.3.0...tap_core-v0.4.0) (2023-08-08)


### ⚠ BREAKING CHANGES

* **core:** All ReceiptChecksAdapter trait method now return Result. Makes it possible to return Adapter errors besides just check failures.
* All instances of the word "collateral" are replaced with "escrow".

### Code Refactoring

* **core:** make ChecksAdapter return Result ([0983c27](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/0983c27c40d2061a51687fa0517add2593d3b934))
* Replace "collateral" with "escrow" ([6f9d0c7](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/6f9d0c7a54fa445f1e058a65bcfccdef06cf9d02))

## [0.3.0](https://github.com/semiotic-ai/timeline-aggregation-protocol/compare/tap_core-v0.2.0...tap_core-v0.3.0) (2023-07-31)


### ⚠ BREAKING CHANGES

* **core:** all the traits are now async

### Features

* serde for ReceivedReceipt ([1765b5d](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/1765b5d101e73ff0085e6246252b47e4d5e98890))


### Code Refactoring

* **core:** make it all async ([30ca4ba](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/30ca4ba7da05959d380b653de0307af273c4733d))

## [0.2.0](https://github.com/semiotic-ai/timeline-aggregation-protocol/compare/tap_core-v0.1.0...tap_core-v0.2.0) (2023-07-20)


### ⚠ BREAKING CHANGES

* **tap_core:** Preserve adapter error type using anyhow ([#133](https://github.com/semiotic-ai/timeline-aggregation-protocol/issues/133))
* **receipt_storage_adapter:** all functions unnecessary to the TAP manager have been removed from the `ReceiptStorageAdapter` trait.
* **rav_storage_adapter:** the RavStorageAdapter public trait has changed

### Features

* **manager:** add receipts_auto_delete ([#130](https://github.com/semiotic-ai/timeline-aggregation-protocol/issues/130)) ([37bc8e1](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/37bc8e1212fb525e15be465bdc0dba4e5349c025))
* **receipt_storage_adapter:** Use RangeBounds ([663a8ba](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/663a8ba7f0afb74cc17e4eb58802282771cb0bca))
* **tap_core:** Preserve adapter error type using anyhow ([#133](https://github.com/semiotic-ai/timeline-aggregation-protocol/issues/133)) ([77abbd8](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/77abbd82d5e0c75cd04b1bce55360a312f036bac))


### Bug Fixes

* **tap_manager:** receipt auditor min_timestamp set incorrectly ([743ac7c](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/743ac7cd252a3723ad01fbd4a1b54cbaf68267af))


### Code Refactoring

* **rav_storage_adapter:** simplify trait ([6af9471](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/6af9471bea8aeafd415b4f3e7b92a1eb6bec135b))
* **receipt_storage_adapter:** prune trait ([caef197](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/caef197261f16187402f69091ffea843e6aef970))

## 0.1.0 (2023-06-29)


### ⚠ BREAKING CHANGES

* **tap-manager:** RAV Request definition is updated to include optional previous RAV
* **receipts:** Updates the receipt checks adapter api
* **signed-message:** Updates signed message API
* **allocation-adapter:** removes allocation adapter
* **adapters:** existing adapter trait definitions are updated

### Features

* **adapter-mocks:** alters adapters to use references (arc) to allow sharing common resources ([03e7668](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/03e7668e3c59d27e6cfc869a3a35ad1434d18d6d))
* **adapters:** adds functionality to retrieve and delete receipts within a range ([4143ac6](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/4143ac6293751a0e837709bba43d4fc600911bcc))
* **adapters:** split adapters into storage and check adapters ([39c1c82](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/39c1c827447aceb938ad03643bb7bf08ff330cae))
* **core:** add `verify` to EIP712 signed msg ([a7e3e7d](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/a7e3e7d18044dbe6937cf725376167171fb177b1))
* **receipt-storage:** Adds functionality to update receipt in storage by id ([eb4f8ba](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/eb4f8bae233406b6c5d25def4de1d628d7860b1e))
* **receipts:** Updates checking mechanisms and adds auditor to complete checks ([8d0088d](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/8d0088d6fbc83416737cf33c3e305412741c8ec8))
* **signed-message:** Updates library to use ether-rs for wallet, address, key, signature, and verification ([7f1cb85](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/7f1cb8586e7577221008588cacd5cc6ad47d1c83))
* **tap-manager:** Adds a tap manager for handling receipt and rav validation and storage ([3786042](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/378604263a91d3abd9fdad1dd978f1ba715f7aca))


### Bug Fixes

* **allocation-adapter:** remove obsolete trait ([957f3f9](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/957f3f99efa8b5ebe7f61024f96905b0448c4bed))
* **receipt-errors:** updates receipt errors to work with adapter trait api ([fc121bf](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/fc121bf21bee2b8bdf0d1db026e97afb2844d75b))
* **receipts:** adds receipt ID to check unique to allow checking after receipt is in storage ([5072fb9](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/5072fb9ba614d58bcc712778deb451eaefbc993f))
* **tap-manager:** adds an error when receipt timestamp window is inverted ([e21d7d9](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/e21d7d941b8b298bdda5dc8143b3163f65ca1e85))
* **tap-manager:** receipts being used after being added to RAV ([efd88a2](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/efd88a214a3737b7bb201cabaf4037284ec5d547))
* verification benchmarks ([#114](https://github.com/semiotic-ai/timeline-aggregation-protocol/issues/114)) ([96cdf24](https://github.com/semiotic-ai/timeline-aggregation-protocol/commit/96cdf24db98a715cec654ff77de1837ba36f81a4))
