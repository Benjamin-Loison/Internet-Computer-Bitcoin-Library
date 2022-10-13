import Result "mo:base/Result";

import TestCommon "TestCommon";
import TestUtils "Utils";

type Result<Ok, Err> = Result.Result<Ok, Err>;

do {
    /// Check that `get_current_fees` returns the correct fees.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);
    let fees = TestUtils.get_current_fees();

    assert bitcoin_agent.get_current_fees() == fees;
};

do {
    /// Check that `get_current_fee` returns the correct fee.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);

    assert(bitcoin_agent.get_current_fee(#Standard) == #ok 51_000);

    assert(Result.isOk(bitcoin_agent.get_current_fee(#Percentile(98))));

    assert(Result.isErr(bitcoin_agent.get_current_fee(#Percentile(99))));
};