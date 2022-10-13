import Iter "mo:base/Iter";
import Blob "mo:base/Blob";
import Array "mo:base/Array";
import Buffer "mo:base/Buffer";

import Agent "../src/Agent";
import Types "../src/Types";

import CanisterMock "CanisterMock";
import TestCommon "TestCommon";
import TestUtils "Utils";

type OutPoint = Types.OutPoint;
type Satoshi = Types.Satoshi;
type Address = Types.Address;
type Utxo = Types.Utxo;

let MIN_CONFIRMATIONS_UPPER_BOUND = Types.MIN_CONFIRMATIONS_UPPER_BOUND;

func assert_utxos(bitcoin_agent : Agent.BitcoinAgent, canister_bitcoin_address : Address, min_confirmations : Nat32, expected_utxos : [Utxo]) {
    let actual_utxos = Buffer.Buffer<Utxo>(0);
    assert bitcoin_agent.get_utxos(canister_bitcoin_address, min_confirmations, actual_utxos) == null;
    assert (actual_utxos.toArray()) == expected_utxos;
};

do {
    /// Check that `get_utxos` returns the correct address' UTXOs according to `min_confirmations`.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);
    let utxos = TestUtils.get_init_utxos();
    let canister_bitcoin_address = bitcoin_agent.get_main_address();

    assert_utxos(bitcoin_agent, canister_bitcoin_address, 0, utxos);
    assert_utxos(bitcoin_agent, canister_bitcoin_address, 1, utxos);
    assert_utxos(bitcoin_agent, canister_bitcoin_address, 2, []);
};

do{
    /// Check that `peek_utxos_update` returns the correct `UtxosUpdate` associated with the Bitcoin agent's main address.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);
    let utxos_update = TestUtils.get_init_utxos_update();
    let canister_bitcoin_address = bitcoin_agent.get_main_address();

    for (_ in Iter.range(0, 1)) {
        assert await bitcoin_agent.peek_utxos_update(canister_bitcoin_address) ==
            #ok utxos_update;
    };
};

do {
    /// Check that `update_state` updates the Bitcoin agent's state according to its main address.
    let management_canister = CanisterMock.ManagementCanisterMock(#Regtest);
    let bitcoin_agent = Agent.BitcoinAgent(management_canister, #P2pkh, 0);

    let utxos_update = TestUtils.get_init_utxos_update();
    let canister_bitcoin_address = bitcoin_agent.get_main_address();

    assert await bitcoin_agent.peek_utxos_update(canister_bitcoin_address) ==
        #ok utxos_update;

    let added_utxo = {
        outpoint : OutPoint = {
            txid = Blob.fromArray(Array.freeze(Array.init<Nat8>(32, 0)));
            vout = 0;
        };
        value : Satoshi = 250_000;
        height : Nat32 = MIN_CONFIRMATIONS_UPPER_BOUND + 1;
    };
    
    management_canister
        .utxos
        .add(added_utxo);
    management_canister.tip_height += 1;

    assert bitcoin_agent.update_state(canister_bitcoin_address) == #ok (());

    let new_utxos_update = {
        added_utxos = [added_utxo];
        removed_utxos = [];
    };

    assert await bitcoin_agent.peek_utxos_update(canister_bitcoin_address) == #ok new_utxos_update;

    assert bitcoin_agent.update_state(canister_bitcoin_address) == #ok (());

    assert await bitcoin_agent.peek_utxos_update(canister_bitcoin_address) == #ok (Types.utxos_update_new());
};

do {
    /// Check that `get_utxos_update` returns the correct `UtxosUpdate` associated with the Bitcoin agent main address.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);
    let utxos_update = TestUtils.get_init_utxos_update();
    let canister_bitcoin_address = bitcoin_agent.get_main_address();

    assert await bitcoin_agent.get_utxos_update(canister_bitcoin_address) == #ok utxos_update;

    assert await bitcoin_agent.get_utxos_update(canister_bitcoin_address) == #ok (Types.utxos_update_new());
};

do {
    /// Check that `get_balance` returns the correct address' balance according to `min_confirmations`.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);
    let utxos = TestUtils.get_init_utxos();
    let balance = Types.get_balance_from_utxos(utxos);
    let canister_bitcoin_address = bitcoin_agent.get_main_address();

    assert await bitcoin_agent.get_balance(canister_bitcoin_address, 0) == #ok balance;
    assert await bitcoin_agent.get_balance(canister_bitcoin_address, 1) == #ok balance;
    assert await bitcoin_agent.get_balance(canister_bitcoin_address, 2) == #ok 0;
};

do {
    /// Check that `peek_balance_update` returns the correct `BalanceUpdate` associated with the Bitcoin agent's main address.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);
    let balance_update = TestUtils.get_init_balance_update();
    let canister_bitcoin_address = bitcoin_agent.get_main_address();

    for (_ in Iter.range(0, 1)) {
        assert bitcoin_agent.peek_balance_update(canister_bitcoin_address) == #ok balance_update;
    };
};

do {
    /// Check that `get_balance_update` returns the correct `BalanceUpdate` associated with the Bitcoin agent main address.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);
    let balance_update = TestUtils.get_init_balance_update();
    let canister_bitcoin_address = bitcoin_agent.get_main_address();

    assert await bitcoin_agent.get_balance_update(canister_bitcoin_address) == #ok balance_update;

    assert await bitcoin_agent.get_balance_update(canister_bitcoin_address) == #ok (Types.balance_update_new());
};