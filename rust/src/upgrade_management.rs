use crate::{
    types::{from_bitcoin_network_to_types_network, from_types_network_to_bitcoin_network},
    AddressUsingPrimitives, BitcoinAgent, BitcoinAgentState, EcdsaPubKey, ManagementCanister,
    UtxosState,
};
use bitcoin::{Address, Network};
use std::{collections::BTreeMap, str::FromStr};

/// Returns the Bitcoin agent state.
pub(crate) fn get_state<C: ManagementCanister>(
    bitcoin_agent: &BitcoinAgent<C>,
) -> BitcoinAgentState {
    let ecdsa_pub_key_addresses: BTreeMap<AddressUsingPrimitives, EcdsaPubKey> = bitcoin_agent
        .ecdsa_pub_key_addresses
        .iter()
        .map(|(address, ecdsa_pub_key)| {
            (get_address_using_primitives(address), ecdsa_pub_key.clone())
        })
        .collect();

    let utxos_state_addresses: BTreeMap<AddressUsingPrimitives, UtxosState> = bitcoin_agent
        .utxos_state_addresses
        .iter()
        .map(|(address, utxos_state)| (get_address_using_primitives(address), utxos_state.clone()))
        .collect();

    BitcoinAgentState {
        network: from_bitcoin_network_to_types_network(
            bitcoin_agent.management_canister.get_network(),
        ),
        main_address_type: bitcoin_agent.main_address_type,
        ecdsa_pub_key_addresses,
        utxos_state_addresses,
        min_confirmations: bitcoin_agent.min_confirmations,
        ecdsa_pub_key: bitcoin_agent.management_canister.get_ecdsa_public_key(),
    }
}

/// Returns the associated Bitcoin agent with the given `bitcoin_agent_state`.
pub(crate) fn from_state<C: ManagementCanister>(
    bitcoin_agent_state: BitcoinAgentState,
) -> BitcoinAgent<C> {
    let ecdsa_pub_key_addresses: BTreeMap<Address, EcdsaPubKey> = bitcoin_agent_state
        .ecdsa_pub_key_addresses
        .into_iter()
        .map(|(address_using_primitives, ecdsa_pub_key)| {
            (get_address(address_using_primitives), ecdsa_pub_key)
        })
        .collect();

    let utxos_state_addresses: BTreeMap<Address, UtxosState> = bitcoin_agent_state
        .utxos_state_addresses
        .into_iter()
        .map(|(address_using_primitives, utxos_state)| {
            (get_address(address_using_primitives), utxos_state)
        })
        .collect();

    let management_canister = C::new_using_ecdsa_public_key(
        bitcoin_agent_state.network,
        bitcoin_agent_state.ecdsa_pub_key,
    );
    BitcoinAgent {
        management_canister,
        main_address_type: bitcoin_agent_state.main_address_type,
        ecdsa_pub_key_addresses,
        min_confirmations: bitcoin_agent_state.min_confirmations,
        utxos_state_addresses,
    }
}

/// Returns the `AddressUsingPrimitives` associated with a given `bitcoin::Address`.
pub(crate) fn get_address_using_primitives(address: &Address) -> AddressUsingPrimitives {
    (
        address.to_string(),
        from_bitcoin_network_to_types_network(address.network),
    )
}

/// Returns the `bitcoin::Address` associated with a given `AddressUsingPrimitives`.
pub(crate) fn get_address((address_string, address_network): AddressUsingPrimitives) -> Address {
    let mut address = Address::from_str(&address_string).unwrap();
    address.network = if cfg!(all(not(test), locally)) {
        Network::Regtest
    } else {
        from_types_network_to_bitcoin_network(address_network)
    };
    address
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{agent, canister_mock::ManagementCanisterMock, AddressType, Network};

    /// Check that `get_state` and `from_state` return respectively the Bitcoin agent state and the Bitcoin agent associated with the former Bitcoin agent state.
    #[test]
    fn check_upgrade() {
        // Every field of the `BitcoinAgentState` is filled with non-default value during the `BitcoinAgent` instantiation.
        let pre_upgrade_bitcoin_agent =
            agent::tests::new_mock(&Network::Regtest, &AddressType::P2pkh);

        let pre_upgrade_state = pre_upgrade_bitcoin_agent.get_state();
        let post_upgrade_bitcoin_agent: BitcoinAgent<ManagementCanisterMock> =
            BitcoinAgent::from_state(pre_upgrade_state.clone());

        assert_eq!(post_upgrade_bitcoin_agent.get_state(), pre_upgrade_state)
    }
}
