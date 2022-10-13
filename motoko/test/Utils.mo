import Array "mo:base/Array";
import Blob "mo:base/Blob";
import Text "mo:base/Text";
import Result "mo:base/Result";
import Nat64 "mo:base/Nat64";

import Types "../src/Types";

module {
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    type Utxo = Types.Utxo;
    type Address = Types.Address;
    type AddressType = Types.AddressType;
    type UtxosUpdate = Types.UtxosUpdate;
    type BalanceUpdate = Types.BalanceUpdate;
    type MillisatoshiPerByte = Types.MillisatoshiPerByte;
    type GetCurrentFeesError = Types.GetCurrentFeesError;
    let MIN_CONFIRMATIONS_UPPER_BOUND = Types.MIN_CONFIRMATIONS_UPPER_BOUND;

    // Get some hard-coded UTXOs to be used by the mock.
    public func get_init_utxos() : [Utxo] {
        [{
            outpoint = {
                txid = Blob.fromArray(Array.freeze(Array.init<Nat8>(32, 0)));
                vout = 0;
            };
            value = 250_000;
            height = MIN_CONFIRMATIONS_UPPER_BOUND;
        }]
    };

    /// Get the initial UTXOs update to be used by the mock.
    public func get_init_utxos_update() : UtxosUpdate {
        {
            added_utxos = get_init_utxos();
            removed_utxos = [];
        }
    };

    /// Get the initial balance update to be used by the mock.
    public func get_init_balance_update() : BalanceUpdate {
        Types.balance_update_from(get_init_utxos_update())
    };

    let PUBKEY_ADDRESS_PREFIX_MAIN : Char = '0';

    /// Returns the `AddressType`, if any, for a given `Address`.
    public func get_address_type(address : Address) : ?AddressType {
        switch (Text.toIter(address).next()) {
            case (?PUBKEY_ADDRESS_PREFIX_MAIN) ?#P2pkh;
            case _ null
        }
    };

    // Get some hard-coded fees to be used by the mock.
    public func get_current_fees() : Result<[MillisatoshiPerByte], GetCurrentFeesError> {
        let current_fees = Array.tabulate<MillisatoshiPerByte>(99, func(i) : MillisatoshiPerByte { Nat64.fromNat((i + 1) * 1_000) } );
        #ok current_fees
    };
};