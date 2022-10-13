import Result "mo:base/Result";
import Option "mo:base/Option";
import Prelude "mo:base/Prelude";
import ExperimentalCycles "mo:base/ExperimentalCycles";
import Error "mo:base/Error";
import Buffer "mo:base/Buffer";
import Array "mo:base/Array";

import Types "Types";
import Agent "AgentPrototype";
import Utils "Utils";
import CanisterCommon "CanisterCommon";

module {
    type Address = Types.Address;
    type Utxo = Types.Utxo;
    type GetUtxosError = Types.GetUtxosError;
    type GetUtxosResponse = Types.GetUtxosResponse;
    type ManagementCanisterActor = Types.ManagementCanisterActor;
    type UtxosUpdate = Types.UtxosUpdate;
    type TrackingAddressError = Types.TrackingAddressError;
    type Satoshi = Types.Satoshi;
    type Network = Types.Network;
    type BalanceUpdate = Types.BalanceUpdate;
    type UtxosFilter = Types.UtxosFilter;
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    let GET_UTXOS_COST_CYCLES = CanisterCommon.GET_UTXOS_COST_CYCLES;
    let MIN_CONFIRMATIONS_UPPER_BOUND = Types.MIN_CONFIRMATIONS_UPPER_BOUND;

    /// Returns the actual UTXOs of the given Bitcoin `address` according to `min_confirmations`.
    public func get_utxos(management_canister_actor : ManagementCanisterActor, network : Network, address : Address, min_confirmations : Nat32, utxos : Buffer.Buffer<Utxo>) : async (?GetUtxosError) {
        if(min_confirmations > MIN_CONFIRMATIONS_UPPER_BOUND) {
            return ?(#MinConfirmationsTooHigh);
        };
        var filter : ?UtxosFilter = ?(#MinConfirmations(min_confirmations));
        label pages loop {
            ExperimentalCycles.add(GET_UTXOS_COST_CYCLES);
            try {
                let get_utxos_response = await management_canister_actor.bitcoin_get_utxos({
                    network = network;
                    filter = filter;
                    address = address;
                });
                utxos.append(Utils.buffer_from_array(get_utxos_response.utxos));
                switch (get_utxos_response.next_page) {
                    case (?next_page) {
                        filter := ?(#Page next_page);
                    };
                    case null {
                        break pages;
                    };
                };
            } catch (e) {
                return ?(#ManagementCanisterReject (Error.code(e), Error.message(e)));
            };
        };
        null
        // Comment above code from `get_utxos` and uncomment the line below to make tests compilable.
        //Prelude.unreachable();
    };

    /// Returns the difference between the current UTXO state and the last seen state for this address.
    /// The last seen state for an address is updated to the current unseen state by calling `update_state` or implicitly when invoking `get_utxos_update`.
    /// If there are no changes to the UTXO set since the last call, the returned `UtxosUpdate` will be identical.
    public func peek_utxos_update(
        bitcoin_agent : Agent.BitcoinAgent,
        address : Address,
    ) : async Result<UtxosUpdate, TrackingAddressError> {
        if (Option.isNull(bitcoin_agent.utxos_state_addresses.get(address))) {
            return #err(#AddressNotTracked);
        };
        let min_confirmations = Utils.unwrap(bitcoin_agent.utxos_state_addresses.get(address)).min_confirmations;
        let utxos = Buffer.Buffer<Utxo>(0);
        let result = await bitcoin_agent.get_utxos(address, min_confirmations, utxos);
        switch result {
            case null {
                let current_utxos = utxos.toArray();
                let utxos_state_address = Utils.unwrap(bitcoin_agent.utxos_state_addresses.get(address));
                bitcoin_agent.utxos_state_addresses.put(address, {
                    seen_state = utxos_state_address.seen_state;
                    unseen_state = current_utxos;
                    min_confirmations = utxos_state_address.min_confirmations;
                });
                #ok (Types.utxos_update_from_state(
                    utxos_state_address.seen_state,
                    current_utxos,
                ))
            };
            case (?(#ManagementCanisterReject(error_code, error_message))) {
                #err (#ManagementCanisterReject(error_code, error_message))
            };
            // `#MinConfirmationsTooHigh` can't be raised see `add_address_with_parameters`.
            case (?(#MinConfirmationsTooHigh)) {
                Prelude.unreachable();
            };
        }
    };

    /// Updates the state of the `BitcoinAgent` for the given `address`.
    /// This function doesn't invoke a Bitcoin integration API function.
    public func update_state(
        bitcoin_agent : Agent.BitcoinAgent,
        address : Address,
    ) : Result<(), TrackingAddressError> {
        if (Option.isNull(bitcoin_agent.utxos_state_addresses.get(address))) {
            return #err(#AddressNotTracked);
        };
        let utxos_state_address = Utils.unwrap(bitcoin_agent.utxos_state_addresses.get(address));
        let unseen_state = utxos_state_address.unseen_state;
        bitcoin_agent
            .utxos_state_addresses
            .put(
                address,
                {
                    seen_state = unseen_state;
                    unseen_state = unseen_state;
                    min_confirmations = utxos_state_address.min_confirmations;
                }
            );
        #ok
    };

    /// Returns the difference in the set of UTXOs of an address controlled by the `BitcoinAgent` between the current state and the seen state when the function was last called, considering only UTXOs with the number of confirmations specified when adding the given address.
    /// The returned `UtxosUpdate` contains the information which UTXOs were added and removed. If the function is called for the first time, the current set of UTXOs is returned.
    /// Note that the function changes the state of the `BitcoinAgent`: A subsequent call will return changes to the UTXO set that have occurred since the last call.
    public func get_utxos_update(
        bitcoin_agent : Agent.BitcoinAgent,
        address : Address,
    ) : async Result<UtxosUpdate, TrackingAddressError> {
        let peeked_utxos_update = await peek_utxos_update(bitcoin_agent, address);
        switch peeked_utxos_update {
            case (#ok utxos_update) {
                ignore(update_state(bitcoin_agent, address));
                #ok utxos_update
            };
            case (#err err) {
                #err err
            };
        }
    };

    /// Returns the balance of the given Bitcoin `address` according to `min_confirmations` from a given set of UTXOs.
    public func get_balance(bitcoin_agent : Agent.BitcoinAgent, address : Address, min_confirmations : Nat32) : async Result<Satoshi, GetUtxosError> {
        let utxos = Buffer.Buffer<Utxo>(0);
        let result = await bitcoin_agent.get_utxos(address, min_confirmations, utxos);
        switch result {
            case null {
                #ok(Types.get_balance_from_utxos(utxos.toArray()))
            };
            case (?err) {
                #err err
            };
        }
    };

    /// Returns the difference between the current balance state and the last seen state for this address.
    /// The last seen state for an address is updated to the current unseen state by calling `update_state` or implicitly when invoking `get_balance_update`.
    /// If there are no changes to the balance since the last call, the returned `BalanceUpdate` will be identical.
    public func peek_balance_update(
        bitcoin_agent : Agent.BitcoinAgent,
        address : Address,
    ) : async Result<BalanceUpdate, TrackingAddressError> {
        let peeked_utxos_update = await peek_utxos_update(bitcoin_agent, address);
        switch peeked_utxos_update {
            case (#ok utxos_update) {
                #ok(Types.balance_update_from(utxos_update))
            };
            case (#err err) {
                #err err
            };
        }
    };

    /// Returns the difference in the balance of an address controlled by the `BitcoinAgent` between the current state and the seen state when the function was last called, considering only transactions with the specified number of confirmations.
    /// The returned `BalanceUpdate` contains the information on how much balance was added and subtracted in total. If the function is called for the first time, the current balance of the address is returned.
    /// It is equivalent to calling `get_utxos_update` and summing up the balances in the returned UTXOs.
    public func get_balance_update(
        bitcoin_agent : Agent.BitcoinAgent,
        address : Address,
    ) : async Result<BalanceUpdate, TrackingAddressError> {
        let got_utxos_update = await get_utxos_update(bitcoin_agent, address);
        switch got_utxos_update {
            case (#ok utxos_update) {
                #ok(Types.balance_update_from(utxos_update))
            };
            case (#err err) {
                #err err
            };
        }
    };
}
