import Iter "mo:base/Iter";
import Array "mo:base/Array";
import Buffer "mo:base/Buffer";
import Text "mo:base/Text";

import Types "../src/Types";
import Utils "../src/Utils";
import AddressManagement "../src/AddressManagement";

import TestUtils "Utils";
import TestCommon "TestCommon";

type AddressType = Types.AddressType;
type Address = Types.Address;

/// Returns the parsed `AddressType` based on a generated address of given `address_type`.
func get_parsed_address_type_from_generated_address(
    address_type : AddressType,
) : AddressType {
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);
    Utils.unwrap(TestUtils.get_address_type(bitcoin_agent.get_main_address()))
};

do {
    // Check that `get_main_address` returns an address of the correct type according to Bitcoin agent `main_address_type`.
    for (address_type in Iter.fromArray([
        #P2pkh,
    ])) {
        assert get_parsed_address_type_from_generated_address(address_type) == address_type;
    };
};

/// Returns true if the two given array contain the same addresses without considering the order, otherwise false.
func contains_same_addresses(v0: [Address], v1: [Address]) : Bool {
    Array.sort(v0, Text.compare) == Array.sort(v1, Text.compare)
};

do {
    // Check that `add_address`, `remove_address` and `list_addresses` respectively add, remove and list managed addresses.
    let bitcoin_agent = TestCommon.new_mock(#Regtest, #P2pkh);

    let addresses : Buffer.Buffer<Address> = Buffer.Buffer(2);
    addresses.add(bitcoin_agent.list_addresses()[0]);

    let address = Utils.get_ok(bitcoin_agent.add_address([0]));

    addresses.add(address);

    assert contains_same_addresses(
        bitcoin_agent.list_addresses(),
        addresses.toArray()
    );

    assert bitcoin_agent.remove_address(address);
    ignore(addresses.removeLast());
    assert contains_same_addresses(
        bitcoin_agent.list_addresses(),
        addresses.toArray()
    );
};

/// Check that the public key and address of the derived child match those expected from the given public key, chain code and derivation path.
func test_derive_ecdsa_public_key_and_address_from_unhardened_path(
    public_key : [Nat8],
    chain_code : [Nat8],
    derivation_path : [[Nat8]],
    expected_child_public_key : [Nat8],
    expected_child_address : Address,
) : () {
    let (ecdsa_public_key, address) = AddressManagement.derive_ecdsa_public_key_and_address_from_unhardened_path(
        derivation_path,
        #P2pkh,
        #Mainnet,
        {
            public_key = public_key;
            chain_code = chain_code;
            derivation_path = [];
        },
    );
    assert ecdsa_public_key.public_key == expected_child_public_key;
    assert address == expected_child_address;
};

do {
    test_derive_ecdsa_public_key_and_address_from_unhardened_path(
        [0x03, 0xad, 0xbe, 0x4f, 0x86, 0xc2, 0x69, 0x94, 0xa9, 0x74, 0x46, 0xfb, 0x8f, 0xb3, 0xd3, 0x51, 0x89, 0xc9, 0xeb, 0xf3, 0xf3, 0x8a, 0x2f, 0xce, 0x23, 0xd4, 0x9f, 0x81, 0x1e, 0xdb, 0x6f, 0x2d, 0x0e],
        [0x73, 0x73, 0x84, 0x81, 0x71, 0xc5, 0xf7, 0x98, 0x74, 0xeb, 0x0a, 0x85, 0xcb, 0xa8, 0x46, 0x18, 0xdc, 0xcf, 0x29, 0xa7, 0x18, 0x74, 0xdf, 0x69, 0xdf, 0xc6, 0xb5, 0xc8, 0x37, 0x0d, 0x60, 0x18],
        [[0x7F, 0xFF, 0xFF, 0xFF]],
        [0x03, 0x7b, 0x28, 0x5f, 0xd4, 0x79, 0xb6, 0x4c, 0x9d, 0x8e, 0xbe, 0x08, 0x98, 0x47, 0xbf, 0xc1, 0x26, 0x8d, 0x5f, 0xed, 0x18, 0x82, 0xfa, 0xba, 0xbe, 0x48, 0x14, 0x45, 0x44, 0xfd, 0x9f, 0xdb, 0xfe],
        "19pwaccNmLtmCag1WgRjNHgMTJ7CWcJJu4",
    );

    test_derive_ecdsa_public_key_and_address_from_unhardened_path(
        [0x02, 0x3e, 0x47, 0x40, 0xd0, 0xba, 0x63, 0x9e, 0x28, 0x96, 0x3f, 0x34, 0x76, 0x15, 0x7b, 0x7c, 0xf2, 0xfb, 0x7c, 0x6f, 0xdf, 0x42, 0x54, 0xf9, 0x70, 0x99, 0xcf, 0x86, 0x70, 0xb5, 0x05, 0xea, 0x59],
        [0x18, 0x0c, 0x99, 0x86, 0x15, 0x63, 0x6c, 0xd8, 0x75, 0xaa, 0x70, 0xc7, 0x1c, 0xfa, 0x6b, 0x7b, 0xf5, 0x70, 0x18, 0x7a, 0x56, 0xd8, 0xc6, 0xd0, 0x54, 0xe6, 0x0b, 0x64, 0x4d, 0x13, 0xe9, 0xd3],
        [[0, 0, 0, 1], [0, 0, 0, 2], [0, 0, 0, 3]],
        [0x02, 0x56, 0x11, 0x4e, 0x0a, 0x59, 0x9a, 0xe1, 0x04, 0xc9, 0x08, 0xda, 0xf2, 0xde, 0x6c, 0x06, 0x22, 0xea, 0xfb, 0x35, 0x2f, 0x16, 0x45, 0x2e, 0x95, 0x6d, 0x3f, 0x6c, 0xf5, 0x9b, 0xe6, 0x75, 0xa8],
        "1JnJVbQ9feEmwrwT4NzrEC3MAffg3uMmH4",
    );

    assert(
        AddressManagement.get_derivation_path([0x00] : [Nat8]) == ([[0x00, 0x00, 0x00, 0x00]] : [[Nat8]])
    );
    assert(
        AddressManagement.get_derivation_path([0xff] : [Nat8]) == ([[0x7f, 0x80, 0x00, 0x00]] : [[Nat8]])
    );
    assert(
        AddressManagement.get_derivation_path([0x05] : [Nat8]) == ([[0x02, 0x80, 0x00, 0x00]] : [[Nat8]])
    );
    assert(
        AddressManagement.get_derivation_path([0x00, 0x00] : [Nat8]) == ([[0x00, 0x00, 0x00, 0x00]] : [[Nat8]])
    );
    assert(
        AddressManagement.get_derivation_path([0xff, 0xff] : [Nat8]) == ([[0x7f, 0xff, 0x80, 0x00]] : [[Nat8]])
    );
    assert(
        AddressManagement.get_derivation_path([0x05, 0x05] : [Nat8]) == ([[0x02, 0x82, 0x80, 0x00]] : [[Nat8]])
    );
    assert(
        AddressManagement.get_derivation_path([0x96, 0x75] : [Nat8]) == ([[0x4b, 0x3a, 0x80, 0x00]] : [[Nat8]])
    );
    assert(
        AddressManagement.get_derivation_path([0x00, 0x00, 0x00, 0x00] : [Nat8]) == ([[0x00, 0x00, 0x00, 0x00], [0x00, 0x00, 0x00, 0x00]] : [[Nat8]])
    );
    assert(
        AddressManagement.get_derivation_path([0xff, 0xff, 0xff, 0xff] : [Nat8]) == ([[0x7f, 0xff, 0xff, 0xff], [0x40, 0x00, 0x00, 0x00]] : [[Nat8]])
    );

    // Dear reviewer, I know what you are thinking and yes, this test is nightmare to
    // understand and review. Let me try help you with it.
    //
    // [BIP-32 derivation paths](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki#public-parent-key--public-child-key)
    // are arrays of blobs where each blob is composed by 4 bytes. The first bit of the first
    // byte, i.e. the most significant bit, is always 0.
    //
    // Given an array of bytes called `input`, how can you calculate manually the `expected` result?
    //
    // First of all, each vec of `expected` is equivalent to 4 bytes of `input` shifted right by
    // the position of the vec itself inside expected.
    // For instance, the vec number 1 of `expected` is composed by the first 4 bytes of `input`
    // shifted right by 1:
    // * 0b_1111_0000 >> 1 => 0b_0111_1000
    // * 0b_0011_0101 >> 1 => 0b_0001_1010
    // * 0b_0000_0100 >> 1 => 0b_1000_0010
    // * 0b_1010_1111 >> 1 => 0b_0101_0111
    //
    // Secondly, the bits "overflowing" are moved to the next byte. You can see this in the
    // third byte above. 0b_0000_0100 becomes 0b_1000_0010 where the left-most 1 has overflowed
    // from the second byte.
    //
    // Finally, the left-most bit of each blob in `expected` must be 0. You can see this in
    // the second blob of `expected` 0b_0111_0110. Note that the leftmost 1 overflowed from
    // the previous byte (the last row of the example above).

    let input : [Nat8] = [
        0xf0, 0x35, 0x04, 0xaf,
        0xdb, 0x30, 0x9b, 0x0c,
        0x5d, 0xb8, 0x39, 0x75,
        0x7a, 0xbf, 0xce, 0xc2,
        0x41, 0x89, 0xd1, 0x1f,
        0xe0, 0x66, 0x3d, 0xbb,
        0x74, 0x0e, 0x46, 0x5f,
        0x02];
    let expected : [[Nat8]] = [
        [0x78, 0x1a, 0x82, 0x57],
        [0x76, 0xcc, 0x26, 0xc3],
        [0x0b, 0xb7, 0x07, 0x2e],
        [0x57, 0xab, 0xfc, 0xec],
        [0x12, 0x0c, 0x4e, 0x88],
        [0x7f, 0x81, 0x98, 0xf6],
        [0x76, 0xe8, 0x1c, 0x8c],
        [0x5f, 0x02, 0x00, 0x00],
    ];
    assert(expected ==  AddressManagement.get_derivation_path(input));
};