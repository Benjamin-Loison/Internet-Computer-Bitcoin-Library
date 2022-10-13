import ExperimentalCycles "mo:base/ExperimentalCycles";
import Result "mo:base/Result";
import Prelude "mo:base/Prelude";
import Error "mo:base/Error";

import Types "Types";
import CanisterCommon "CanisterCommon";

module {
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    type ManagementCanisterActor = Types.ManagementCanisterActor;
    type Network = Types.Network;
    type MillisatoshiPerByte = Types.MillisatoshiPerByte;
    type GetCurrentFeesError = Types.GetCurrentFeesError;
    let GET_CURRENT_FEE_PERCENTILES_COST_CYCLES = CanisterCommon.GET_CURRENT_FEE_PERCENTILES_COST_CYCLES;

    /// Returns fees as percentiles in millisatoshis/byte over the last 10,000 transactions.
    public func get_current_fees(management_canister_actor : ManagementCanisterActor, network : Network) : async Result<[MillisatoshiPerByte], GetCurrentFeesError> {
        ExperimentalCycles.add(GET_CURRENT_FEE_PERCENTILES_COST_CYCLES);
        try {
            #ok (await management_canister_actor.bitcoin_get_current_fee_percentiles({
                network = network;
            }))
        } catch (e) {
            #err (#ManagementCanisterReject (Error.code(e), Error.message(e)))
        }
        // Comment above code from `get_current_fees` and uncomment the line below to make tests compilable.
        //Prelude.unreachable();
    };
}