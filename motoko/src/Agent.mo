import Result "mo:base/Result";
import Map "mo:base/HashMap";
import Text "mo:base/Text";
import Buffer "mo:base/Buffer";
import Nat8 "mo:base/Nat8";
import Error "mo:base/Error";

import Types "Types";
import CanisterCommon "CanisterCommon";
import AddressManagement "AddressManagement";
import UtxoManagement "UtxoManagement";
import Utils "Utils";

module {
    type ErrorCode = Error.ErrorCode;
    type GetUtxosError = Types.GetUtxosError;
    type Utxo = Types.Utxo;
    type AddressType = Types.AddressType;
    type Address = Types.Address;
    type EcdsaPubKey = Types.EcdsaPubKey;
    type UtxosState = Types.UtxosState;
    type UtxosUpdate = Types.UtxosUpdate;
    type TrackingAddressError = Types.TrackingAddressError;
    type Satoshi = Types.Satoshi;
    type BalanceUpdate = Types.BalanceUpdate;
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    type AddAddressError = Types.AddAddressError;
    type MillisatoshiPerByte = Types.MillisatoshiPerByte;
    type GetCurrentFeesError = Types.GetCurrentFeesError;
    type GetCurrentFeeError = Types.GetCurrentFeeError;
    type FeeRequest = Types.FeeRequest;

    /// Creates a new Bitcoin agent using the given management canister.
    public class BitcoinAgent(_management_canister : CanisterCommon.ManagementCanister, _main_address_type : AddressType, _min_confirmations : Nat32) = self {
        public let management_canister = _management_canister;
        public let main_address_type = _main_address_type;
        public let min_confirmations = _min_confirmations;

        public let ecdsa_pub_key_addresses = Map.HashMap<Address, EcdsaPubKey>(1, Text.equal, Text.hash);
        public let utxos_state_addresses = Map.HashMap<Address, UtxosState>(1, Text.equal, Text.hash);
        
        /// Returns the main Bitcoin address of the canister.
        public func get_main_address() : Address {
            AddressManagement.get_main_address(management_canister.network, main_address_type)
        };

        let main_address = get_main_address();
        ecdsa_pub_key_addresses.put(main_address, AddressManagement.get_btc_ecdsa_public_key());
        utxos_state_addresses.put(main_address, Types.utxos_state_new(min_confirmations));

        /// Adds an address based on the provided derivation path and address type to the list of managed addresses.
        /// A minimum number of confirmations must further be specified, which is used when calling `get_utxos` and `get_balance`.
        /// Returns the derived address if the operation is successful and an error otherwise.
        public func add_address_with_parameters(
            derivation_path : [Nat8],
            address_type : AddressType,
            min_confirmations : Nat32,
        ) : Result<Address, AddAddressError> {
            AddressManagement.add_address_with_parameters(
                self,
                management_canister.network,
                derivation_path,
                address_type,
                min_confirmations,
            )
        };

        /// Adds an address to the agent with the provided derivation path.
        /// The default address type and default number of confirmations are used.
        public func add_address(
            derivation_path : [Nat8],
        ) : Result<Address, AddAddressError> {
            add_address_with_parameters(derivation_path, main_address_type, min_confirmations)
        };

        /// Removes the given address from given BitcoinAgent managed addresses.
        /// The address is removed if it is already managed and if it is different from the main address.
        /// Returns true if the removal was successful, false otherwise.
        public func remove_address(address : Address) : Bool {
            AddressManagement.remove_address(self, address)
        };

        /// Returns the managed addresses according to given BitcoinAgent.
        public func list_addresses() : [Address] {
            AddressManagement.list_addresses(self)
        };

        /// Returns the UTXOs of the given Bitcoin `address` according to `min_confirmations`.
        public func get_utxos(address : Address, min_confirmations : Nat32, utxos : Buffer.Buffer<Utxo>) : async (?GetUtxosError) {
            await management_canister.get_utxos(address, min_confirmations, utxos)
        };

        /// Returns the difference between the current UTXO state and the last seen state for this address.
        /// The last seen state for an address is updated to the current state by calling `update_state` or implicitly when invoking `get_utxos_update`.
        /// If there are no changes to the UTXO set since the last call, the returned `UtxosUpdate` will be identical.
        public func peek_utxos_update(
            address : Address,
        ) : async Result<UtxosUpdate, TrackingAddressError> {
            await UtxoManagement.peek_utxos_update(self, address)
        };

        /// Updates the state of the `BitcoinAgent` for the given `address`.
        /// This function doesn't invoke a Bitcoin integration API function.
        public func update_state(address : Address) : Result<(), TrackingAddressError> {
            UtxoManagement.update_state(self, address)
        };

        /// Returns the difference in the set of UTXOs of an address controlled by the `BitcoinAgent` between the current state and the seen state when the function was last called, considering only UTXOs with the number of confirmations specified when adding the given address.
        /// The returned `UtxosUpdate` contains the information which UTXOs were added and removed. If the function is called for the first time, the current set of UTXOs is returned.
        /// Note that the function changes the state of the `BitcoinAgent`: A subsequent call will return changes to the UTXO set that have occurred since the last call.
        public func get_utxos_update(
            address : Address,
        ) : async Result<UtxosUpdate, TrackingAddressError> {
            await UtxoManagement.get_utxos_update(self, address)
        };

        /// Returns the balance of the given Bitcoin `address` according to `min_confirmations`.
        public func get_balance(address : Address, min_confirmations : Nat32) : async Result<Satoshi, GetUtxosError> {
            await UtxoManagement.get_balance(self, address, min_confirmations)
        };

        /// Returns the difference between the current balance state and the last seen state for this address.
        /// The last seen state for an address is updated to the current unseen state by calling `update_state` or implicitly when invoking `get_balance_update`.
        /// If there are no changes to the balance since the last call, the returned `BalanceUpdate` will be identical.
        public func peek_balance_update(
            address : Address,
        ) : async Result<BalanceUpdate, TrackingAddressError> {
            await UtxoManagement.peek_balance_update(self, address)
        };

        /// Returns the difference in the balance of an address controlled by the `BitcoinAgent` between the current state and the seen state when the function was last called, considering only transactions with the specified number of confirmations.
        /// The returned `BalanceUpdate` contains the information on how much balance was added and subtracted in total. If the function is called for the first time, the current balance of the address is returned.
        /// It is equivalent to calling `get_utxos_update` and summing up the balances in the returned UTXOs.
        public func get_balance_update(
            address : Address,
        ) : async Result<BalanceUpdate, TrackingAddressError> {
            await UtxoManagement.get_balance_update(self, address)
        };

        /// Returns fees as percentiles in millisatoshis/byte over the last 10,000 transactions.
        public func get_current_fees() : async Result<[MillisatoshiPerByte], GetCurrentFeesError> {
            await management_canister.get_current_fees()
        };

        /// Returns the percentile associated with the given `FeeRequest`.
        func evaluate_fee_request(fee_request : FeeRequest) : Result<Nat, GetCurrentFeeError> {
            let percentile = switch (fee_request) {
                case (#Slow) {
                    25
                };
                case (#Standard) {
                    50
                };
                case (#Fast) {
                    75
                };
                case (#Percentile(percentile)) {
                    Nat8.toNat(percentile)
                };
            };
            if(percentile >= 99) {
                return #err (#InvalidPercentile);
            };
            #ok percentile
        };

        /// Returns the fee as a percentile in millisatoshis/byte over the last 10,000 transactions.
        public func get_current_fee(fee_request : FeeRequest) : async Result<MillisatoshiPerByte, GetCurrentFeeError> {
            switch (evaluate_fee_request(fee_request)) {
                case (#ok percentile) {
                    switch (await get_current_fees()) {
                        case (#ok fees) {
                            // A given percentile between 0 and 99 is invalid if the management canister doesn't have enough transactions to compute current fees.
                            if(percentile > fees.size()) {
                                return #err (#InvalidPercentile);
                            };
                            #ok (fees[percentile])
                        };
                        case (#err (#ManagementCanisterReject(code : ErrorCode, message : Text))) {
                            #err (#ManagementCanisterReject(code, message))
                        };
                    }
                };
                case _ {
                    #err (#InvalidPercentile);
                };
            }
        }
    }
}
