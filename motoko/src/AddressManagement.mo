import Result "mo:base/Result";
import Buffer "mo:base/Buffer";
import Array "mo:base/Array";
import Iter "mo:base/Iter";
import Option "mo:base/Option";
import Nat8 "mo:base/Nat8";

import Wif "../motoko-bitcoin/src/bitcoin/Wif";
import BitcoinTypes "../motoko-bitcoin/src/bitcoin/Types";
import Affine "../motoko-bitcoin/src/ec/Affine";
import Jacobi "../motoko-bitcoin/src/ec/Jacobi";
import PublicKey "../motoko-bitcoin/src/ecdsa/Publickey";
import P2pkh "../motoko-bitcoin/src/bitcoin/P2pkh";
import Bip32 "../motoko-bitcoin/src/Bip32";
import EcdsaTypes "../motoko-bitcoin/src/ecdsa/Types";

import Types "Types";
import Agent "AgentPrototype";
import Utils "Utils";

module {
    type PrivateKey = BitcoinTypes.BitcoinPrivateKey;
    type PublicKey = EcdsaTypes.PublicKey;
    type Address = Types.Address;
    type AddressType = Types.AddressType;
    type Network = Types.Network;
    type EcdsaPubKey = Types.EcdsaPubKey;
    let CURVE = Types.CURVE;
    type BitcoinAgent = Agent.BitcoinAgent;
    type Result<Ok, Err> = Result.Result<Ok, Err>;
    type AddAddressError = Types.AddAddressError;
    type Path = Bip32.Path;

    // A private key in WIF (wallet import format). This is only for demonstrational
    // purposes. When the Bitcoin integration is released on mainnet, canisters will
    // have the ability to securely generate ECDSA keys.
    let PRIVATE_KEY_WIF = "L2C1QgyKqNgfV7BpEPAm6PVn2xW8zpXq6MojSbWdH18nGQF2wGsT";

    /// Returns the Bitcoin private key.
    func get_btc_private_key() : PrivateKey {
        Utils.get_ok(Wif.decode(PRIVATE_KEY_WIF))
    };

    /// Returns the Bitcoin public key.
    func get_btc_public_key() : PublicKey {
        let private_key = get_btc_private_key();
        let sk = private_key.key;
        let point = Jacobi.toAffine(Jacobi.mulBase(sk, CURVE));
        Utils.get_ok(PublicKey.decode(#point point))
    };

    /// Returns the Bitcoin ECDSA public key from a given public key.
    func get_btc_ecdsa_public_key_from_public_key(public_key : PublicKey) : EcdsaPubKey {
        // TODO(ER-2617): Add support for public child key derivation from a given derivation path (should use tECDSA to get the canister’s extended public key).
        let coords = public_key.coords;
        let point = #point (coords.x, coords.y, public_key.curve);
        {
            public_key = Affine.toBytes(point, true);
            chain_code = [];
            derivation_path = [];
        }
    };

    /// Returns the Bitcoin ECDSA public key.
    public func get_btc_ecdsa_public_key() : EcdsaPubKey {
        get_btc_ecdsa_public_key_from_public_key(get_btc_public_key())
    };

    /// Returns the P2PKH address from a given network and public key.
    public func get_p2pkh_address(
        network : Network,
        public_key: PublicKey,
    ) : Address {
        P2pkh.deriveAddress(network, PublicKey.toSec1(public_key, true))
    };

    /// Returns the Bitcoin address from a given network, address type and public key.
    func get_address(
        network : Network,
        address_type : AddressType,
        public_key : PublicKey,
    ) : Address {
        switch address_type {
            case (#P2pkh) {
                get_p2pkh_address(network, public_key);
            };
        }
    };

    /// Returns the Bitcoin address for a given network, address type and public key.
    public func get_main_address(network : Network, address_type : AddressType) : Address {
        let public_key = get_btc_public_key();
        get_address(network, address_type, public_key)
    };

    /// Returns the (31-bit) child number associated with a given vector of at most four `u8`s.
    /// Assuming that the first bit is zero, making `child_bytes` always corresponds to an unhardened derivation path.
    /// It is the case by following the only possible code path to reach `get_child_number`.
    func get_child_number(child_bytes : [Nat8]) : Nat32 {
        var index = Utils.nat8_to_nat32(child_bytes[0]) << 24;
        if (child_bytes.size() > 1) {
            index |= Utils.nat8_to_nat32(child_bytes[1]) << 16;
            if (child_bytes.size() > 2) {
                index |= Utils.nat8_to_nat32(child_bytes[2]) << 8;
                if (child_bytes.size() > 3) {
                    index |= Utils.nat8_to_nat32(child_bytes[3]);
                };
            };
        };
        index
    };

    /// Return a valid [BIP-32 derivation path](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki#public-parent-key--public-child-key).
    ///
    /// Each byte string (`blob`) in the `derivation_path` must be a 4-byte
    /// big-endian encoding of an unsigned integer less than 2^31 for non-hardened key derivation.
    public func get_derivation_path(input : [Nat8]) : [[Nat8]] {
        // Below there is an example of how indexes changes for each iteration. Each column represents
        // setting a bit in the result:
        //
        // i   0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 ...
        // ip  0                       1                       2                       3                       4    ...
        // iz  0  1  2  3  4  5  6  7  0  1  2  3  4  5  6  7  0  1  2  3  4  5  6  7  0  1  2  3  4  5  6  7  0  1 ...
        // cp  0                    1                       2                       3                       0       ...
        // cz  1  2  3  4  5  6  7  0  1  2  3  4  5  6  7  0  1  2  3  4  5  6  7  0  1  2  3  4  5  6  7  1  2  3 ...
        //
        // e.g. for i = 23, we set the bit 0 of the current word at position 3 to the value of the bit 7
        //      of the input word at position 2

        let res : Buffer.Buffer<[Nat8]> = Buffer.Buffer(1); // the final result of this function
        var buff = Array.init<Nat8>(4, 0); // buffer for the next 4-byte word to be put into the result
        for (i in Iter.range(0, 8 * input.size() - 1)) {
            // the position in the "flattened" input
            let y = i % 31; // flush to res when y == 0
            if (i > 0 and y == 0) {
                // curr is completed, flush and clear it
                res.add(Array.freeze(buff));
                buff := Array.init<Nat8>(4, 0);
            };
            // we need to set a bit in curr to the value of i in input. We do this in 2 steps:

            // 1) set b to the value of the current bit in input
            let ip = i / 8; // position in input
            let iz = Nat8.fromNat(i % 8); // position in input[ip]
            let b = 0x80 & (input[ip] << iz); // store the bit in the most significant bit

            // 2) set the bit in curr. Note that we need the +1 because the most important bit is always 0
            let cp = (y + 1) / 8; // position in curr
            let cz = Nat8.fromNat((y + 1) % 8); // position in curr[p]
            buff[cp] |= b >> cz;
        };
        if (input.size() % 8 != 0) {
            res.add(Array.freeze(buff));
        };

        res.toArray()
    };

    /// Adds an address based on the provided derivation path and address type to the list of managed addresses.
    /// A minimum number of confirmations must further be specified, which is used when calling `get_utxos` and `get_balance`.
    /// Returns the derived address if the operation is successful and an error otherwise.
    public func add_address_with_parameters(
        bitcoin_agent : BitcoinAgent,
        network : Network,
        derivation_path : [Nat8],
        address_type : AddressType,
        min_confirmations : Nat32,
    ) : Result.Result<Address, AddAddressError> {
        if ((8 * derivation_path.size()) > (255 * 31)) {
            return #err(#DerivationPathTooLong);
        };
        // TODO(ER-2617): Add support for public child key derivation from a given derivation path (should modify motoko-bitcoin module in order to support extended BIP-32 derivation path (including “arbitrary“ length) instead of using `get_derivation_path`).
        let address = add_address_from_unhardened_path(
            bitcoin_agent,
            network,
            get_derivation_path(derivation_path),
            address_type,
            min_confirmations,
        );
        #ok address
    };

    /// Returns the public key and address of the derived child from the given public key, chain code, derivation path, address type and network.
    public func derive_ecdsa_public_key_and_address_from_unhardened_path(
        derivation_path : [[Nat8]],
        address_type : AddressType,
        network : Network,
        ecdsa_public_key : EcdsaPubKey,
    ) : (EcdsaPubKey, Address) {
        let derivation_path_iter = Iter.fromArray(derivation_path);
        let path_iter : Iter.Iter<Nat32> = Iter.map(derivation_path_iter, get_child_number);
        let path : Path = #array (Iter.toArray(path_iter));

        let extended_public_key = Bip32.ExtendedPublicKey(ecdsa_public_key.public_key, ecdsa_public_key.chain_code, 0, 0, null);
        let child_extended_public_key = Utils.unwrap(extended_public_key.derivePath(path));

        let child_point = Utils.unwrap(Affine.fromBytes(child_extended_public_key.key, Types.CURVE));
        let child_public_key = Utils.get_ok(PublicKey.decode(#point child_point));

        let address = get_address(network, address_type, child_public_key);

        let child_ecdsa_public_key = {
            public_key = child_extended_public_key.key;
            chain_code = ecdsa_public_key.chain_code;
            derivation_path = Utils.concat(ecdsa_public_key.derivation_path, derivation_path);
        };

        (child_ecdsa_public_key, address)
    };

    /// Adds the address for the given unhardened derivation path and address type to the given BitcoinAgent if the derived address is not already managed.
    /// This function assumes that the passed derivation path is an unhardened path. This assumption has to be checked in the caller function.
    public func add_address_from_unhardened_path(
        bitcoin_agent : BitcoinAgent,
        network : Network,
        derivation_path : [[Nat8]],
        address_type : AddressType,
        min_confirmations : Nat32,
    ) : Address {
        let (ecdsa_public_key, address) = derive_ecdsa_public_key_and_address_from_unhardened_path(
            derivation_path,
            address_type,
            network,
            get_btc_ecdsa_public_key(),
        );
        if (Option.isNull(bitcoin_agent.ecdsa_pub_key_addresses.get(address))) {
            bitcoin_agent.ecdsa_pub_key_addresses
                .put(address, ecdsa_public_key);
            let utxos_state = Types.utxos_state_new(min_confirmations);
            bitcoin_agent.utxos_state_addresses
                .put(address, utxos_state);
        };
        address
    };

    /// Removes the given address from the list of addresses managed by the given BitcoinAgent.
    /// The address is removed if it is already managed and if it is different from the main address.
    /// Returns true if the removal was successful, false otherwise.
    public func remove_address(
        bitcoin_agent : BitcoinAgent,
        address : Address,
    ) : Bool {
        let address_can_be_removed = Option.isSome(bitcoin_agent.ecdsa_pub_key_addresses.get(address)) and address != bitcoin_agent.get_main_address();
        if address_can_be_removed {
            bitcoin_agent.ecdsa_pub_key_addresses.delete(address);
            bitcoin_agent.utxos_state_addresses.delete(address);
        };
        address_can_be_removed
    };

    /// Returns the currently managed addresses of the given BitcoinAgent.
    public func list_addresses(bitcoin_agent : BitcoinAgent) : [Address] {
        Iter.toArray(bitcoin_agent.ecdsa_pub_key_addresses.keys())
    };
}
