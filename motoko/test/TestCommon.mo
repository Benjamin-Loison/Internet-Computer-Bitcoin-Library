import Types "../src/Types";
import Agent "../src/Agent";

import CanisterMock "CanisterMock";

module {
    type Network = Types.Network;
    type AddressType = Types.AddressType;

    /// Creates a new instance of the Bitcoin agent using the management canister mock.
    public func new_mock(
        network : Network,
        main_address_type : AddressType,
    ) : Agent.BitcoinAgent {
        let management_canister = CanisterMock.ManagementCanisterMock(network);
        Agent.BitcoinAgent(management_canister, main_address_type, 0)
    };
}