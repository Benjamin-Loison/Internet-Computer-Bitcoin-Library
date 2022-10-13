import Result "mo:base/Result";
import Prelude "mo:base/Prelude";
import Buffer "mo:base/Buffer";

import Types "Types";

module {
    type Utxo = Types.Utxo;
    type GetUtxosError = Types.GetUtxosError;
    type Address = Types.Address;
    type Network = Types.Network;
    type Cycles = Types.Cycles;
    type MillisatoshiPerByte = Types.MillisatoshiPerByte;
    type GetCurrentFeesError = Types.GetCurrentFeesError;
    type Result<Ok, Err> = Result.Result<Ok, Err>;

    // The fees for the various Bitcoin endpoints.
    public let GET_UTXOS_COST_CYCLES : Cycles = 100_000_000;
    public let GET_CURRENT_FEE_PERCENTILES_COST_CYCLES : Cycles = 100_000_000;

    /// Creates a new instance of the management canister.
    public class ManagementCanister(network : Network) {
        public let network : Network = Prelude.unreachable();

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