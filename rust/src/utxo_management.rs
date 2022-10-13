use crate::{
    agent::BitcoinAgent,
    canister_common::{ManagementCanister, GET_UTXOS_COST_CYCLES},
    types::{from_bitcoin_network_to_ic_btc_types_network, GetUtxosResponse},
    AddressNotTracked, BalanceUpdate, GetUtxosError, Satoshi, Utxo, UtxosUpdate,
    MIN_CONFIRMATIONS_UPPER_BOUND,
};
use bitcoin::{Address, Network};
use ic_btc_types::{
    GetUtxosRequest,
    UtxosFilter::{MinConfirmations, Page},
};
use ic_cdk::{api::call::call_with_payment, export::Principal};

/// Returns the actual UTXOs of the given Bitcoin `address` according to `min_confirmations`.
pub(crate) async fn get_utxos(
    network: Network,
    address: &Address,
    min_confirmations: u32,
) -> Result<GetUtxosResponse, GetUtxosError> {
    if min_confirmations > MIN_CONFIRMATIONS_UPPER_BOUND {
        return Err(GetUtxosError::MinConfirmationsTooHigh);
    }
    let mut filter = Some(MinConfirmations(min_confirmations));
    let mut utxos = vec![];
    let tip_height;
    loop {
        let res: Result<(ic_btc_types::GetUtxosResponse,), _> = call_with_payment(
            Principal::management_canister(),
            "bitcoin_get_utxos",
            (GetUtxosRequest {
                address: address.to_string(),
                network: from_bitcoin_network_to_ic_btc_types_network(network),
                filter,
            },),
            GET_UTXOS_COST_CYCLES,
        )
        .await;

        match res {
            Ok((mut get_utxos_response,)) => {
                utxos.append(&mut get_utxos_response.utxos);
                if get_utxos_response.next_page.is_none() {
                    tip_height = get_utxos_response.tip_height;
                    break;
                } else {
                    filter = get_utxos_response.next_page.map(Page);
                }
            }

            // The call to `get_utxos` was rejected for a given reason (e.g., not enough cycles were attached to the call).
            Err((rejection_code, message)) => {
                return Err(GetUtxosError::ManagementCanisterReject(
                    rejection_code,
                    message,
                ))
            }
        }
    }

    Ok(GetUtxosResponse { utxos, tip_height })
}

/// Returns the difference between the current UTXO state and the last seen state for this address.
/// The last seen state for an address is updated to the current unseen state by calling `update_state` or implicitly when invoking `get_utxos_update`.
/// If there are no changes to the UTXO set since the last call, the returned `UtxosUpdate` will be identical.
pub(crate) fn peek_utxos_update<C: ManagementCanister>(
    bitcoin_agent: &BitcoinAgent<C>,
    address: &Address,
) -> Result<UtxosUpdate, AddressNotTracked> {
    if !bitcoin_agent.utxos_state_addresses.contains_key(address) {
        return Err(AddressNotTracked);
    }
    let utxos_state_address = bitcoin_agent.utxos_state_addresses.get(address).unwrap();
    Ok(UtxosUpdate::from_state(
        &utxos_state_address.seen_state,
        &utxos_state_address.unseen_state,
    ))
}

/// Updates the state of the `BitcoinAgent` for the given `address`.
/// This function doesn't invoke a Bitcoin integration API function.
pub(crate) fn update_state<C: ManagementCanister>(
    bitcoin_agent: &mut BitcoinAgent<C>,
    address: &Address,
) -> Result<(), AddressNotTracked> {
    if !bitcoin_agent.utxos_state_addresses.contains_key(address) {
        return Err(AddressNotTracked);
    }
    let unseen_state = bitcoin_agent.utxos_state_addresses[address]
        .unseen_state
        .clone();
    bitcoin_agent
        .utxos_state_addresses
        .get_mut(address)
        .unwrap()
        .seen_state = unseen_state;
    Ok(())
}

/// Returns the difference in the set of UTXOs of an address controlled by the `BitcoinAgent` between the current state and the seen state when the function was last called, considering only UTXOs with the number of confirmations specified when adding the given address.
/// The returned `UtxosUpdate` contains the information which UTXOs were added and removed. If the function is called for the first time, the current set of UTXOs is returned.
/// Note that the function changes the state of the `BitcoinAgent`: A subsequent call will return changes to the UTXO set that have occurred since the last call.
pub(crate) fn get_utxos_update<C: ManagementCanister>(
    bitcoin_agent: &mut BitcoinAgent<C>,
    address: &Address,
) -> Result<UtxosUpdate, AddressNotTracked> {
    let utxos_update = peek_utxos_update(bitcoin_agent, address)?;
    update_state(bitcoin_agent, address).unwrap();
    Ok(utxos_update)
}

/// Returns the total value of a UTXOs set.
pub(crate) fn get_balance_from_utxos(utxos: &[Utxo]) -> Satoshi {
    utxos.iter().map(|utxo| utxo.value).sum()
}

/// Returns the difference between the current balance state and the last seen state for this address.
/// The last seen state for an address is updated to the current unseen state by calling `update_state` or implicitly when invoking `get_balance_update`.
/// If there are no changes to the balance since the last call, the returned `BalanceUpdate` will be identical.
pub(crate) fn peek_balance_update<C: ManagementCanister>(
    bitcoin_agent: &BitcoinAgent<C>,
    address: &Address,
) -> Result<BalanceUpdate, AddressNotTracked> {
    let utxos_update = peek_utxos_update(bitcoin_agent, address)?;
    Ok(BalanceUpdate::from(utxos_update))
}

/// Returns the difference in the balance of an address controlled by the `BitcoinAgent` between the current state and the seen state when the function was last called, considering only transactions with the specified number of confirmations.
/// The returned `BalanceUpdate` contains the information on how much balance was added and subtracted in total. If the function is called for the first time, the current balance of the address is returned.
/// It is equivalent to calling `get_utxos_update` and summing up the balances in the returned UTXOs.
pub(crate) fn get_balance_update<C: ManagementCanister>(
    bitcoin_agent: &mut BitcoinAgent<C>,
    address: &Address,
) -> Result<BalanceUpdate, AddressNotTracked> {
    let utxos_update = get_utxos_update(bitcoin_agent, address)?;
    Ok(BalanceUpdate::from(utxos_update))
}

/// Returns whether or not a given UTXO has been confirmed `min_confirmations` times according to current `tip_height`.
pub(crate) fn has_utxo_min_confirmations(
    utxo: &Utxo,
    tip_height: u32,
    min_confirmations: u32,
) -> bool {
    utxo.height <= tip_height + 1 - min_confirmations
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        agent,
        agent::tests::MOCK_AGENT,
        canister_mock,
        canister_mock::{
            get_init_balance_update, get_init_utxos, get_init_utxos_update, ManagementCanisterMock,
        },
        AddressType, BalanceUpdate, Network, OutPoint,
    };

    /// Check that `get_utxos` returns the correct address' UTXOs according to `min_confirmations`.
    #[test]
    fn check_get_utxos() {
        let bitcoin_agent = agent::tests::new_mock(&Network::Regtest, &AddressType::P2pkh);
        let init_utxos = get_init_utxos();
        let canister_bitcoin_address = &bitcoin_agent.get_main_address();

        (0..=2).for_each(|min_confirmations| {
            let utxos = canister_mock::get_utxos(
                &bitcoin_agent,
                canister_bitcoin_address,
                min_confirmations,
            );
            let expected_utxos = if min_confirmations < 2 {
                init_utxos.clone()
            } else {
                vec![]
            };
            assert_eq!(utxos, expected_utxos);
        });
    }

    /// Check that `peek_utxos_update` returns the correct `UtxosUpdate` associated with the Bitcoin agent's main address.
    #[test]
    fn check_peek_utxos_update() {
        let mut bitcoin_agent = agent::tests::new_mock(&Network::Regtest, &AddressType::P2pkh);
        let utxos_update = get_init_utxos_update();
        let canister_bitcoin_address = &bitcoin_agent.get_main_address();
        apply_utxos_pattern(&mut bitcoin_agent, canister_bitcoin_address);

        for _ in 0..=1 {
            assert_eq!(
                bitcoin_agent.peek_utxos_update(canister_bitcoin_address),
                Ok(utxos_update.clone())
            );
        }
    }

    /// Check that `update_state` updates the Bitcoin agent's state according to its main address.
    #[test]
    fn check_update_state() {
        let mut bitcoin_agent = agent::tests::new_mock(&Network::Regtest, &AddressType::P2pkh);
        let utxos_update = get_init_utxos_update();
        let canister_bitcoin_address = &bitcoin_agent.get_main_address();
        apply_utxos_pattern(&mut bitcoin_agent, canister_bitcoin_address);

        assert_eq!(
            bitcoin_agent.peek_utxos_update(canister_bitcoin_address),
            Ok(utxos_update),
            "Wrong value returned by peek_utxos_update (1)."
        );

        let added_utxo = Utxo {
            outpoint: OutPoint {
                txid: vec![1; 32],
                vout: 0,
            },
            value: 42_000,
            height: MIN_CONFIRMATIONS_UPPER_BOUND + 1,
        };
        bitcoin_agent
            .management_canister
            .utxos_addresses
            .get_mut(&bitcoin_agent.get_main_address())
            .unwrap()
            .push(added_utxo.clone());
        bitcoin_agent.management_canister.tip_height += 1;

        assert_eq!(
            update_state(&mut bitcoin_agent, canister_bitcoin_address),
            Ok(()),
            "Wrong value returned by update_state."
        );

        apply_utxos_pattern(&mut bitcoin_agent, canister_bitcoin_address);

        let new_utxos_update = UtxosUpdate {
            added_utxos: vec![added_utxo],
            removed_utxos: vec![],
        };
        assert_eq!(
            bitcoin_agent.peek_utxos_update(canister_bitcoin_address),
            Ok(new_utxos_update),
            "Wrong value returned by peek_utxos_update (2)."
        );

        assert_eq!(bitcoin_agent.update_state(canister_bitcoin_address), Ok(()));

        assert_eq!(
            bitcoin_agent.peek_utxos_update(canister_bitcoin_address),
            Ok(UtxosUpdate::new())
        );
    }

    /// Check that `get_utxos_update` returns the correct `UtxosUpdate` associated with the Bitcoin agent main address.
    #[test]
    fn check_get_utxos_update() {
        let mut bitcoin_agent = agent::tests::new_mock(&Network::Regtest, &AddressType::P2pkh);
        let utxos_update = get_init_utxos_update();
        let canister_bitcoin_address = &bitcoin_agent.get_main_address();
        apply_utxos_pattern(&mut bitcoin_agent, canister_bitcoin_address);

        assert_eq!(
            bitcoin_agent.get_utxos_update(canister_bitcoin_address),
            Ok(utxos_update)
        );

        assert_eq!(
            bitcoin_agent.get_utxos_update(canister_bitcoin_address),
            Ok(UtxosUpdate::new())
        );
    }

    /// Check that `get_balance` returns the correct address' balance according to `min_confirmations`.
    #[test]
    fn check_get_balance() {
        let bitcoin_agent = agent::tests::new_mock(&Network::Regtest, &AddressType::P2pkh);
        let utxos = get_init_utxos();
        let init_balance = get_balance_from_utxos(&utxos);
        let canister_bitcoin_address = &bitcoin_agent.get_main_address();

        (0..=2).for_each(|min_confirmations| {
            let balance = canister_mock::get_balance(
                &bitcoin_agent,
                canister_bitcoin_address,
                min_confirmations,
            );
            let expected_balance = if min_confirmations < 2 {
                init_balance
            } else {
                0
            };
            assert_eq!(balance, expected_balance);
        });
    }

    /// Check that `peek_balance_update` returns the correct `BalanceUpdate` associated with the Bitcoin agent's main address.
    #[test]
    fn check_peek_balance_update() {
        let mut bitcoin_agent = agent::tests::new_mock(&Network::Regtest, &AddressType::P2pkh);
        let balance_update = get_init_balance_update();
        let canister_bitcoin_address = &bitcoin_agent.get_main_address();
        apply_utxos_pattern(&mut bitcoin_agent, canister_bitcoin_address);

        for _ in 0..=1 {
            assert_eq!(
                bitcoin_agent.peek_balance_update(canister_bitcoin_address),
                Ok(balance_update.clone())
            );
        }
    }

    /// Check that `get_balance_update` returns the correct `BalanceUpdate` associated with the Bitcoin agent main address.
    #[test]
    fn check_get_balance_update() {
        let mut bitcoin_agent = agent::tests::new_mock(&Network::Regtest, &AddressType::P2pkh);
        let balance_update = get_init_balance_update();
        let canister_bitcoin_address = &bitcoin_agent.get_main_address();
        apply_utxos_pattern(&mut bitcoin_agent, canister_bitcoin_address);

        assert_eq!(
            bitcoin_agent.get_balance_update(canister_bitcoin_address),
            Ok(balance_update)
        );

        assert_eq!(
            bitcoin_agent.get_balance_update(canister_bitcoin_address),
            Ok(BalanceUpdate::new())
        );
    }

    /// Apply update following the same pattern a canister developer will use.
    pub(crate) fn apply_utxos_pattern(
        bitcoin_agent: &mut BitcoinAgent<ManagementCanisterMock>,
        address: &Address,
    ) {
        let utxos_args = bitcoin_agent.get_utxos_args(address, 0);
        let utxos_result = bitcoin_agent
            .get_utxos_from_args_test(utxos_args)
            .expect("Error while getting UTXOs result.");
        let _utxos_update = bitcoin_agent.apply_utxos(utxos_result);
    }

    /// We need to test library usage with thread_local agents as a canister developer would do.
    #[test]
    fn test_thread_local_peek_utxos_update() {
        // Build args.
        let address = MOCK_AGENT.with(|a| a.borrow().get_main_address());
        let args = MOCK_AGENT.with(|a| a.borrow().get_utxos_args(&address, 1));
        let utxos = MOCK_AGENT.with(|a| a.borrow().get_utxos_from_args_test(args));
        let utxos = utxos.expect("Error while getting UTXOs result.");

        // Update agent state.
        let result = MOCK_AGENT.with(|a| a.borrow_mut().apply_utxos(utxos));
        assert!(!result.added_utxos.is_empty());
        let utxos_update_init = get_init_utxos_update();
        assert_eq!(utxos_update_init, result);

        // Call peek_utxos_update.
        let result = MOCK_AGENT.with(|a| a.borrow().peek_utxos_update(&address));
        let result = result.unwrap();
        let utxos_update = get_init_utxos_update();
        assert_eq!(utxos_update, result);
    }
}
