//! # Internet Computer Rust Bitcoin Library
//!
//! Please read the [library's README](https://github.com/Benjamin-Loison/Internet-Computer-Bitcoin-Library/blob/main/README.adoc) first for an overview of its current features.
//!
//! The core component of the library is the stateful [BitcoinAgent]. It can be used for the following tasks:
//!
//! * It can derive and manage Bitcoin addresses, handling the associated unspent transaction outputs (UTXOs).
//! * It can provide information about account balances of Bitcoin addresses.
//! * It can be used to transfer bitcoins from a managed address to any other address.
//!
//! A step-by-step tutorial is presented in [Section 1](#1-step-by-step-tutorial).
//!
//! Snippets of sample code to illustrate its usage are provided in [Section 2](#2-sample-code).
//!
//! As mentioned above, the [BitcoinAgent] is stateful. Therefore, it is important to store and load the agent’s state properly in the canister’s life cycle management. This aspect is discussed in detail in [Section 3](#3-life-cycle-management).
//!
//! It's an established and secure practice to encapsulate global state within a [Cell]/[RefCell] and this practice should be followed with respect to the [BitcoinAgent].
//! In order to ensure the integrity of a [RefCell]<[BitcoinAgent]>, for instance, getting the balance of an address has to use `get_balance!` or the equivalent lines of code detailed in [Section 4](#4-best-practices-for-the-management-of-global-state).
//!
//! While working on the Internet Computer does not require more configuration, working locally does. The additional instructions are provided in [Section 5](#5-testing-locally).

//! # 0. Disclaimer


//! While the library is reasonably well tested, it is still under development, therefore use it *at your own risk*.
//!
//! This library is *work in progress* and is *not production-grade* yet. At least the following aspects of the library need further consideration before using it in production use cases:
//!
//! * Canister upgradability and management of state related to the library requires further consideration before the library can be used in production. If upgradeability of a canister w.r.t. the replicated state managed by the library is flawed, we risk canisters to break on an upgrade in production, in the worst case in a non-recoverable or hard-to-recover fashion.
//! * More test coverage would be important for some parts of the code.
//! * We have not gained much experience yet in using the library in real-world use cases.

//! # 1. Step-by-step tutorial

//! Make sure that [Rust](https://www.rust-lang.org/tools/install) and [dfx](https://github.com/dfinity/sdk#getting-started) are installed.
//!
//! Create a Rust example dfx project.

//! ```bash
//! dfx new --type rust example_rust
//! ```

//! Move into the `example_rust/` project directory.

//! ```bash
//! cd example_rust/
//! ```

//! Add the most recent version of the `ic-btc-library` and its dependency to your `src/example_rust_backend/Cargo.toml` dependencies.

//! ```toml
//! ic-btc-library = { git = "https://github.com/Benjamin-Loison/Internet-Computer-Bitcoin-Library/" }
//! bitcoin = "0.28.1"
//! ```

//! Replace the content of `src/example_rust_backend/src/lib.rs` with the sample code from [Section 2](#2-sample-code).
//!
//! While working on the Internet Computer does not require more configuration, working locally does. The additional instructions are provided in [Section 5](#5-testing-locally).
//!
//! Replace the content of `src/example_rust_backend/example_rust_backend.did` with:

//! ```candid
//! service : {
//!     "main": () -> (text, nat64, text, variant { Ok : text; Err : null });
//! }
//! ```

//! Install `ic-cdk-optimizer` to optimize the output WASM module.

//! ```bash
//! cargo install ic-cdk-optimizer
//! ```

//! Deploy (or redeploy by adding `--mode=reinstall`) your canister using the `ic-btc-library`.
//!
//! Note: On macOS a specific version of llvm-ar and clang need to be set, otherwise the WASM compilation of rust-secp256k1 will fail. To this end, Mac users first need to run the following command:

//! ```bash
//! AR="/usr/local/opt/llvm/bin/llvm-ar" CC="/usr/local/opt/llvm/bin/clang" cargo build --target "wasm32-unknown-unknown" --release
//! ```

//! Run the following command to deploy the canister locally:

//! ```bash
//! dfx deploy
//! ```

//! Run the following command to deploy the canister on the Internet Computer (make sure to use `Network::Testnet` or `Network::Mainnet` instead of `Network::Regtest`):

//! ```bash
//! dfx deploy --network ic
//! ```

//! Execute the `main` function locally.

//! ```bash
//! dfx canister call example_rust_backend main
//! ```

//! Execute the `main` function on the Internet Computer.

//! ```bash
//! dfx canister --network ic call example_rust_backend main
//! ```

//! If you are running the code locally then the output of `ic_cdk::print` is displayed on the terminal running dfx.
//!
//! If you are interested in sending bitcoins to the canister you created, [see these instructions](#sending-bitcoin-to-the-example-canister).

//! # 2. Sample Code

//! The following code shows how to create a [BitcoinAgent] instance, add a managed address derived from the canister’s public key and get its current balance.
//! ```ignore
//! use ic_cdk::print;
//! # use ic_btc_library::{AddressType, Network, BitcoinAgent, ManagementCanister, ManagementCanisterMock, Satoshi, Fee};
//! # /*
//! use ic_cdk_macros::update;
//! use ic_btc_library::{AddressType, Network, BitcoinAgent, ManagementCanister, ManagementCanisterImpl, Satoshi, Fee, get_balance_from_args, get_initialization_parameters_from_args, multi_transfer_from_args, get_utxos_from_args};
//! # */
//! use std::collections::BTreeMap;
//!
//! # #[tokio::main]
//! # async fn main() {
//! # /*
//! #[update]
//! pub async fn main() -> (String, Satoshi, String, Result<String, ()>) {
//! # */
//!     # /*
//!     let num_confirmations = 6;
//!     # */
//!     # let num_confirmations = 1;
//!
//!     let mut agent = BitcoinAgent::new(
//!         // Choose the Bitcoin network your `BitcoinAgent` will use: mainnet, testnet, or regtest.
//!         # /*
//!         ManagementCanisterImpl::new(Network::Regtest),
//!         # */
//!         # ManagementCanisterMock::new(Network::Regtest),
//!         &AddressType::P2pkh,
//!         num_confirmations,
//!     ).unwrap();
//!
//!     // Initializes the Bitcoin agent.
//!     let get_initialization_parameters_args = agent.get_initialization_parameters_args();
//!     # /*
//!     let initialization_parameters = get_initialization_parameters_from_args(get_initialization_parameters_args).await.unwrap();
//!     # */
//!     # let initialization_parameters = agent.get_initialization_parameters_from_args_test(get_initialization_parameters_args).unwrap();
//!     agent.initialize(initialization_parameters);
//!
//!     // Print the address of the main account and its balance:
//!     let main_address = agent.get_main_address();
//!     # /*
//!     print(&format!("Main account address: {}", main_address));
//!     let get_utxos_args = agent.get_utxos_args(&main_address, num_confirmations);
//!     let balance = get_balance_from_args(get_utxos_args).await.unwrap();
//!     print(&format!("Main account balance: {}", balance));
//!     # */
//!     # println!("Main account address: {}", main_address);
//!     # let get_utxos_args = agent.get_utxos_args(&main_address, num_confirmations);
//!     # let balance = agent.get_balance_from_args_test(get_utxos_args).unwrap();
//!     # println!("Main account balance: {}", balance);
//!
//!     // Derive an address and print it:
//!     let derivation_path: &[Vec<u8>] = &[vec![1]];
//!     let new_address = agent.add_address(&derivation_path).unwrap();
//!     # /*
//!     print(&format!("Derived address: {}", new_address));
//!     # */
//!     # println!("Derived address: {}", new_address);
//!
//!     // Send bitcoin to a derived address:
//!     let amount: Satoshi = 1_000_000;
//!     let payouts = BTreeMap::from([(new_address.clone(), amount)]);
//!
//!     let get_utxos_args = agent.get_utxos_args(&main_address, num_confirmations);
//!     # /*
//!     let get_utxos_result = get_utxos_from_args(get_utxos_args).await.unwrap();
//!     # */
//!     # let get_utxos_result = agent.get_utxos_from_args_test(get_utxos_args).unwrap();
//!     agent.apply_utxos(get_utxos_result);
//!     agent.get_balance_update(&main_address).unwrap();
//!
//!     let multi_transfer_args = agent.get_multi_transfer_args(&payouts, &main_address, Fee::Standard, num_confirmations, false);
//!     # /*
//!     let multi_transfer_result = multi_transfer_from_args(multi_transfer_args).await;
//!     # let multi_transfer_result = agent.multi_transfer_from_args_test(multi_transfer_args).await;
//!     let multi_transfer_result = if let Ok(multi_transfer_result) = multi_transfer_result {
//!         agent.apply_multi_transfer_result(&multi_transfer_result);
//!         Ok(multi_transfer_result.transaction_info.id)
//!     } else {
//!         Err(())
//!     };
//!     # */
//!
//!     // If running on the Internet Computer, then `ic_cdk::print` doesn't print anywhere.
//!     // So to get the output, we return the printed variables.
//!     # /*
//!     (main_address.to_string(), balance, new_address.to_string(), multi_transfer_result)
//!     # */
//! }
//! ```

//! Given a [BitcoinAgent] instance, it is possible to get updates for a particular address using the function [`get_balance_update`](BitcoinAgent::get_balance_update):

//! ```ignore
//! # use ic_btc_library::{AddressType, BitcoinAgent, ManagementCanister, ManagementCanisterMock, tests::new_mock, Network};
//! #
//! # fn main() {
//! # let mut agent = new_mock(&Network::Regtest, &AddressType::P2pkh);
//! # let address = agent.get_main_address();
//! #
//! let balance_update = agent.get_balance_update(&address).unwrap();
//! if balance_update.added_balance > 0 {
//!     // ...
//! }
//! # }
//! ```

//! Note that the [`get_balance_update`](BitcoinAgent::get_balance_update) call changes the state of the agent. If the function is called again before any other balance change is recorded, the return value will indicate no balance changes, i.e., `balance_update.added_balance == 0`.
//! In a more complex example, asynchronous actions may be triggered based on the update. If these actions fail, the library state should not change in order to avoid inconsistencies.
//! This case can be handled using [`peek_balance_update`](BitcoinAgent::peek_balance_update) and [`update_state`](BitcoinAgent::update_state) as follows.

//! ```ignore
//! # use ic_btc_library::{AddressType, BitcoinAgent, ManagementCanister, ManagementCanisterMock, tests::new_mock, Network};
//! #
//! # fn main() {
//! # let mut agent = new_mock(&Network::Regtest, &AddressType::P2pkh);
//! # let address = agent.get_main_address();
//! #
//! // ...
//! // NOTE: A guard must be in place to prevent access to the given
//! // address until the end of the code snippet!
//! let balance_update = agent.peek_balance_update(&address).unwrap();
//! if balance_update.added_balance > 0 {
//!     // async_call(balance_update.added_balance).await.unwrap();
//!     // The state is updated after completing the asynchronous call.
//!     agent.update_state(&address);
//! }
//! // Access to the address can be made available again here.
//! # }
//! ```

//! Calling [`peek_balance_update`](BitcoinAgent::peek_balance_update) followed by [`update_state`](BitcoinAgent::update_state) is equivalent to calling [`get_balance_update`](BitcoinAgent::get_balance_update).
//!
//! As noted in the code snippet, care needs to be taken not to call [`peek_balance_update`](BitcoinAgent::peek_balance_update) multiple times for concurrent requests when waiting for a response for the asynchronous call.
//! The simplest approach is to keep a data structure with all addresses that are currently being served. The code snippet must not be executed for any address currently found in the data structure.
//!
//! Moreover, it is important to ensure that:
//! - the same address is never managed by multiple [BitcoinAgent]s
//! - the `multi_transfer` function isn't executed multiple times concurrently

//! # 3. Life Cycle Management

//! The canister developer has the responsibility to initialize [BitcoinAgent]s and to store and restore the [BitcoinAgent]s' states during canister upgrades.
//!
//! As far as initialization is concerned, the canister developer must ensure that [`initialize`](BitcoinAgent::initialize) is called before any [BitcoinAgent] is used. The canister developer has multiple options such as:
//! - Initializing the [BitcoinAgent]s by adding a custom endpoint that needs to be called once. This endpoint can then be removed in a canister upgrade.
//! - Calling [BitcoinAgent::initialize] in every function before using the agent. Note that it is okay to call the function multiple times as the initialization will only happen on the first invocation.
//!
//! As far as storing and restoring state is concerned, the following sample code shows how to manage a single [BitcoinAgent] instance.

//! ```
//! use ic_cdk::storage;
//! use std::cell::RefCell;
//! use ic_btc_library::{BitcoinAgentState, AddressType, Network, BitcoinAgent, ManagementCanister, ManagementCanisterImpl};
//! use ic_cdk_macros::{post_upgrade, pre_upgrade};
//!
//! thread_local! {
//!     static BITCOIN_AGENT: RefCell<BitcoinAgent<ManagementCanisterImpl>> =
//!         RefCell::new(BitcoinAgent::new(ManagementCanisterImpl::new(Network::Regtest), &AddressType::P2pkh, 0).unwrap());
//! }
//!
//! #[pre_upgrade]
//! fn pre_upgrade() {
//!     BITCOIN_AGENT
//!         .with(|bitcoin_agent| storage::stable_save((bitcoin_agent.borrow().get_state(),)).unwrap());
//! }
//!
//! #[post_upgrade]
//! fn post_upgrade() {
//!     let (old_bitcoin_agent_state,): (BitcoinAgentState,) = storage::stable_restore().unwrap();
//!     BITCOIN_AGENT.with(|bitcoin_agent| {
//!         *bitcoin_agent.borrow_mut() = BitcoinAgent::from_state(old_bitcoin_agent_state)
//!     });
//! }
//! ```

//! Note that the functions must be annotated with `#[pre_upgrade]` and `#[post_upgrade]`.

//! Furthermore the canister developer must enforce that no address is managed by multiple [BitcoinAgent]s.

//! # 4. Best practices for the management of global state

//! In order to ensure the integrity of a [RefCell]<[BitcoinAgent]>, for instance, getting the balance of an address has to be done as follows:

//! ```ignore
//! # use std::cell::RefCell;
//! # use ic_btc_library::{AddressType, BitcoinAgent, ManagementCanister, ManagementCanisterMock, get_utxos_from_args, tests::new_mock, Network};
//! #
//! # thread_local! {
//! #     static BITCOIN_AGENT: RefCell<BitcoinAgent<ManagementCanisterMock>> =
//! #        RefCell::new(new_mock(&Network::Regtest, &AddressType::P2pkh));
//! # }
//! #
//! # fn main() {
//! # let address = BITCOIN_AGENT.with(|bitcoin_agent| bitcoin_agent.borrow().get_main_address());
//! let get_utxos_args = BITCOIN_AGENT.with(|bitcoin_agent| bitcoin_agent.borrow().get_utxos_args(&address, 0));
//! # let balance = BITCOIN_AGENT.with(|bitcoin_agent| bitcoin_agent.borrow().get_balance_from_args_test(get_utxos_args).unwrap());
//! # /*
//! let balance = BITCOIN_AGENT.with(|bitcoin_agent| bitcoin_agent.borrow().get_balance_from_args(get_utxos_args).await.unwrap());
//! # */
//! # }
//! ```

//! Note that the macro `get_balance!` can be used instead, which is equivalent to the lines of code above.

//! # 5. Testing locally

//! The [BitcoinAgent] invokes the Bitcoin integration API through the management canister.
//! In order to test the Bitcoin wallet locally, follow the instructions below.

//! # Prerequisites

//! - [Bitcoin Core](https://bitcoin.org/en/download). Mac users are recommended to download the `.tar.gz` version.
//!
//! The first step is to setup a local Bitcoin network.

//! # Setting up a local Bitcoin network

//! 1. Unpack the `.tar.gz` file.

//! 2. Create a directory named `data` inside the unpacked folder.

//! 3. Create a file called `bitcoin.conf` at the root of the unpacked folder and add the following contents:
//! ```conf
//! ## Enable regtest mode. This is required to setup a private Bitcoin network.
//! regtest=1
//!
//! ## Dummy credentials that are required by `bitcoin-cli`.
//! rpcuser=btc-library
//! rpcpassword=Wjh4u6SAjT4UMJKxPmoZ0AN2r9qbE-ksXQ5I2_-Hm4w=
//! rpcauth=btc-library:8555f1162d473af8e1f744aa056fd728$afaf9cb17b8cf0e8e65994d1195e4b3a4348963b08897b4084d210e5ee588bcb
//! ```

//! 4. Run bitcoind to start the Bitcoin client using the following command:
//! ```bash
//! ./bin/bitcoind -conf=$(pwd)/bitcoin.conf -datadir=$(pwd)/data
//! ```

//! 5. Create a wallet:
//! ```bash
//! ./bin/bitcoin-cli -conf=$(pwd)/bitcoin.conf createwallet mywallet
//! ```
//! If everything is setup correctly, you should see the following output:
//! ```bash
//! {
//!   "name": "mywallet",
//!   "warning": ""
//! }

//! ```
//! 6. Generate a Bitcoin address and save it in a variable for later reuse:
//! ```bash
//! export BTC_ADDRESS=$(./bin/bitcoin-cli -conf=$(pwd)/bitcoin.conf getnewaddress)
//! ```
//! This will generate a Bitcoin address for your wallet to receive funds.
//!
//! 7. Mine blocks to receive some bitcoins as a reward.
//! ```bash
//! ./bin/bitcoin-cli -conf=$(pwd)/bitcoin.conf generatetoaddress 101 $BTC_ADDRESS
//! ```
//! You should see an output that looks similar to, but not exactly like, the following:
//! ```bash
//! [
//!   "1625281b2595b77276903868a0fe2fc31cb0c624e9bdc269e74a3f319ceb48de",
//!   "1cc5ba7e86fc313333c5448af6c7af44ff249eca3c8b681edc3c275efd3a2d38",
//!   "1d3c85b674497ba08a48d1b955bee5b4dc4505ffe4e9f49b428153e02e3e0764",
//!   ...
//!   "0dfd066985dc001ccc1fe6d7bfa53b7ad4944285dc173615792653bbd52151f1",
//!   "65975f1cd5809164f73b0702cf326204d8fee8b9669bc6bd510cb221cf09db5c",
//! ]
//! ```

//! # Synchronize blocks from bitcoind and create the canister

//! Make your project use your local bitcoind by adding `bitcoin` to the entry `defaults` in your `dfx.json` file.

//! ```json
//!     "bitcoin": {
//!         "enabled": true,
//!         "nodes": ["127.0.0.1:18444"],
//!         "log_level": "info"
//!     },
//! ```

//! Synchronize blocks from bitcoind with the replica by executing the following command in the `example_rust` folder:

//! ```bash
//! dfx start
//! ```

//! # Sending bitcoin to the example canister

//! To top up the example canister with bitcoins, run the following:

//! ```bash
//! ## Send a transaction that transfers 10 BTC to the canister.
//! ## `$CANISTER_BTC_ADDRESS` is the Bitcoin canister address that you get by running `BitcoinAgent::get_main_address`.
//! ./bin/bitcoin-cli -conf=$(pwd)/bitcoin.conf -datadir=$(pwd)/data sendtoaddress $CANISTER_BTC_ADDRESS 10 "" "" true true null "unset" null 1.1
//!
//! ## Mine 6 blocks that contains the transaction in order to reach provided `min_confirmations`.
//! ./bin/bitcoin-cli -conf=$(pwd)/bitcoin.conf generatetoaddress 6 $BTC_ADDRESS
//! ```
//!
//! If successful, querying the balance of the canister should return the updated balance.

pub mod address_management;
mod agent;
mod bip32_extended_derivation;
mod canister_common;
mod canister_implementation;
#[cfg(test)]
pub mod canister_mock;
mod ecdsa;
mod transaction_management;
mod types;
mod upgrade_management;
mod utxo_management;

pub use ic_btc_types::{MillisatoshiPerByte, OutPoint, Satoshi, Utxo};
pub use types::{
    AddAddressWithParametersError, AddressNotTracked, AddressType, AddressUsingPrimitives,
    BalanceUpdate, BitcoinAgentState, CurrentFeeArgs, CurrentFeesArgs, DerivationPathTooLong,
    ECDSAPublicKeyReply, EcdsaPubKey, Fee, FeeRequest, GetCurrentFeeError, GetUtxosError,
    InitializationParametersArgs, InvalidPercentile, ManagementCanisterReject,
    MinConfirmationsTooHigh, MultiTransferArgs, MultiTransferError, MultiTransferResult, Network,
    TransactionID, TransactionInfo, UtxosArgs, UtxosResult, UtxosState, UtxosUpdate,
    MIN_CONFIRMATIONS_UPPER_BOUND,
};

pub use agent::{
    get_balance_from_args, get_current_fee_from_args, get_current_fees_from_args,
    get_initialization_parameters_from_args, get_utxos_from_args, multi_transfer_from_args,
    BitcoinAgent,
};
pub use canister_common::ManagementCanister;
pub use canister_implementation::ManagementCanisterImpl;

/*
    To run documentation tests:
    1. uncomment the `use` line below.
    2. comment `#[cfg(test)]` above, above the second `use` of agent.rs, in agent.rs above `impl BitcoinAgent<ManagementCanisterMock>`, in agent.rs above `pub mod tests` and in address_management.rs above `pub mod tests`.
    3. remove the four `ignore` documentation test attribute above.
    4. add `hex = "0.4.3"` to Cargo.toml `[dependencies]`
*/
/*pub use {
    agent::tests,
    canister_mock::ManagementCanisterMock,
    std::cell::{Cell, RefCell},
};*/
