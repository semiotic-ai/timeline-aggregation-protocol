#[cfg(test)]
mod manager_unit_test {
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
        sync::{Arc, RwLock},
    };

    use crate::{
        adapters::{
            collateral_adapter_mock::CollateralAdapterMock,
            rav_storage_adapter_mock::RAVStorageAdapterMock,
            receipt_checks_adapter_mock::ReceiptChecksAdapterMock,
            receipt_storage_adapter_mock::ReceiptStorageAdapterMock,
        },
        eip_712_signed_message::EIP712SignedMessage,
        get_current_timestamp_u64_ns,
        tap_manager::Manager,
        tap_receipt::{get_full_list_of_checks, Receipt, ReceiptCheck},
    };
    use ethereum_types::Address;
    use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer};
    use rstest::*;

    #[fixture]
    fn keys() -> (LocalWallet, Address) {
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
         .phrase("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")
         .build()
         .unwrap();
        let address = wallet.address();
        (wallet, address)
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
    fn gateway_ids() -> Vec<Address> {
        vec![
            Address::from_str("0xfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfbfb").unwrap(),
            Address::from_str("0xfafafafafafafafafafafafafafafafafafafafa").unwrap(),
            Address::from_str("0xadadadadadadadadadadadadadadadadadadadad").unwrap(),
            keys().1,
        ]
    }

    #[fixture]
    fn rav_storage_adapter() -> RAVStorageAdapterMock {
        let rav_storage = Arc::new(RwLock::new(HashMap::new()));
        let rav_storage_adapter = RAVStorageAdapterMock::new(rav_storage);
        rav_storage_adapter
    }

    #[fixture]
    fn collateral_adapters() -> (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>) {
        let gateway_collateral_storage = Arc::new(RwLock::new(HashMap::new()));
        let collateral_adapter =
            CollateralAdapterMock::new(Arc::clone(&gateway_collateral_storage));
        (collateral_adapter, gateway_collateral_storage)
    }

    #[fixture]
    fn receipt_adapters() -> (
        ReceiptStorageAdapterMock,
        ReceiptChecksAdapterMock,
        Arc<RwLock<HashMap<u64, u128>>>,
    ) {
        let receipt_storage = Arc::new(RwLock::new(HashMap::new()));
        let receipt_storage_adapter = ReceiptStorageAdapterMock::new(Arc::clone(&receipt_storage));

        let allocation_ids_set = Arc::new(RwLock::new(HashSet::from_iter(allocation_ids())));
        let gateway_ids_set = Arc::new(RwLock::new(HashSet::from_iter(gateway_ids())));
        let query_appraisal_storage = Arc::new(RwLock::new(HashMap::new()));

        let receipt_checks_adapter = ReceiptChecksAdapterMock::new(
            Arc::clone(&receipt_storage),
            Arc::clone(&query_appraisal_storage),
            Arc::clone(&allocation_ids_set),
            Arc::clone(&gateway_ids_set),
        );

        (
            receipt_storage_adapter,
            receipt_checks_adapter,
            query_appraisal_storage,
        )
    }

    #[rstest]
    #[case::full_checks(get_full_list_of_checks())]
    #[case::partial_checks(vec![ReceiptCheck::CheckSignature])]
    #[case::no_checks(Vec::<ReceiptCheck>::new())]
    async fn manager_verify_and_store_varying_initial_checks(
        rav_storage_adapter: RAVStorageAdapterMock,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        #[case] initial_checks: Vec<ReceiptCheck>,
    ) {
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let mut manager = Manager::new(
            collateral_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );

        let query_id = 1;
        let value = 20u128;
        let signed_receipt =
            EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                .await
                .unwrap();
        query_appraisal_storage
            .write()
            .unwrap()
            .insert(query_id, value);
        collateral_storage.write().unwrap().insert(keys.1, 999999);

        assert!(manager
            .verify_and_store_receipt(signed_receipt, query_id, initial_checks)
            .is_ok());
    }

    #[rstest]
    #[case::full_checks(get_full_list_of_checks())]
    #[case::partial_checks(vec![ReceiptCheck::CheckSignature])]
    #[case::no_checks(Vec::<ReceiptCheck>::new())]
    async fn manager_create_rav_request_all_valid_receipts(
        rav_storage_adapter: RAVStorageAdapterMock,
        collateral_adapters: (CollateralAdapterMock, Arc<RwLock<HashMap<Address, u128>>>),
        receipt_adapters: (
            ReceiptStorageAdapterMock,
            ReceiptChecksAdapterMock,
            Arc<RwLock<HashMap<u64, u128>>>,
        ),
        keys: (LocalWallet, Address),
        allocation_ids: Vec<Address>,
        #[case] initial_checks: Vec<ReceiptCheck>,
    ) {
        let (collateral_adapter, collateral_storage) = collateral_adapters;
        let (receipt_storage_adapter, receipt_checks_adapter, query_appraisal_storage) =
            receipt_adapters;
        // give receipt 5 second variance for min start time
        let starting_min_timestamp = get_current_timestamp_u64_ns().unwrap() - 500000000;

        let mut manager = Manager::new(
            collateral_adapter,
            receipt_checks_adapter,
            rav_storage_adapter,
            receipt_storage_adapter,
            get_full_list_of_checks(),
            starting_min_timestamp,
        );

        let mut stored_signed_receipts = Vec::new();
        for query_id in 0..10 {
            let value = 20u128;
            let signed_receipt =
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .await
                    .unwrap();
            stored_signed_receipts.push(signed_receipt.clone());
            query_appraisal_storage
                .write()
                .unwrap()
                .insert(query_id, value);
            collateral_storage.write().unwrap().insert(keys.1, 999999);
            assert!(manager
                .verify_and_store_receipt(signed_receipt, query_id, initial_checks.clone())
                .is_ok());
        }
        let rav_request_result = manager.create_rav_request(0);
        assert!(rav_request_result.is_ok());

        let rav_request = rav_request_result.unwrap();
        // all passing
        assert_eq!(rav_request.0.len(), stored_signed_receipts.len());
        // no failing
        assert_eq!(rav_request.1.len(), 0);

        let signed_rav = EIP712SignedMessage::new(rav_request.2.clone(), &keys.0)
            .await
            .unwrap();
        assert!(manager
            .verify_and_store_rav(rav_request.2, signed_rav)
            .is_ok());
    }
}
