import Array "mo:base/Array";
import Result "mo:base/Result";
import Buffer "mo:base/Buffer";
import Int32 "mo:base/Int32";

import Types "../src/Types";
import Utils "../src/Utils";

import TestUtils "Utils";

module {
    type Address = Types.Address;
    type Utxo = Types.Utxo;
    type GetUtxosError = Types.GetUtxosError;
    type Network = Types.Network;
    type MillisatoshiPerByte = Types.MillisatoshiPerByte;
    type GetCurrentFeesError = Types.GetCurrentFeesError;
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    let MIN_CONFIRMATIONS_UPPER_BOUND = Types.MIN_CONFIRMATIONS_UPPER_BOUND;

    /// Creates a new instance of the management canister mock.
    public class ManagementCanisterMock(_network : Network) = self {
        public let network = _network;

        public var tip_height : Int32 = Int32.fromNat32(MIN_CONFIRMATIONS_UPPER_BOUND);
        public let utxos : Buffer.Buffer<Utxo> = Utils.buffer_from_array(TestUtils.get_init_utxos());

        /// Returns the mock UTXOs of the canister address according to `min_confirmations`.
        /// Note: `address` is ignored for simplicity purpose.
        public func get_utxos(address : Address, min_confirmations : Nat32, utxos_buffer : Buffer.Buffer<Utxo>) : (?GetUtxosError) {
            let threshold_height = tip_height + 1 - Int32.fromNat32(min_confirmations);
            for (utxo in utxos.vals()) {
                if (Int32.fromNat32(utxo.height) <= threshold_height) {
                    utxos_buffer.add(utxo)
                };
            };
            null
        };

        /// Returns fees as percentiles in millisatoshis/byte over the last 10,000 transactions.
        public func get_current_fees() : Result<[MillisatoshiPerByte], GetCurrentFeesError> {
            TestUtils.get_current_fees()
        };
    };
}