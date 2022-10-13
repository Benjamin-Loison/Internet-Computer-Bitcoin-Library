import Agent "../src/Agent";
import Types "../src/Types";
import UpgradeManagement "../src/UpgradeManagement";

import TestCommon "TestCommon";

type BitcoinAgentState = Types.BitcoinAgentState;
type BitcoinAgent = Agent.BitcoinAgent;

do {
    /// Check that `get_state` and `from_state` return the Bitcoin agent state and the Bitcoin agent associated with the former Bitcoin agent state, respectively.
    // Every field of the `BitcoinAgentState` is filled with non-default value during the `BitcoinAgent` instantiation.
    let pre_upgrade_bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);

    let pre_upgrade_state : BitcoinAgentState = UpgradeManagement.get_state(pre_upgrade_bitcoin_agent);
    let post_upgrade_bitcoin_agent : BitcoinAgent =
        UpgradeManagement.from_state(pre_upgrade_state);

    assert UpgradeManagement.get_state(post_upgrade_bitcoin_agent) == pre_upgrade_state;
};