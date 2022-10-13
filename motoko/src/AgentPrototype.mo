import Result "mo:base/Result";
import Prelude "mo:base/Prelude";
import Map "mo:base/HashMap";
import Buffer "mo:base/Buffer";

import Types "Types";
import CanisterCommon "CanisterCommon";

module {
    type GetUtxosError = Types.GetUtxosError;
    type Utxo = Types.Utxo;
    type Address = Types.Address;
    type AddressType = Types.AddressType;
    type EcdsaPubKey = Types.EcdsaPubKey;
    type UtxosState = Types.UtxosState;
    type MillisatoshiPerByte = Types.MillisatoshiPerByte;
    type GetCurrentFeesError = Types.GetCurrentFeesError;
    type Result<Ok, Err> = Result.Result<Ok, Err>;

    /// Creates a new Bitcoin agent.
    public class BitcoinAgent() {
        public let management_canister : CanisterCommon.ManagementCanister = Prelude.unreachable();
        public let main_address_type : AddressType = Prelude.unreachable();
        public let min_confirmations : Nat32 = Prelude.unreachable();

        public let ecdsa_pub_key_addresses : Map.HashMap<Address, EcdsaPubKey> = Prelude.unreachable();
        public let utxos_state_addresses : Map.HashMap<Address, UtxosState> = Prelude.unreachable();

        /// Returns the main Bitcoin address of the canister.
        public func get_main_address() : Address {
            Prelude.unreachable()
        };

        /// Returns the UTXOs of the given Bitcoin `address` according to `min_confirmations`.
        public func get_utxos(address : Address, min_confirmations : Nat32, utxos : Buffer.Buffer<Utxo>) : async (?GetUtxosError) {
            Prelude.unreachable()
        };

        /// Returns fees as percentiles in millisatoshis/byte over the last 10,000 transactions.
        public func get_current_fees() : async Result<[MillisatoshiPerByte], GetCurrentFeesError> {
            Prelude.unreachable()
        };
    }
}
