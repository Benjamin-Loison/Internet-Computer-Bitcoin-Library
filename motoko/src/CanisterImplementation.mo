import Result "mo:base/Result";
import Buffer "mo:base/Buffer";

import Types "Types";
import UtxoManagement "UtxoManagement";
import TransactionManagement "TransactionManagement";

module {
    type Utxo = Types.Utxo;
    type GetUtxosError = Types.GetUtxosError;
    type Address = Types.Address;
    type Network = Types.Network;
    type ManagementCanisterActor = Types.ManagementCanisterActor;
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    let MANAGEMENT_CANISTER_ID = Types.MANAGEMENT_CANISTER_ID;
    type MillisatoshiPerByte = Types.MillisatoshiPerByte;
    type GetCurrentFeesError = Types.GetCurrentFeesError;

    /// Creates a new instance of the real management canister.
    public class ManagementCanisterImpl(_network : Network) {
        public let network = _network;

        /// Used to interact with the management canister.
        let management_canister_actor : ManagementCanisterActor = actor(MANAGEMENT_CANISTER_ID);

        /// Returns the UTXOs of the given Bitcoin `address` according to `min_confirmations`.
        /// This getter always return the same value until a block, with transactions concerning the address, is mined.
        public func get_utxos(address : Address, min_confirmations : Nat32, utxos : Buffer.Buffer<Utxo>) : async (?GetUtxosError) {
            await UtxoManagement.get_utxos(management_canister_actor, network, address, min_confirmations, utxos)
        };

        /// Returns fees as percentiles in millisatoshis/byte over the last 10,000 transactions.
        public func get_current_fees() : async Result<[MillisatoshiPerByte], GetCurrentFeesError> {
            await TransactionManagement.get_current_fees(management_canister_actor, network)
        };
    }
}