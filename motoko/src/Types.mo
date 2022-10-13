import Array "mo:base/Array";
import Option "mo:base/Option";
import Error "mo:base/Error";

import Curves "../motoko-bitcoin/src/ec/Curves";

/// Types used to support the candid API.
module Types {
    /// Actor definition to handle interactions with the management canister.
    public type ManagementCanisterActor = actor {
        /// Retrieves the UTXOs from the management canister.
        bitcoin_get_utxos : GetUtxosRequest -> async GetUtxosResponse;
        /// Retrieves the current fees from the management canister.
        bitcoin_get_current_fee_percentiles : GetCurrentFeePercentilesRequest -> async [MillisatoshiPerByte];
    };

    public let MANAGEMENT_CANISTER_ID = "aaaaa-aa";

    public type Cycles = Nat;
    public type Satoshi = Nat64;
    public type MillisatoshiPerByte = Nat64;
    public type Address = Text;
    public type BlockHash = [Nat8];
    public type Page = [Nat8];
    public type ErrorCode = Error.ErrorCode;

    /// The type of Bitcoin network the dapp will be interacting with.
    public type Network = {
        #Mainnet;
        #Regtest;
        #Testnet;
    };

    /// A reference to a transaction output.
    public type OutPoint = {
        txid : Blob;
        vout : Nat32;
    };

    /// An unspent transaction output.
    public type Utxo = {
        outpoint : OutPoint;
        value : Satoshi;
        height : Nat32;
    };

    /// A filter used when requesting UTXOs.
    public type UtxosFilter = {
        #MinConfirmations : Nat32;
        #Page : Page;
    };

    /// A request for getting the UTXOs for a given address.
    public type GetUtxosRequest = {
        address : Address;
        network : Network;
        filter : ?UtxosFilter;
    };

    /// The response returned for a request to get the UTXOs of a given address.
    public type GetUtxosResponse = {
        utxos : [Utxo];
        tip_block_hash : BlockHash;
        tip_height : Nat32;
        next_page : ?Page;
    };

    /// Errors when processing a `get_utxos` request.
    public type GetUtxosError = {
        #MinConfirmationsTooHigh;
        #ManagementCanisterReject : (ErrorCode, Text);
    };

    public type SendTransactionRequest = {
        transaction : Blob;
    };

    public type SendTransactionResponse = {
        #Ok;
        #Err : ?SendTransactionError;
    };

    public type SendTransactionError = {
        #MalformedTransaction;
    };

    /// ECDSA public key and chain code.
    public type EcdsaPubKey = {
        public_key : [Nat8];
        chain_code : [Nat8];
        derivation_path : [[Nat8]];
    };

    public let CURVE = Curves.secp256k1;

    /// Address types supported by the Bitcoin library.
    public type AddressType = {
        #P2pkh;
    };

    /// Error when processing an `add_address` request.
    public type AddAddressError = {
        #DerivationPathTooLong;
    };

    /// Contains the information which UTXOs were added and removed since a given moment.
    public type UtxosUpdate = {
        added_utxos : [Utxo];
        removed_utxos : [Utxo];
    };

    public func utxos_update_new() : UtxosUpdate {
        {
            added_utxos = [];
            removed_utxos = [];
        }
    };

    /// Returns `state_0`'s UTXOs that aren't in `state_1`.
    func state_difference(state_0: [Utxo], state_1: [Utxo]) : [Utxo] {
        Array.filter(state_0, func(utxo_0 : Utxo) : Bool {
            Option.isNull(Array.find(state_1, func(utxo_1 : Utxo) : Bool {
                utxo_0 == utxo_1
            }))
        })
    };

    /// Returns an `UtxosUpdate` defined by the changes in the UTXOs set between `seen_state` and `unseen_state`.
    public func utxos_update_from_state(seen_state: [Utxo], unseen_state: [Utxo]) : UtxosUpdate {
        {
            added_utxos = state_difference(unseen_state, seen_state);
            removed_utxos = state_difference(seen_state, unseen_state);
        }
    };

    /// Represents the last seen state and the unseen state UTXOs for a given `min_confirmations`.
    public type UtxosState = {
        seen_state : [Utxo];
        unseen_state : [Utxo];
        min_confirmations : Nat32;
    };

    public func utxos_state_new(min_confirmations : Nat32) : UtxosState {
        {
            seen_state = [];
            unseen_state = [];
            min_confirmations = min_confirmations;
        }
    };

    public type TrackingAddressError = {
        #AddressNotTracked;
        #ManagementCanisterReject : (ErrorCode, Text);
    };

    /// Represents the last seen state and the unseen state balances for a given `min_confirmations`.
    public type BalanceUpdate = {
        added_balance : Satoshi;
        removed_balance : Satoshi;
    };

    public func balance_update_new() : BalanceUpdate {
        {
            added_balance = 0;
            removed_balance = 0;
        }
    };

    /// Returns the total value of a UTXOs set.
    public func get_balance_from_utxos(utxos : [Utxo]) : Satoshi {
        var total_value : Satoshi = 0;
        for (utxo in utxos.vals()) {
            total_value += utxo.value;
        };
        total_value
    };

    public func balance_update_from(utxos_update : UtxosUpdate) : BalanceUpdate {
        {
            added_balance = get_balance_from_utxos(utxos_update.added_utxos);
            removed_balance = get_balance_from_utxos(utxos_update.removed_utxos);
        }
    };

    /// Represents the Bitcoin agent state used for canister upgrades.
    public type BitcoinAgentState = {
        network : Network;
        main_address_type : AddressType;
        ecdsa_pub_key_addresses : [(Address, EcdsaPubKey)];
        utxos_state_addresses : [(Address, UtxosState)];
        min_confirmations : Nat32;
    };

    /// The upper bound on the minimum number of confirmations supported by the Bitcoin integration.
    public let MIN_CONFIRMATIONS_UPPER_BOUND : Nat32 = 6;

    /// A request for getting the current fee percentiles.
    public type GetCurrentFeePercentilesRequest = {
        network : Network;
    };

    /// Error when processing a `get_current_fees` request.
    public type GetCurrentFeesError = {
        #ManagementCanisterReject : (ErrorCode, Text);
    };

    /// Errors when processing a `get_current_fee` request.
    public type GetCurrentFeeError = {
        #InvalidPercentile;
        #ManagementCanisterReject : (ErrorCode, Text);
    };

    /// Represents the fee request as a percentile in millisatoshis/byte over the last 10,000 transactions.
    public type FeeRequest = {
        #Slow;                // 25th percentile
        #Standard;            // 50th percentile
        #Fast;                // 75th percentile
        #Percentile : (Nat8); // custom percentile
    };
}