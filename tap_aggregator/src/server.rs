use crate::aggregator::check_and_aggregate_receipts;
use anyhow::Result;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::{core::async_trait, server::ServerHandle};
use k256::ecdsa::{SigningKey, VerifyingKey};
use tap_core::{
    eip_712_signed_message::EIP712SignedMessage,
    receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
};

#[rpc(server)]
pub trait Rpc {
    /// Returns a protocol version.
    #[method(name = "api_version")]
    async fn api_version(&self) -> Result<String, jsonrpsee::types::ErrorObjectOwned>;
    #[method(name = "aggregate_receipts")]
    async fn aggregate_receipts(
        &self,
        receipts: Vec<EIP712SignedMessage<Receipt>>,
        previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    ) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>, jsonrpsee::types::ErrorObjectOwned>;
}

struct RpcImpl {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    api_version: String,
}

#[async_trait]
impl RpcServer for RpcImpl {
    async fn api_version(&self) -> Result<String, jsonrpsee::types::ErrorObjectOwned> {
        Ok(self.api_version.clone())
    }
    async fn aggregate_receipts(
        &self,
        receipts: Vec<EIP712SignedMessage<Receipt>>,
        previous_rav: Option<EIP712SignedMessage<ReceiptAggregateVoucher>>,
    ) -> Result<EIP712SignedMessage<ReceiptAggregateVoucher>, jsonrpsee::types::ErrorObjectOwned>
    {
        let res = check_and_aggregate_receipts(
            &receipts,
            previous_rav,
            self.signing_key.clone(),
            self.verifying_key,
        );
        // handle error
        match res {
            Ok(res) => Ok(res),
            Err(e) => Err(jsonrpsee::types::ErrorObject::owned(
                -32000,
                e.to_string(),
                None::<()>,
            )),
        }
    }
}

pub async fn run_server(
    port: u16,
    signing_key: SigningKey,
) -> Result<(ServerHandle, std::net::SocketAddr)> {
    // Setting up the JSON RPC server
    println!("Starting server...");
    let server = ServerBuilder::default()
        .build(format!("127.0.0.1:{}", port))
        .await?;
    let addr = server.local_addr()?;
    println!("Listening on: {}", addr);
    let rpc_impl = RpcImpl {
        signing_key: signing_key.clone(),
        verifying_key: VerifyingKey::from(&signing_key),
        api_version: "🤷".into(),
    };
    let handle = server.start(rpc_impl.into_rpc())?;
    Ok((handle, addr))
}

#[cfg(test)]
mod tests {
    use crate::server;
    use ethereum_types::Address;
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::http_client::HttpClientBuilder;
    use jsonrpsee::rpc_params;
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rand_core::OsRng;
    use rstest::*;
    use std::str::FromStr;
    use tap_core::{
        eip_712_signed_message::EIP712SignedMessage,
        receipt_aggregate_voucher::ReceiptAggregateVoucher, tap_receipt::Receipt,
    };

    #[fixture]
    fn keys() -> (SigningKey, VerifyingKey) {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        (signing_key, verifying_key)
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

    #[rstest]
    #[tokio::test]
    async fn protocol_version(keys: (SigningKey, VerifyingKey)) {
        // Start the JSON-RPC server.
        let (handle, local_addr) = server::run_server(0, keys.0).await.unwrap();

        // Start the JSON-RPC client.
        let client = HttpClientBuilder::default()
            .build(format!("http://127.0.0.1:{}", local_addr.port()))
            .unwrap();
        let res: String = client
            .request("api_version", rpc_params!(None::<()>))
            .await
            .unwrap();

        // Print the result.
        println!("Result: {:?}", res);

        handle.stop().unwrap();
        handle.stopped().await;
    }

    #[rstest]
    #[case::basic_rav_test (vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts (vec![0,0,0,0])]
    #[tokio::test]
    async fn signed_rav_is_valid_with_no_previous_rav(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] values: Vec<u128>,
    ) {
        // Start the JSON-RPC server.
        let (handle, local_addr) = server::run_server(0, keys.0.clone()).await.unwrap();

        // Start the JSON-RPC client.
        let client = HttpClientBuilder::default()
            .build(format!("http://127.0.0.1:{}", local_addr.port()))
            .unwrap();

        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .unwrap(),
            );
        }

        // Skipping receipts validation in this test, aggregate_receipts assumes receipts are valid.
        // Create RAV through the JSON-RPC server.
        let remote_rav: EIP712SignedMessage<ReceiptAggregateVoucher> = client
            .request("aggregate_receipts", rpc_params!(&receipts, None::<()>))
            .await
            .unwrap();

        let local_rav =
            ReceiptAggregateVoucher::aggregate_receipts(allocation_ids[0], &receipts, None)
                .unwrap();

        assert!(remote_rav.message.allocation_id == local_rav.allocation_id);
        assert!(remote_rav.message.timestamp == local_rav.timestamp);
        assert!(remote_rav.message.value_aggregate == local_rav.value_aggregate);

        assert!(remote_rav.check_signature(keys.1).is_ok());

        handle.stop().unwrap();
        handle.stopped().await;
    }

    #[rstest]
    #[case::basic_rav_test (vec![45,56,34,23])]
    #[case::rav_from_zero_valued_receipts (vec![0,0,0,0])]
    #[tokio::test]
    async fn signed_rav_is_valid_with_previous_rav(
        keys: (SigningKey, VerifyingKey),
        allocation_ids: Vec<Address>,
        #[case] values: Vec<u128>,
    ) {
        // Start the JSON-RPC server.
        let (handle, local_addr) = server::run_server(0, keys.0.clone()).await.unwrap();

        // Start the JSON-RPC client.
        // let client = HttpClientBuilder::default()
        //     .build(format!("http://127.0.0.1:{}", local_addr.port()))
        //     .unwrap();

        let client = HttpClientBuilder::default()
            .build("http://127.0.0.1:8080").unwrap();

        // Create receipts
        let mut receipts = Vec::new();
        for value in values {
            receipts.push(
                EIP712SignedMessage::new(Receipt::new(allocation_ids[0], value).unwrap(), &keys.0)
                    .unwrap(),
            );
        }

        // Create previous RAV from first half of receipts locally
        let prev_rav = ReceiptAggregateVoucher::aggregate_receipts(
            allocation_ids[0],
            &receipts[0..receipts.len() / 2],
            None,
        )
        .unwrap();
        let signed_prev_rav = EIP712SignedMessage::new(prev_rav, &keys.0).unwrap();

        // Create new RAV from last half of receipts and prev_rav through the JSON-RPC server
        let rav: EIP712SignedMessage<ReceiptAggregateVoucher> = client
            .request(
                "aggregate_receipts",
                rpc_params!(
                    &receipts[receipts.len() / 2..receipts.len()],
                    Some(signed_prev_rav)
                ),
            )
            .await
            .unwrap();

        assert!(rav.check_signature(keys.1).is_ok());

        handle.stop().unwrap();
        handle.stopped().await;
    }
}
