use crate::{
    bip32_extended_derivation::extended_bip32_derivation, types::BitcoinAddressError,
    AddAddressWithParametersError, BitcoinAgent, EcdsaPubKey, ManagementCanister, UtxosState,
    MIN_CONFIRMATIONS_UPPER_BOUND,
};
use bitcoin::{
    blockdata::{opcodes, script::Builder},
    hashes,
    hashes::Hash,
    util,
    util::address::Payload,
    Address, AddressType, Network, PublicKey, ScriptHash,
};

/// Returns the public key from a given Bitcoin ECDSA public key.
pub(crate) fn get_btc_public_key_from_ecdsa_public_key(
    ecdsa_public_key: &EcdsaPubKey,
) -> Result<PublicKey, bitcoin::util::key::Error> {
    PublicKey::from_slice(&ecdsa_public_key.public_key)
}

/// Adds an address based on the provided derivation path and address type to the list of managed addresses.
/// A minimum number of confirmations must further be specified, which is used when calling `get_utxos` and `get_balance`.
/// Returns the derived address if the operation is successful and an error otherwise.
pub(crate) fn add_address_with_parameters(
    bitcoin_agent: &mut BitcoinAgent<impl ManagementCanister>,
    derivation_path: &[Vec<u8>],
    address_type: &crate::AddressType,
    min_confirmations: u32,
) -> Result<Address, AddAddressWithParametersError> {
    if min_confirmations > MIN_CONFIRMATIONS_UPPER_BOUND {
        return Err(AddAddressWithParametersError::MinConfirmationsTooHigh);
    }
    if derivation_path.len() > 255 {
        return Err(AddAddressWithParametersError::DerivationPathTooLong);
    }
    let address = add_address_from_extended_path(
        bitcoin_agent,
        derivation_path,
        address_type,
        min_confirmations,
    );
    Ok(address)
}

/// Returns the public key and address of the derived child from the given public key, chain code, derivation path, address type and network.
pub(crate) fn derive_ecdsa_public_key_and_address_from_extended_path(
    derivation_path: &[Vec<u8>],
    address_type: &crate::AddressType,
    network: &Network,
    ecdsa_public_key: &EcdsaPubKey,
) -> (EcdsaPubKey, Address) {
    let (child_public_key, child_chain_code) = extended_bip32_derivation(
        &ecdsa_public_key.public_key,
        &ecdsa_public_key.chain_code,
        derivation_path,
    );

    let child_ecdsa_public_key = EcdsaPubKey {
        public_key: child_public_key,
        chain_code: child_chain_code,
        derivation_path: ecdsa_public_key
            .derivation_path
            .iter()
            .cloned()
            .chain(derivation_path.iter().cloned())
            .collect(),
    };
    let address = get_address(network, address_type, &child_ecdsa_public_key).unwrap();

    (child_ecdsa_public_key, address)
}

/// Adds the address for the given extended derivation path and address type to the given BitcoinAgent if the derived address is not already managed.
/// This function assumes that the passed derivation path is an extended path. This assumption has to be checked in the caller function.
pub(crate) fn add_address_from_extended_path(
    bitcoin_agent: &mut BitcoinAgent<impl ManagementCanister>,
    derivation_path: &[Vec<u8>],
    address_type: &crate::AddressType,
    min_confirmations: u32,
) -> Address {
    let (ecdsa_public_key, address) = derive_ecdsa_public_key_and_address_from_extended_path(
        derivation_path,
        address_type,
        &bitcoin_agent.management_canister.get_network(),
        &bitcoin_agent.management_canister.get_ecdsa_public_key(),
    );
    if !bitcoin_agent.ecdsa_pub_key_addresses.contains_key(&address) {
        bitcoin_agent
            .ecdsa_pub_key_addresses
            .insert(address.clone(), ecdsa_public_key);
        let utxos_state = UtxosState::new(min_confirmations);
        bitcoin_agent
            .utxos_state_addresses
            .insert(address.clone(), utxos_state);
    }
    address
}

/// Removes the given address from given BitcoinAgent managed addresses.
/// The address is removed if it is already managed and if it is different from the main address.
/// Returns true if the removal was successful, false otherwise.
pub(crate) fn remove_address(
    bitcoin_agent: &mut BitcoinAgent<impl ManagementCanister>,
    address: &Address,
) -> bool {
    let address_can_be_removed = bitcoin_agent.ecdsa_pub_key_addresses.contains_key(address)
        && *address != bitcoin_agent.get_main_address();
    if address_can_be_removed {
        bitcoin_agent.ecdsa_pub_key_addresses.remove(address);
        bitcoin_agent.utxos_state_addresses.remove(address);
    }
    address_can_be_removed
}

/// Returns the managed addresses according to given BitcoinAgent.
pub(crate) fn list_addresses(
    bitcoin_agent: &BitcoinAgent<impl ManagementCanister>,
) -> Vec<&Address> {
    bitcoin_agent.ecdsa_pub_key_addresses.keys().collect()
}

/// Returns the P2PKH address from a given network and public key.
pub(crate) fn get_p2pkh_address(
    network: &Network,
    ecdsa_public_key: &EcdsaPubKey,
) -> Result<Address, util::key::Error> {
    Ok(Address::p2pkh(
        &get_btc_public_key_from_ecdsa_public_key(ecdsa_public_key)?,
        *network,
    ))
}

/// Returns the P2SH address from a given network and script hash.
pub(crate) fn get_p2sh_address(
    network: &Network,
    script_hash: &[u8],
) -> Result<Address, hashes::error::Error> {
    Ok(Address {
        network: *network,
        payload: Payload::ScriptHash(ScriptHash::from_slice(script_hash)?),
    })
}

/// Returns the P2SH address from a given network and public key.
pub(crate) fn get_p2sh_address_for_pub_key(
    network: &Network,
    ecdsa_public_key: &EcdsaPubKey,
) -> Result<Address, BitcoinAddressError> {
    let public_key = get_btc_public_key_from_ecdsa_public_key(ecdsa_public_key)?;
    let public_key_hash = public_key.pubkey_hash();
    let script = Builder::new()
        .push_slice(&public_key_hash[..])
        .push_opcode(opcodes::all::OP_CHECKSIG)
        .into_script();
    Ok(get_p2sh_address(
        network,
        &script.script_hash().to_ascii_lowercase(),
    )?)
}

/// Returns the P2WPKH address from a given network and public key.
pub(crate) fn get_p2wpkh_address(
    network: &Network,
    ecdsa_public_key: &EcdsaPubKey,
) -> Result<Address, BitcoinAddressError> {
    Ok(Address::p2wpkh(
        &get_btc_public_key_from_ecdsa_public_key(ecdsa_public_key)?,
        *network,
    )?)
}

/// Returns the Bitcoin address from a given network, address type and ECDSA public key.
fn get_address(
    network: &Network,
    address_type: &crate::AddressType,
    ecdsa_public_key: &EcdsaPubKey,
) -> Result<Address, BitcoinAddressError> {
    match get_bitcoin_address_type(address_type) {
        AddressType::P2pkh => Ok(get_p2pkh_address(network, ecdsa_public_key)?),
        AddressType::P2sh => get_p2sh_address_for_pub_key(network, ecdsa_public_key),
        AddressType::P2wpkh => get_p2wpkh_address(network, ecdsa_public_key),
        // TODO (ER-2639): Add more address types (especially P2wsh)
        // Other cases can't happen see BitcoinAgent::new
        _ => panic!(),
    }
}

/// Returns the Bitcoin address for a given network, address type, and ECDSA public key.
pub(crate) fn get_main_address(
    management_canister: &impl ManagementCanister,
    address_type: &crate::AddressType,
) -> Address {
    get_address(
        &management_canister.get_network(),
        address_type,
        &management_canister.get_ecdsa_public_key(),
    )
    .unwrap()
}

/// Returns the bitcoin::AddressType converted from an crate::AddressType
pub(crate) fn get_bitcoin_address_type(address_type: &crate::AddressType) -> AddressType {
    match address_type {
        crate::AddressType::P2pkh => AddressType::P2pkh,
        crate::AddressType::P2sh => AddressType::P2sh,
        crate::AddressType::P2wpkh => AddressType::P2wpkh,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{agent, canister_mock::ManagementCanisterMock};
    use bitcoin::{
        secp256k1::{Secp256k1, SecretKey},
        util::bip32::{ChainCode, ChildNumber, ExtendedPrivKey},
        PrivateKey,
    };
    use std::{cell::RefCell, collections::HashSet, str::FromStr};

    /// Returns the parsed `AddressType` based on a generated address of given `address_type`.
    fn get_parsed_address_type_from_generated_address(
        address_type: &crate::AddressType,
    ) -> AddressType {
        let bitcoin_agent = agent::tests::new_mock(&crate::Network::Regtest, address_type);
        bitcoin_agent.get_main_address().address_type().unwrap()
    }

    /// Check that `get_main_address` returns an address of the correct type according to Bitcoin agent `main_address_type`.
    #[test]
    fn check_get_main_address() {
        for address_type in &[
            crate::AddressType::P2pkh,
            crate::AddressType::P2sh,
            crate::AddressType::P2wpkh,
        ] {
            assert_eq!(
                get_parsed_address_type_from_generated_address(address_type),
                get_bitcoin_address_type(address_type)
            )
        }
    }

    /// Returns `bitcoin_agent` addresses as a `Vec<Address>`
    fn list_addresses(bitcoin_agent: &BitcoinAgent<ManagementCanisterMock>) -> Vec<Address> {
        bitcoin_agent
            .list_addresses()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Returns a `HashSet<Address>` from the given address vector reference.
    fn to_hashset(v: &[Address]) -> HashSet<Address> {
        HashSet::from_iter(v.iter().cloned())
    }

    /// Returns true if the two given vector references contain the same addresses without considering the order, otherwise false.
    fn contains_same_addresses(v0: &[Address], v1: &[Address]) -> bool {
        to_hashset(v0) == to_hashset(v1)
    }

    /// Check that `add_address`, `remove_address` and `list_addresses` respectively add, remove and list managed addresses.
    #[test]
    fn check_managed_addresses() {
        let address_type = &crate::AddressType::P2pkh;
        let bitcoin_agent = &mut agent::tests::new_mock(&crate::Network::Regtest, address_type);
        let mut addresses = list_addresses(bitcoin_agent);

        let address = bitcoin_agent.add_address(&[vec![0]]).unwrap();

        addresses.push(address.clone());
        assert!(contains_same_addresses(
            &list_addresses(bitcoin_agent),
            &addresses
        ));

        assert!(bitcoin_agent.remove_address(&address));
        addresses.pop();
        assert!(contains_same_addresses(
            &list_addresses(bitcoin_agent),
            &addresses
        ));
    }

    // A private key in WIF (wallet import format). This is only for testing purposes.
    const BTC_PRIVATE_KEY_WIF: &str = "L2C1QgyKqNgfV7BpEPAm6PVn2xW8zpXq6MojSbWdH18nGQF2wGsT";

    thread_local! {
        static BTC_PRIVATE_KEY: RefCell<PrivateKey> =
            RefCell::new(PrivateKey::from_wif(BTC_PRIVATE_KEY_WIF).unwrap());
    }

    /// Returns the Bitcoin private key.
    pub(crate) fn get_btc_private_key() -> PrivateKey {
        BTC_PRIVATE_KEY.with(|private_key| *private_key.borrow())
    }

    /// Returns the Bitcoin public key.
    pub(crate) fn get_btc_public_key() -> PublicKey {
        get_btc_private_key().public_key(&Secp256k1::new())
    }

    /// Returns the Bitcoin ECDSA public key from a given public key.
    pub(crate) fn get_btc_ecdsa_public_key_from_public_key(public_key: &PublicKey) -> EcdsaPubKey {
        EcdsaPubKey {
            public_key: public_key.to_bytes(),
            chain_code: vec![],
            derivation_path: vec![],
        }
    }

    /// Returns the Bitcoin ECDSA public key.
    pub fn get_btc_ecdsa_public_key() -> EcdsaPubKey {
        get_btc_ecdsa_public_key_from_public_key(&get_btc_public_key())
    }

    /// Returns the `ChildNumber` (u31) associated with a given vector of at most four `u8`s.
    /// Assuming that the most significant bit of the first byte is zero, classifying `child_bytes` as an unhardened derivation path.
    pub(crate) fn get_child_number(child_bytes: &[u8]) -> ChildNumber {
        let mut index = (child_bytes[0] as u32) << 24;
        if child_bytes.len() > 1 {
            index |= (child_bytes[1] as u32) << 16;
            if child_bytes.len() > 2 {
                index |= (child_bytes[2] as u32) << 8;
                if child_bytes.len() > 3 {
                    index |= child_bytes[3] as u32;
                }
            }
        }
        ChildNumber::Normal { index }
    }

    /// Returns the private key of the derived child from the given private key, chain code and unhardened derivation path.
    pub(crate) fn derive_child_private_key(
        private_key: &[u8],
        chain_code: &[u8],
        derivation_path: &[Vec<u8>],
    ) -> Vec<u8> {
        let child_number_vec: Vec<ChildNumber> = derivation_path
            .iter()
            .map(|child_bytes| get_child_number(child_bytes))
            .collect();
        let parent_extended_private_key = ExtendedPrivKey {
            network: Network::Bitcoin, // The network isn't taken into account when deriving a child private key.
            depth: 0,
            parent_fingerprint: Default::default(),
            child_number: ChildNumber::Normal { index: 0 },
            private_key: SecretKey::from_slice(private_key).unwrap(),
            chain_code: ChainCode::from(&*chain_code),
        };
        parent_extended_private_key
            .derive_priv(&Secp256k1::new(), &child_number_vec)
            .unwrap()
            .private_key
            .secret_bytes()
            .to_vec()
    }

    /// Check that the keys and address of the derived child match those expected from the given keys, chain code and derivation path.
    fn test_derive_ecdsa_keys_and_address_from_extended_path(
        private_key: &str,
        chain_code: &str,
        derivation_path: &[Vec<u8>],
        expected_public_key: &str,
        expected_child_private_key: &str,
        expected_child_public_key: &str,
        expected_child_address: &str,
    ) {
        assert_eq!(
            PrivateKey::from_slice(&hex::decode(private_key).unwrap(), Network::Bitcoin)
                .unwrap()
                .public_key(&Secp256k1::new())
                .to_bytes(),
            hex::decode(expected_public_key).unwrap()
        );
        let chain_code = &hex::decode(chain_code).unwrap();
        let ecdsa_private_key = derive_child_private_key(
            &hex::decode(private_key).unwrap(),
            chain_code,
            derivation_path,
        );
        assert_eq!(
            ecdsa_private_key,
            hex::decode(expected_child_private_key).unwrap()
        );
        let (ecdsa_public_key, address) = derive_ecdsa_public_key_and_address_from_extended_path(
            derivation_path,
            &crate::AddressType::P2pkh,
            &Network::Bitcoin,
            &EcdsaPubKey {
                public_key: PublicKey::from_str(expected_public_key).unwrap().to_bytes(),
                chain_code: chain_code.to_vec(),
                derivation_path: vec![],
            },
        );
        assert_eq!(
            ecdsa_public_key.public_key.to_vec(),
            hex::decode(expected_child_public_key).unwrap()
        );
        assert_eq!(address.to_string(), expected_child_address);
    }

    #[test]
    fn test_derive_ecdsa_keys_and_address_from_extended_path_2147483647() {
        test_derive_ecdsa_keys_and_address_from_extended_path(
            "5c22f8937210130ad1bbc50678a7c0a119a483d47928c323bf0baa3a57fa547d",
            "180c998615636cd875aa70c71cfa6b7bf570187a56d8c6d054e60b644d13e9d3",
            &[vec![0x7F, 0xFF, 0xFF, 0xFF]],
            "023e4740d0ba639e28963f3476157b7cf2fb7c6fdf4254f97099cf8670b505ea59",
            "3e25c4e02adb04477b48b70cfa5d4e2e76cc04630bbe99d794e59ba5bc0821c3",
            "023646dd63e956c0c956059fb45e10e0223be698357b20cc9196a2fda7ff858e35",
            "1MmXtA99GMUGU2PxEro3hZFizSgb9Cn2nw",
        );
    }

    #[test]
    fn test_derive_ecdsa_keys_and_address_from_extended_path_1_2_3() {
        test_derive_ecdsa_keys_and_address_from_extended_path(
            "bf9bd979a532ba3920b17a2789cfc3594bd6016c3ccaea32f82045f71006d26e",
            "8b0d0b42b81f535fb8d7637c93255ac5a6976a8adc045cfc1d214e2cf468c765",
            &[vec![0, 0, 0, 1], vec![0, 0, 0, 2], vec![0, 0, 0, 3]],
            "02b30058c39a7372de41973a792cc6d3faaa29a813ec85530f7ec60b79cb5c2260",
            "aad2ec290613578c55ac5c982a098a2ded2bdbd36ecbb2cc90c620927dbe930f",
            "03399311d21adc7fd7e042b747ee0bb1fc62fe9917a7f57ade3e9fa2c79d2b9aa8",
            "18nddgjnWYWAHrA5sEeNjVFfEkh3B847yk",
        );
    }

    #[test]
    fn test_derive_ecdsa_keys_and_address_from_extended_path_1() {
        test_derive_ecdsa_keys_and_address_from_extended_path(
            &hex::encode(get_btc_private_key().to_bytes()),
            "d84e7baa7130e741f75c23062e514cba7d3acc4dbeb3b269cb12f37d3d57aae0",
            &[vec![0, 0, 0, 1]],
            "02110b3982b01e5429b75c2dbd6227ee9a818780af1b0c2a3b5b00db19b6116b0d",
            "f3a4d55c03f6ba8e5bfa505f947c031412a56bf9c31c06d82c48e75419d3d213",
            "03464a43b0c32c9ae34fc5c00c368c82e208192b0c3ee9d17ab7413537e33a3f57",
            "1KbzFs186EhWeDjzQHqWab3Le5rmGGsGn",
        );
    }
}
