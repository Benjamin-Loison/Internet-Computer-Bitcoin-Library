import Iter "mo:base/Iter";

import Agent "Agent";
import Types "Types";
import CanisterImplementation "CanisterImplementation";

module {
    type BitcoinAgentState = Types.BitcoinAgentState;

    /// Returns the Bitcoin agent state.
    public func get_state(bitcoin_agent : Agent.BitcoinAgent) : BitcoinAgentState {
        {
            network = bitcoin_agent.management_canister.network;
            main_address_type = bitcoin_agent.main_address_type;
            ecdsa_pub_key_addresses = Iter.toArray(bitcoin_agent.ecdsa_pub_key_addresses.entries());
            utxos_state_addresses = Iter.toArray(bitcoin_agent.utxos_state_addresses.entries());
            min_confirmations = bitcoin_agent.min_confirmations;
        }
    };

    /// Returns the associated Bitcoin agent with the given `bitcoin_agent_state`.
    public func from_state(
        bitcoin_agent_state : BitcoinAgentState,
    ) : Agent.BitcoinAgent {
        let management_canister = CanisterImplementation.ManagementCanisterImpl(bitcoin_agent_state.network);
        let bitcoin_agent = Agent.BitcoinAgent(management_canister, bitcoin_agent_state.main_address_type, bitcoin_agent_state.min_confirmations);
        // The instance only contains the main address after creation.
        // So, this loop is used to create an empty Bitcoin agent.
        for (address in bitcoin_agent.ecdsa_pub_key_addresses.keys()) {
            bitcoin_agent.ecdsa_pub_key_addresses.delete(address);
            bitcoin_agent.utxos_state_addresses.delete(address);
        };
        for ((address, ecdsa_public_key) in bitcoin_agent_state.ecdsa_pub_key_addresses.vals()) {
            bitcoin_agent.ecdsa_pub_key_addresses.put(address, ecdsa_public_key);
        };
        for ((address, utxos_state) in bitcoin_agent_state.utxos_state_addresses.vals()) {
            bitcoin_agent.utxos_state_addresses.put(address, utxos_state);
        };
        bitcoin_agent
        // TODO(ER-2726): Add guards for Bitcoin concurrent access.
    };
}