import Result "mo:base/Result";
import Debug "mo:base/Debug";
import Nat32 "mo:base/Nat32";
import Nat8 "mo:base/Nat8";
import Buffer "mo:base/Buffer";

module {
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    
    /// Returns the value of the result.
    public func get_ok<T, U>(result : Result<T, U>) : T {
        switch (result) {
            case (#ok value) {
                value
            };
            case (#err(error)) {
                Debug.trap("pattern failed")
            };
        }
    };

    /// Unwraps the value of the option.
    public func unwrap<T>(option : ?T) : T {
        switch (option) {
            case (?value) {
                value
            };
            case null {
                Debug.trap("Prelude.unreachable()")
            };
        }
    };

    /// Returns the provided `Nat8` as a `Nat32`.
    public func nat8_to_nat32(n : Nat8) : Nat32 {
        Nat32.fromNat(Nat8.toNat(n))
    };

    /// Returns a buffer made of the given array.
    public func buffer_from_array<T>(array : [T]) : Buffer.Buffer<T> {
        let buffer = Buffer.Buffer<T>(array.size());
        for (entry in array.vals()) {
            buffer.add(entry);
        };
        buffer
    };

    /// Returns the concatenation of two arrays.
    public func concat<T>(array_0 : [T], array_1 : [T]) : [T] {
        let buffer_0 : Buffer.Buffer<T> = buffer_from_array(array_0);
        let buffer_1 : Buffer.Buffer<T> = buffer_from_array(array_1);

        buffer_0.append(buffer_1);
        buffer_0.toArray()
    };
}