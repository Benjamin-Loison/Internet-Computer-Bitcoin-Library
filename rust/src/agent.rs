use crate::{
    address_management,
    address_management::get_main_address,
    canister_common::ManagementCanister,
    ecdsa::{get_btc_ecdsa_public_key, get_key_name_from_network},
    transaction_management,
    transaction_management::{get_current_fee, get_current_fees},
    types::{from_bitcoin_network_to_types_network, GetUtxosResponse},
    upgrade_management,
    upgrade_management::get_address,
    utxo_management,
    utxo_management::{get_balance_from_utxos, get_utxos},
    AddAddressWithParametersError, AddressNotTracked, AddressType, BalanceUpdate,
    BitcoinAgentState, CurrentFeeArgs, CurrentFeesArgs, DerivationPathTooLong, EcdsaPubKey, Fee,
    FeeRequest, GetCurrentFeeError, GetUtxosError, InitializationParametersArgs,
    ManagementCanisterReject, MillisatoshiPerByte, MinConfirmationsTooHigh, MultiTransferArgs,
    MultiTransferError, MultiTransferResult, OutPoint, Satoshi, Utxo, UtxosArgs, UtxosResult,
    UtxosState, UtxosUpdate, MIN_CONFIRMATIONS_UPPER_BOUND,
};
#[cfg(test)]
use crate::{canister_mock::ManagementCanisterMock, transaction_management::evaluate_fee_request};
use bitcoin::{hashes, Address};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
pub struct BitcoinAgent<C: ManagementCanister> {
    pub(crate) management_canister: C,
    pub(crate) main_address_type: AddressType,
    pub(crate) ecdsa_pub_key_addresses: BTreeMap<Address, EcdsaPubKey>,
    pub(crate) min_confirmations: u32,
    pub(crate) utxos_state_addresses: BTreeMap<Address, UtxosState>,
}

impl<C: ManagementCanister> BitcoinAgent<C> {
    /// Creates a new Bitcoin agent using the given management canister.
    pub fn new(
        management_canister: C,
        main_address_type: &AddressType,
        min_confirmations: u32,
    ) -> Result<Self, MinConfirmationsTooHigh> {
        if min_confirmations > MIN_CONFIRMATIONS_UPPER_BOUND {
            return Err(MinConfirmationsTooHigh);
        }
        Ok(Self {
            management_canister,
            main_address_type: *main_address_type,
            ecdsa_pub_key_addresses: BTreeMap::default(),
            utxos_state_addresses: BTreeMap::default(),
            min_confirmations,
        })
    }

    /// Returns the Bitcoin agent state.
    pub fn get_state(&self) -> BitcoinAgentState {
        upgrade_management::get_state(self)
    }

    /// Returns the associated Bitcoin agent with the given `bitcoin_agent_state`, assuming that it wasn't modified since its obtention with `get_state`.
    pub fn from_state(bitcoin_agent_state: BitcoinAgentState) -> Self {
        upgrade_management::from_state(bitcoin_agent_state)
    }

    /// Adds an address based on the provided derivation path and address type to the list of managed addresses.
    /// A minimum number of confirmations must further be specified, which is used when calling `get_utxos` and `get_balance`.
    /// Returns the derived address if the operation is successful and an error otherwise.
    pub fn add_address_with_parameters(
        &mut self,
        derivation_path: &[Vec<u8>],
        address_type: &AddressType,
        min_confirmations: u32,
    ) -> Result<Address, AddAddressWithParametersError> {
        address_management::add_address_with_parameters(
            self,
            derivation_path,
            address_type,
            min_confirmations,
        )
    }

    /// Adds an address to the agent with the provided derivation path.
    /// The default address type and default number of confirmations are used.
    pub fn add_address(
        &mut self,
        derivation_path: &[Vec<u8>],
    ) -> Result<Address, DerivationPathTooLong> {
        let address_type = self.main_address_type;
        match self.add_address_with_parameters(
            derivation_path,
            &address_type,
            self.min_confirmations,
        ) {
            Err(AddAddressWithParametersError::DerivationPathTooLong) => Err(DerivationPathTooLong),
            Ok(address) => Ok(address),
            // Other case AddAddressWithParameters::MinConfirmationsTooHigh can't happen see BitcoinAgent::new
            _ => panic!(),
        }
    }

    /// Removes the given address from given BitcoinAgent managed addresses.
    /// The address is removed if it is already managed and if it is different from the main address.
    /// Returns true if the removal was successful, false otherwise.
    pub fn remove_address(&mut self, address: &Address) -> bool {
        address_management::remove_address(self, address)
    }

    /// Returns the managed addresses according to given BitcoinAgent.
    pub fn list_addresses(&self) -> Vec<&Address> {
        address_management::list_addresses(self)
    }

    // TODO(ER-2587): Add support for address management, test spending UTXOs received on addresses of all supported types (relying on ER-2593).

    /// Returns the P2SH address from a given script hash.
    pub fn get_p2sh_address(&self, script_hash: &[u8]) -> Result<Address, hashes::Error> {
        address_management::get_p2sh_address(&self.management_canister.get_network(), script_hash)
    }

    /// Returns the main Bitcoin address of the canister.
    pub fn get_main_address(&self) -> Address {
        address_management::get_main_address(&self.management_canister, &self.main_address_type)
    }

    /// Returns the difference between the current UTXO state and the last seen state for this address.
    /// The last seen state for an address is updated to the current state by calling `update_state` or implicitly when invoking `get_utxos_update`.
    /// If there are no changes to the UTXO set since the last call, the returned `UtxosUpdate` will be identical.
    pub fn peek_utxos_update(&self, address: &Address) -> Result<UtxosUpdate, AddressNotTracked> {
        utxo_management::peek_utxos_update(self, address)
    }

    /// Updates the state of the `BitcoinAgent` for the given `address`.
    /// This function doesn't invoke a Bitcoin integration API function.
    pub fn update_state(&mut self, address: &Address) -> Result<(), AddressNotTracked> {
        utxo_management::update_state(self, address)
    }

    /// Returns the difference in the set of UTXOs of an address controlled by the `BitcoinAgent` between the current state and the seen state when the function was last called, considering only UTXOs with the number of confirmations specified when adding the given address.
    /// The returned `UtxosUpdate` contains the information which UTXOs were added and removed. If the function is called for the first time, the current set of UTXOs is returned.
    /// Note that the function changes the state of the `BitcoinAgent`: A subsequent call will return changes to the UTXO set that have occurred since the last call.
    pub fn get_utxos_update(
        &mut self,
        address: &Address,
    ) -> Result<UtxosUpdate, AddressNotTracked> {
        utxo_management::get_utxos_update(self, address)
    }

    /// Returns the difference between the current balance state and the last seen state for this address.
    /// The last seen state for an address is updated to the current unseen state by calling `update_state` or implicitly when invoking `get_balance_update`.
    /// If there are no changes to the balance since the last call, the returned `BalanceUpdate` will be identical.
    pub fn peek_balance_update(
        &self,
        address: &Address,
    ) -> Result<BalanceUpdate, AddressNotTracked> {
        utxo_management::peek_balance_update(self, address)
    }

    /// Returns the difference in the balance of an address controlled by the `BitcoinAgent` between the current state and the seen state when the function was last called, considering only transactions with the specified number of confirmations.
    /// The returned `BalanceUpdate` contains the information on how much balance was added and subtracted in total. If the function is called for the first time, the current balance of the address is returned.
    /// It is equivalent to calling `get_utxos_update` and summing up the balances in the returned UTXOs.
    pub fn get_balance_update(
        &mut self,
        address: &Address,
    ) -> Result<BalanceUpdate, AddressNotTracked> {
        utxo_management::get_balance_update(self, address)
    }

    // ---
    // Usage pattern to update the utxos state of the agent (eg. with thread_local agents):
    // let args = AGENT.with(|s| s.borrow().get_utxos_args(address));
    // let result = get_utxos_from_args(args).await.unwrap();
    // let utxos = AGENT.with(|s| s.borrow_mut().apply_utxos(result));

    pub fn get_utxos_args(&self, address: &Address, min_confirmations: u32) -> UtxosArgs {
        UtxosArgs {
            network: self.management_canister.get_network(),
            address: address.clone(),
            min_confirmations,
            utxos_state: self
                .utxos_state_addresses
                .get(address)
                .unwrap_or(&UtxosState::new(min_confirmations))
                .clone(),
        }
    }

    pub fn apply_utxos(&mut self, utxos_result: UtxosResult) -> UtxosUpdate {
        let mut utxos_state_address = self
            .utxos_state_addresses
            .get_mut(&utxos_result.address)
            .unwrap();
        utxos_state_address.unseen_state = utxos_result.utxos;
        UtxosUpdate::from_state(
            &utxos_state_address.seen_state,
            &utxos_state_address.unseen_state,
        )
    }

    pub fn get_current_fees_args(&self) -> CurrentFeesArgs {
        CurrentFeesArgs {
            network: self.management_canister.get_network(),
        }
    }

    pub fn get_current_fee_args(&self, fee_request: FeeRequest) -> CurrentFeeArgs {
        CurrentFeeArgs {
            network: self.management_canister.get_network(),
            fee_request,
        }
    }

    pub fn get_initialization_parameters_args(&self) -> InitializationParametersArgs {
        InitializationParametersArgs {
            key_name: get_key_name_from_network(self.management_canister.get_network()),
            ecdsa_public_key: self.management_canister.get_ecdsa_public_key(),
        }
    }

    /// Initializes the Bitcoin agent by setting its ECDSA public key.
    pub fn initialize(&mut self, ecdsa_public_key: EcdsaPubKey) {
        self.management_canister
            .set_ecdsa_public_key(ecdsa_public_key);
        let main_address = get_main_address(&self.management_canister, &self.main_address_type);
        self.ecdsa_pub_key_addresses = BTreeMap::from([(
            main_address.clone(),
            self.management_canister.get_ecdsa_public_key(),
        )]);
        self.utxos_state_addresses =
            BTreeMap::from([(main_address, UtxosState::new(self.min_confirmations))]);
    }

    /// Returns arguments to send a transaction, transferring the specified Bitcoin amounts to the provided addresses.
    /// When `replaceable` is set to true, the transaction is marked as replaceable using Bitcoin's replace-by-fee (RBF) mechanism.
    /// The `min_confirmations` parameter states that only outputs with at least that many confirmations may be used to construct a transaction.
    /// Note that `min_confirmations` = 0 implies that unconfirmed outputs may be used to create a transaction.
    /// Further note that the set of UTXO is restricted to those in the updated state: If new UTXOs are discovered when calling `peek_utxos_update` (or `peek_balance_update`), these UTXOs will not be spent in any transaction until they are made available by calling `update_state`.
    /// On the other hand, the library is free to choose UTXOs of any managed address when constructing transactions.
    /// Also note that the library verifies if the final fee is at least 1 sat/B.
    pub fn get_multi_transfer_args(
        &self,
        payouts: &BTreeMap<Address, Satoshi>,
        change_address: &Address,
        fee: Fee,
        min_confirmations: u32,
        replaceable: bool,
    ) -> MultiTransferArgs {
        MultiTransferArgs {
            key_name: get_key_name_from_network(self.management_canister.get_network()),
            ecdsa_pub_key_addresses: self.ecdsa_pub_key_addresses.clone(),
            utxos_state_addresses: self.utxos_state_addresses.clone(),
            payouts: payouts.clone(),
            change_address: change_address.clone(),
            fee,
            min_confirmations,
            replaceable,
            network: from_bitcoin_network_to_types_network(self.management_canister.get_network()),
        }
    }

    /// Caches the spent and generated outputs to build valid future transactions even with `min_confirmations = 0`.
    pub fn apply_multi_transfer_result(&mut self, multi_transfer_result: &MultiTransferResult) {
        // Cache the spent outputs to not use them for future transactions.
        multi_transfer_result
            .transaction_info
            .utxos_addresses
            .clone()
            .into_iter()
            .for_each(|(address_using_primitives, utxos)| {
                let address = get_address(address_using_primitives);
                utxos.iter().for_each(|utxo| {
                    self.utxos_state_addresses
                        .get_mut(&address)
                        .unwrap()
                        .spent_state
                        .push(utxo.outpoint.clone())
                })
            });
        // Cache the generated outputs to be able to use them for future transactions.
        multi_transfer_result
            .generated_utxos_addresses
            .clone()
            .into_iter()
            .for_each(|(address_using_primitives, mut utxos)| {
                let address = get_address(address_using_primitives);
                if self.utxos_state_addresses.get(&address).is_none() {
                    self.utxos_state_addresses
                        .insert(address.clone(), UtxosState::new(0));
                }
                let utxos_state_address = self.utxos_state_addresses.get_mut(&address).unwrap();
                utxos_state_address.generated_state.append(&mut utxos);
            })
    }
}

pub async fn multi_transfer_from_args(
    multi_transfer_args: MultiTransferArgs,
) -> Result<MultiTransferResult, MultiTransferError> {
    // When running `cargo test`, `multi_transfer` requires an additional argument that is `BitcoinAgent<ManagementCanisterMock>`.
    // This pattern satisfies the compiler for building and testing.
    #[cfg(test)]
    unreachable!();
    #[cfg(not(test))]
    transaction_management::multi_transfer(multi_transfer_args).await
}

pub async fn get_initialization_parameters_from_args(
    initialization_parameters_args: InitializationParametersArgs,
) -> Result<EcdsaPubKey, ManagementCanisterReject> {
    if initialization_parameters_args
        .ecdsa_public_key
        .public_key
        .is_empty()
    {
        get_btc_ecdsa_public_key(&initialization_parameters_args.key_name).await
    } else {
        Ok(initialization_parameters_args.ecdsa_public_key)
    }
}

/// Modify the provided `GetUtxosResponse` to remove spent UTXOs and add generated UTXOs if using `min_confirmations = 0`.
fn get_utxos_from_args_common(
    address: &Address,
    get_utxos_response: GetUtxosResponse,
    utxos_state: UtxosState,
) -> Result<UtxosResult, GetUtxosError> {
    let utxos = if utxos_state.min_confirmations == 0 {
        let mut utxos: Vec<Utxo> = get_utxos_response.utxos;
        utxos.append(&mut utxos_state.generated_state.clone());
        utxos.retain(|utxo| {
            utxos_state
                .spent_state
                .iter()
                .all(|spent_outpoint| utxo.outpoint != spent_outpoint.clone())
        });
        // Remove any duplicated UTXOs with a possible different height, keeping the UTXO with the heighest height.
        // Likewise if a UTXO was generated at height `n` thanks to a sent transaction, if the transaction is confirmed, the UTXO return by this function won't have its height still be `n` but the actual one.
        let mut utxos_occurrences: HashMap<OutPoint, Utxo> = HashMap::default();
        utxos.into_iter().for_each(|utxo| {
            if let Some(utxo_occurrence) = utxos_occurrences.get(&utxo.outpoint) {
                if utxo.height > utxo_occurrence.height {
                    utxos_occurrences.insert(utxo.outpoint.clone(), utxo);
                }
            } else {
                utxos_occurrences.insert(utxo.outpoint.clone(), utxo);
            }
        });
        utxos_occurrences.values().cloned().collect()
    } else {
        get_utxos_response.utxos
    };

    Ok(UtxosResult {
        address: address.clone(),
        utxos,
        tip_height: get_utxos_response.tip_height,
    })
}

pub async fn get_utxos_from_args(utxos_args: UtxosArgs) -> Result<UtxosResult, GetUtxosError> {
    get_utxos_from_args_common(
        &utxos_args.address,
        get_utxos(
            utxos_args.network,
            &utxos_args.address,
            utxos_args.min_confirmations,
        )
        .await?,
        utxos_args.utxos_state,
    )
}

/// Returns the balance of the given Bitcoin `address` according to `min_confirmations`.
pub async fn get_balance_from_args(utxos_args: UtxosArgs) -> Result<Satoshi, GetUtxosError> {
    Ok(get_balance_from_utxos(
        &get_utxos_from_args(utxos_args).await?.utxos,
    ))
}

/// Returns fees as percentiles in millisatoshis/byte over the last 10,000 transactions.
pub async fn get_current_fees_from_args(
    current_fees_args: CurrentFeesArgs,
) -> Result<Vec<MillisatoshiPerByte>, ManagementCanisterReject> {
    get_current_fees(current_fees_args.network).await
}

/// Returns the fee as a percentile in millisatoshis/byte over the last 10,000 transactions.
pub async fn get_current_fee_from_args(
    current_fee_args: CurrentFeeArgs,
) -> Result<MillisatoshiPerByte, GetCurrentFeeError> {
    get_current_fee(current_fee_args.fee_request, current_fee_args.network).await
}

#[cfg(test)]
impl BitcoinAgent<ManagementCanisterMock> {
    /// Simulates UTXOs retrieval from the Bitcoin network during tests.
    pub fn get_utxos_from_args_test(
        &self,
        utxos_args: UtxosArgs,
    ) -> Result<UtxosResult, GetUtxosError> {
        get_utxos_from_args_common(
            &utxos_args.address,
            self.management_canister
                .internal_get_utxos(&utxos_args.address, utxos_args.min_confirmations),
            utxos_args.utxos_state,
        )
    }

    /// Simulates balance retrieval from the Bitcoin network during tests.
    pub fn get_balance_from_args_test(
        &self,
        utxos_args: UtxosArgs,
    ) -> Result<Satoshi, GetUtxosError> {
        let utxos = self.get_utxos_from_args_test(utxos_args).unwrap().utxos;
        Ok(get_balance_from_utxos(&utxos))
    }

    /// Simulates current fees retrieval from the Bitcoin network during tests.
    pub fn get_current_fees_from_args_test(
        &self,
        _current_fees_args: CurrentFeesArgs,
    ) -> Result<Vec<MillisatoshiPerByte>, ManagementCanisterReject> {
        Ok(self.management_canister.internal_get_current_fees())
    }

    /// Simulates current fee retrieval from the Bitcoin network during tests.
    pub fn get_current_fee_from_args_test(
        &self,
        current_fee_args: CurrentFeeArgs,
    ) -> Result<MillisatoshiPerByte, GetCurrentFeeError> {
        let percentile = evaluate_fee_request(current_fee_args.fee_request)?;
        Ok(self.management_canister.internal_get_current_fees()[percentile])
    }

    /// Simulates initialization parameters retrieval from the management canister during tests.
    pub fn get_initialization_parameters_from_args_test(
        &self,
        initialization_parameters_args: InitializationParametersArgs,
    ) -> Result<EcdsaPubKey, ManagementCanisterReject> {
        Ok(
            if initialization_parameters_args
                .ecdsa_public_key
                .public_key
                .is_empty()
            {
                address_management::tests::get_btc_ecdsa_public_key()
            } else {
                initialization_parameters_args.ecdsa_public_key
            },
        )
    }

    /// Simulates making a multi_transfer on the Bitcoin network during tests.
    pub async fn multi_transfer_from_args_test(
        &mut self,
        multi_transfer_args: MultiTransferArgs,
    ) -> Result<MultiTransferResult, MultiTransferError> {
        // When running `cargo build`, `multi_transfer` doesn't require an additional argument that is `BitcoinAgent<ManagementCanisterMock>`.
        // This pattern satisfies the compiler for building and testing.
        #[cfg(not(test))]
        unreachable!();
        #[cfg(test)]
        transaction_management::multi_transfer(multi_transfer_args, self).await
    }
}

/// Creates a new instance of the Bitcoin agent using the management canister mock.
#[cfg(test)]
pub mod tests {
    use crate::{
        address_management::tests::get_btc_ecdsa_public_key, canister_mock::ManagementCanisterMock,
        AddressType, BitcoinAgent, Network,
    };
    use std::cell::RefCell;

    pub fn new_mock(
        network: &Network,
        main_address_type: &AddressType,
    ) -> BitcoinAgent<ManagementCanisterMock> {
        let ecdsa_public_key = get_btc_ecdsa_public_key();
        let mut bitcoin_agent = BitcoinAgent::new(
            ManagementCanisterMock::new_using_ecdsa_public_key_test(
                *network,
                ecdsa_public_key.clone(),
                *main_address_type,
            ),
            main_address_type,
            0,
        )
        .unwrap();
        bitcoin_agent.initialize(ecdsa_public_key);
        bitcoin_agent
    }

    // Thread local agent to verify library usage pattern.
    thread_local! {
        pub(crate) static MOCK_AGENT: RefCell<BitcoinAgent<ManagementCanisterMock>> = RefCell::new(new_mock(&Network::Regtest, &AddressType::P2pkh));
    }
}
