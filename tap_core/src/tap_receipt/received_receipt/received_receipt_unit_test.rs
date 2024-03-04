use std::{collections::HashMap, str::FromStr, sync::Arc};

use alloy_primitives::Address;
use alloy_sol_types::Eip712Domain;
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
use rstest::*;
use tokio::sync::RwLock;

use crate::{
    adapters::{
        auditor_executor_mock::AuditorExecutorMock,
        executor_mock::{EscrowStorage, QueryAppraisals},
    },
    checks::{tests::get_full_list_of_checks, ReceiptCheck},
    eip_712_signed_message::EIP712SignedMessage,
    get_current_timestamp_u64_ns, tap_eip712_domain,
    tap_receipt::{Receipt, ReceiptAuditor, ReceiptCheckResults, ReceivedReceipt},
};

use super::{Checking, ReceiptWithState};

impl ReceiptWithState<Checking> {
    fn check_is_complete(&self, check: &str) -> bool {
        self.state.checks.get(check).unwrap().is_complete()
    }

    fn checking_is_complete(&self) -> bool {
        self.state
            .checks
            .iter()
            .all(|(_, status)| status.is_complete())
    }
    /// Returns all checks that completed with errors
    fn completed_checks_with_errors(&self) -> ReceiptCheckResults {
        self.state
            .checks
            .iter()
            .filter_map(|(check, result)| {
                if result.is_failed() {
                    return Some((*check, result.clone()));
                }
                None
            })
            .collect()
    }
}

#[fixture]
fn keys() -> (LocalWallet, Address) {
    let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
    // Alloy library does not have feature parity with ethers library (yet) This workaround is needed to get the address
    // to convert to an alloy Address. This will not be needed when the alloy library has wallet support.
    let address: [u8; 20] = wallet.address().into();

    (wallet, address.into())
}

#[fixture]
fn allocation_ids() -> Vec<Address> {
    vec![
        Address::from_str("0xabababababababababababababababababababab").unwrap(),
        Address::from_str("0xdeaddeaddeaddeaddeaddeaddeaddeaddeaddead").unwrap(),
        Address::from_str("0xbeefbeefbeefbeefbeefbeefbeefbeefbeefbeef").unwrap(),
        Address::from_str("0x1234567890abcdef1234567890abcdef12345678").unwrap(),
    ]
}

#[fixture]
fn sender_ids() -> Vec<Address> {
    vec![
        Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
        Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
        Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
        keys().1,
    ]
}

#[fixture]
fn receipt_storage() -> Arc<RwLock<HashMap<u64, ReceivedReceipt>>> {
    Arc::new(RwLock::new(HashMap::new()))
}

#[fixture]
fn auditor_executor() -> (AuditorExecutorMock, EscrowStorage, QueryAppraisals) {
    let sender_escrow_storage = Arc::new(RwLock::new(HashMap::new()));

    let query_appraisal_storage = Arc::new(RwLock::new(HashMap::new()));
    (
        AuditorExecutorMock::new(sender_escrow_storage.clone()),
        sender_escrow_storage,
        query_appraisal_storage,
    )
}

#[fixture]
fn domain_separator() -> Eip712Domain {
    tap_eip712_domain(1, Address::from([0x11u8; 20]))
}

#[fixture]
fn checks(
    domain_separator: Eip712Domain,
    auditor_executor: (AuditorExecutorMock, EscrowStorage, QueryAppraisals),
    receipt_storage: Arc<RwLock<HashMap<u64, ReceivedReceipt>>>,
    allocation_ids: Vec<Address>,
    sender_ids: Vec<Address>,
) -> Vec<ReceiptCheck> {
    let (_, _escrow_storage, query_appraisal_storage) = auditor_executor;
    get_full_list_of_checks(
        domain_separator,
        sender_ids.iter().cloned().collect(),
        Arc::new(RwLock::new(allocation_ids.iter().cloned().collect())),
        receipt_storage,
        query_appraisal_storage,
    )
}

#[rstest]
#[tokio::test]
async fn initialization_valid_receipt(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    checks: Vec<ReceiptCheck>,
) {
    let signed_receipt = EIP712SignedMessage::new(
        &domain_separator,
        Receipt::new(allocation_ids[0], 10).unwrap(),
        &keys.0,
    )
    .unwrap();
    let query_id = 1;

    let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);

    let received_receipt = match received_receipt {
        ReceivedReceipt::Checking(checking) => checking,
        _ => panic!("ReceivedReceipt should be in Checking state"),
    };

    assert!(received_receipt.completed_checks_with_errors().is_empty());
    assert!(received_receipt.incomplete_checks().len() == checks.len());
}

#[rstest]
#[tokio::test]
async fn partial_then_full_check_valid_receipt(
    keys: (LocalWallet, Address),
    domain_separator: Eip712Domain,
    allocation_ids: Vec<Address>,
    auditor_executor: (AuditorExecutorMock, EscrowStorage, QueryAppraisals),
    checks: Vec<ReceiptCheck>,
) {
    let (executor, escrow_storage, query_appraisal_storage) = auditor_executor;
    // give receipt 5 second variance for min start time
    let _starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;
    let _receipt_auditor = ReceiptAuditor::new(domain_separator.clone(), executor);

    let query_value = 20u128;
    let signed_receipt = EIP712SignedMessage::new(
        &domain_separator,
        Receipt::new(allocation_ids[0], query_value).unwrap(),
        &keys.0,
    )
    .unwrap();

    let query_id = 1;

    // prepare adapters and storage to correctly validate receipt

    // add escrow for sender
    escrow_storage
        .write()
        .await
        .insert(keys.1, query_value + 500);
    // appraise query
    query_appraisal_storage
        .write()
        .await
        .insert(query_id, query_value);

    let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);
    let _receipt_id = 0u64;

    let mut received_receipt = match received_receipt {
        ReceivedReceipt::Checking(checking) => checking,
        _ => panic!("ReceivedReceipt should be in Checking state"),
    };

    // perform single arbitrary check
    let arbitrary_check_to_perform = checks[0].typetag_name();

    received_receipt
        .perform_check(arbitrary_check_to_perform)
        .await;
    assert!(received_receipt.check_is_complete(arbitrary_check_to_perform));

    received_receipt
        .perform_checks(&checks.iter().map(|c| c.typetag_name()).collect::<Vec<_>>())
        .await;
    assert!(received_receipt.checking_is_complete());
}

#[rstest]
#[tokio::test]
async fn partial_then_finalize_valid_receipt(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    auditor_executor: (AuditorExecutorMock, EscrowStorage, QueryAppraisals),
    checks: Vec<ReceiptCheck>,
) {
    let (executor, escrow_storage, query_appraisal_storage) = auditor_executor;
    // give receipt 5 second variance for min start time
    let _starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;
    let _receipt_auditor = ReceiptAuditor::new(domain_separator.clone(), executor);

    let query_value = 20u128;
    let signed_receipt = EIP712SignedMessage::new(
        &domain_separator,
        Receipt::new(allocation_ids[0], query_value).unwrap(),
        &keys.0,
    )
    .unwrap();

    let query_id = 1;

    // prepare adapters and storage to correctly validate receipt

    // add escrow for sender
    escrow_storage
        .write()
        .await
        .insert(keys.1, query_value + 500);
    // appraise query
    query_appraisal_storage
        .write()
        .await
        .insert(query_id, query_value);

    let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);
    let _receipt_id = 0u64;

    let mut received_receipt = match received_receipt {
        ReceivedReceipt::Checking(checking) => checking,
        _ => panic!("ReceivedReceipt should be in Checking state"),
    };

    // perform single arbitrary check
    let arbitrary_check_to_perform = checks[0].typetag_name();

    received_receipt
        .perform_check(arbitrary_check_to_perform)
        .await;
    assert!(received_receipt.check_is_complete(arbitrary_check_to_perform));

    let awaiting_escrow_receipt = received_receipt.finalize_receipt_checks().await;
    println!("{:?}", awaiting_escrow_receipt);
    assert!(awaiting_escrow_receipt.is_ok());

    let awaiting_escrow_receipt = awaiting_escrow_receipt.unwrap();
    let receipt = awaiting_escrow_receipt
        .check_and_reserve_escrow(&_receipt_auditor)
        .await;
    assert!(receipt.is_ok());
}

#[rstest]
#[tokio::test]
async fn standard_lifetime_valid_receipt(
    keys: (LocalWallet, Address),
    allocation_ids: Vec<Address>,
    domain_separator: Eip712Domain,
    auditor_executor: (AuditorExecutorMock, EscrowStorage, QueryAppraisals),
    checks: Vec<ReceiptCheck>,
) {
    let (executor, escrow_storage, query_appraisal_storage) = auditor_executor;
    // give receipt 5 second variance for min start time
    let _starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;
    let _receipt_auditor = ReceiptAuditor::new(domain_separator.clone(), executor);

    let query_value = 20u128;
    let signed_receipt = EIP712SignedMessage::new(
        &domain_separator,
        Receipt::new(allocation_ids[0], query_value).unwrap(),
        &keys.0,
    )
    .unwrap();

    let query_id = 1;

    // prepare adapters and storage to correctly validate receipt

    // add escrow for sender
    escrow_storage
        .write()
        .await
        .insert(keys.1, query_value + 500);
    // appraise query
    query_appraisal_storage
        .write()
        .await
        .insert(query_id, query_value);

    let received_receipt = ReceivedReceipt::new(signed_receipt, query_id, &checks);
    let _receipt_id = 0u64;

    let received_receipt = match received_receipt {
        ReceivedReceipt::Checking(checking) => checking,
        _ => panic!("ReceivedReceipt should be in Checking state"),
    };

    assert!(received_receipt.finalize_receipt_checks().await.is_ok());
}
