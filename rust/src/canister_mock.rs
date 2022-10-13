use crate::{
    address_management::{get_main_address, tests::derive_child_private_key},
    canister_common::ManagementCanister,
    types::{from_types_network_to_bitcoin_network, GetUtxosResponse},
    utxo_management::has_utxo_min_confirmations,
    AddressType, BalanceUpdate, BitcoinAgent, EcdsaPubKey, Fee, GetUtxosError,
    ManagementCanisterReject, MillisatoshiPerByte, OutPoint, Satoshi, TransactionInfo, Utxo,
    UtxosUpdate, MIN_CONFIRMATIONS_UPPER_BOUND,
};
use async_trait::async_trait;
use bitcoin::{
    psbt::serialize::Deserialize,
    secp256k1::{Message, Secp256k1, SecretKey},
    Address, Network, Transaction,
};
use std::collections::BTreeMap;

/// The management canister mock is used to perform unit tests against the library.
pub struct ManagementCanisterMock {
    pub(crate) utxos_addresses: BTreeMap<Address, Vec<Utxo>>,
    network: Network,
    ecdsa_public_key: EcdsaPubKey,
    pub(crate) tip_height: u32,
    pending_transactions: Vec<Transaction>,
}

#[async_trait]
impl ManagementCanister for ManagementCanisterMock {
    /// Creates a new instance of the management canister mock.
    fn new(network: crate::Network) -> Self {
        Self::new_using_ecdsa_public_key(
            network,
            EcdsaPubKey {
                public_key: vec![],
                chain_code: vec![],
                derivation_path: vec![],
            },
        )
    }

    /// Creates a new instance of the management canister mock using a given ECDSA public key.
    fn new_using_ecdsa_public_key(network: crate::Network, ecdsa_public_key: EcdsaPubKey) -> Self {
        ManagementCanisterMock::new_using_ecdsa_public_key_test(
            network,
            ecdsa_public_key,
            AddressType::P2pkh,
        )
    }

    /// Initializes the management canister by initializing its ECDSA public key.
    fn set_ecdsa_public_key(&mut self, ecdsa_public_key: EcdsaPubKey) {
        self.ecdsa_public_key = ecdsa_public_key;
    }

    /// Returns the network the management canister interacts with.
    fn get_network(&self) -> Network {
        self.network
    }

    /// Returns the ECDSA public key of this canister.
    fn get_ecdsa_public_key(&self) -> EcdsaPubKey {
        self.ecdsa_public_key.clone()
    }

    /// Returns the mock UTXOs of the canister address according to `min_confirmations`.
    /// Note: `address` is ignored for simplicity purpose.
    async fn get_utxos(
        &self,
        _address: &Address,
        _min_confirmations: u32,
    ) -> Result<GetUtxosResponse, GetUtxosError> {
        unreachable!()
    }

    /// Returns fees as percentiles in millisatoshis/byte over the last 10,000 transactions.
    async fn get_current_fees(&self) -> Result<Vec<MillisatoshiPerByte>, ManagementCanisterReject> {
        unreachable!()
    }

    /// Returns the DER signature of the given `message_hash` associated with the ECDSA public key of this canister at the given derivation path.
    async fn sign_with_ecdsa(
        &self,
        _derivation_path: &[Vec<u8>],
        _message_hash: &[u8],
    ) -> Result<Vec<u8>, ManagementCanisterReject> {
        unreachable!()
    }

    /// Sends the given transaction to the network the management canister interacts with.
    async fn send_transaction(
        &mut self,
        _transaction: Vec<u8>,
        _network: Network,
    ) -> Result<(), ManagementCanisterReject> {
        unreachable!()
    }
}

impl ManagementCanisterMock {
    /// Creates a new instance of the management canister mock using a given ECDSA public key.
    pub(crate) fn new_using_ecdsa_public_key_test(
        network: crate::Network,
        ecdsa_public_key: EcdsaPubKey,
        address_type: AddressType,
    ) -> Self {
        let mut management_canister = Self {
            utxos_addresses: BTreeMap::default(),
            network: from_types_network_to_bitcoin_network(network),
            ecdsa_public_key: ecdsa_public_key.clone(),
            tip_height: MIN_CONFIRMATIONS_UPPER_BOUND,
            pending_transactions: vec![],
        };
        if !ecdsa_public_key.public_key.is_empty() {
            let main_address = get_main_address(&management_canister, &address_type);
            management_canister.utxos_addresses =
                BTreeMap::from([(main_address, get_init_utxos())]);
        }
        management_canister
    }

    pub(crate) fn internal_get_utxos(
        &self,
        address: &Address,
        min_confirmations: u32,
    ) -> GetUtxosResponse {
        let utxos = self
            .utxos_addresses
            .get(address)
            .unwrap_or(&vec![])
            .iter()
            .filter(|utxo| has_utxo_min_confirmations(utxo, self.tip_height, min_confirmations))
            .cloned()
            .collect();
        GetUtxosResponse {
            utxos,
            tip_height: self.tip_height,
        }
    }

    pub(crate) fn internal_get_current_fees(&self) -> Vec<MillisatoshiPerByte> {
        (1_000..100_000).step_by(1_000).collect()
    }

    pub(crate) fn internal_sign_with_ecdsa(
        &self,
        private_key: &[u8],
        chain_code: &[u8],
        derivation_path: &[Vec<u8>],
        message_hash: &[u8],
    ) -> Vec<u8> {
        let child_private_key = derive_child_private_key(private_key, chain_code, derivation_path);
        Secp256k1::new()
            .sign_ecdsa(
                &Message::from_slice(message_hash).unwrap(),
                &SecretKey::from_slice(&child_private_key).unwrap(),
            )
            .serialize_der()
            .to_vec()
    }

    pub(crate) fn internal_send_transaction(&mut self, transaction: Vec<u8>, _network: Network) {
        self.pending_transactions
            .push(Transaction::deserialize(&transaction).unwrap());
    }
}

pub(crate) fn get_utxos(
    bitcoin_agent: &BitcoinAgent<ManagementCanisterMock>,
    address: &Address,
    min_confirmations: u32,
) -> Vec<Utxo> {
    let get_utxos_args = bitcoin_agent.get_utxos_args(address, min_confirmations);
    bitcoin_agent
        .get_utxos_from_args_test(get_utxos_args)
        .unwrap()
        .utxos
}

pub(crate) fn get_balance(
    bitcoin_agent: &BitcoinAgent<ManagementCanisterMock>,
    address: &Address,
    min_confirmations: u32,
) -> Satoshi {
    let get_utxos_args = bitcoin_agent.get_utxos_args(address, min_confirmations);
    bitcoin_agent
        .get_balance_from_args_test(get_utxos_args)
        .unwrap()
}

pub(crate) fn get_balance_update(
    bitcoin_agent: &mut BitcoinAgent<ManagementCanisterMock>,
    address: &Address,
    min_confirmations: u32,
) -> BalanceUpdate {
    let get_utxos_args = bitcoin_agent.get_utxos_args(address, min_confirmations);
    let get_utxos_result = bitcoin_agent
        .get_utxos_from_args_test(get_utxos_args)
        .unwrap();
    bitcoin_agent.apply_utxos(get_utxos_result);
    bitcoin_agent.get_balance_update(address).unwrap()
}

pub(crate) fn get_current_fees(
    bitcoin_agent: &BitcoinAgent<ManagementCanisterMock>,
) -> Vec<MillisatoshiPerByte> {
    let get_current_fees_args = bitcoin_agent.get_current_fees_args();
    bitcoin_agent
        .get_current_fees_from_args_test(get_current_fees_args)
        .unwrap()
}

pub(crate) async fn multi_transfer(
    bitcoin_agent: &mut BitcoinAgent<ManagementCanisterMock>,
    payouts: &BTreeMap<Address, Satoshi>,
    change_address: &Address,
    fee: Fee,
    min_confirmations: u32,
    replaceable: bool,
) -> TransactionInfo {
    let multi_transfer_args = bitcoin_agent.get_multi_transfer_args(
        payouts,
        change_address,
        fee,
        min_confirmations,
        replaceable,
    );
    let multi_transfer_result = bitcoin_agent
        .multi_transfer_from_args_test(multi_transfer_args)
        .await
        .unwrap();
    bitcoin_agent.apply_multi_transfer_result(&multi_transfer_result);
    multi_transfer_result.transaction_info
}

/// Gets some hard-coded UTXOs to be used by the mock.
pub(crate) fn get_init_utxos() -> Vec<Utxo> {
    vec![Utxo {
        outpoint: OutPoint {
            txid: vec![0; 32],
            vout: 0,
        },
        value: 250_000,
        height: MIN_CONFIRMATIONS_UPPER_BOUND,
    }]
}

/// Gets the initial balance to be used by the mock.
pub(crate) fn get_init_balance() -> Satoshi {
    get_init_utxos().iter().map(|utxo| utxo.value).sum()
}

/// Gets the initial UTXOs update to be used by the mock.
pub(crate) fn get_init_utxos_update() -> UtxosUpdate {
    UtxosUpdate {
        added_utxos: get_init_utxos(),
        removed_utxos: vec![],
    }
}

/// Gets the initial balance update to be used by the mock.
pub(crate) fn get_init_balance_update() -> BalanceUpdate {
    BalanceUpdate::from(get_init_utxos_update())
}

/// Returns an `ic_btc_types::OutPoint` from a given `bitcoin::OutPoint`.
pub(crate) fn get_outpoint_ic_type(outpoint_btc_type: bitcoin::OutPoint) -> OutPoint {
    OutPoint {
        txid: outpoint_btc_type.txid.to_vec(),
        vout: outpoint_btc_type.vout,
    }
}

pub(crate) fn mine_block(management_canister_mock: &mut ManagementCanisterMock) {
    management_canister_mock
        .pending_transactions
        .iter()
        .for_each(|transaction| {
            // Consumes UTXOs from the given transaction inputs.
            transaction.input.iter().for_each(|input| {
                management_canister_mock
                    .utxos_addresses
                    .clone()
                    .keys()
                    .for_each(|address| {
                        let address_utxos = &mut management_canister_mock
                            .utxos_addresses
                            .get_mut(address)
                            .unwrap();
                        let outpoint_ic_type = get_outpoint_ic_type(input.previous_output);
                        address_utxos.retain(|utxo| utxo.outpoint != outpoint_ic_type);
                    });
            });
            let tx_id = transaction.txid().to_vec();
            // Generates UTXOs from the given transaction outputs.
            transaction
                .output
                .iter()
                .enumerate()
                .for_each(|(outputs_index, output)| {
                    let address = Address::from_script(
                        &output.script_pubkey,
                        management_canister_mock.network,
                    )
                    .unwrap();
                    let new_utxo = Utxo {
                        outpoint: OutPoint {
                            txid: tx_id.clone(),
                            vout: outputs_index as u32,
                        },
                        value: output.value,
                        height: management_canister_mock.tip_height,
                    };
                    management_canister_mock
                        .utxos_addresses
                        .entry(address)
                        .or_insert_with(Vec::new)
                        .push(new_utxo);
                });
        });
    management_canister_mock.pending_transactions.clear();
    management_canister_mock.tip_height += 1;
}
