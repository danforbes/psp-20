#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract()]
mod psp20 {
    #[cfg(not(feature = "ink-as-dependency"))]
    use ink_core::storage::{collections::HashMap as StorageHashMap, lazy::Lazy};

    /// An ERC-20 implementation written in ink!, the smart contract language for Substrate.
    /// PSPs are [Polkadot Standards Proposals](https://github.com/w3f/PSPs).
    /// Ref: https://github.com/paritytech/ink/blob/master/examples/psp20/lib.rs
    #[ink(storage)]
    pub struct Psp20 {
        /// The total number of tokens that exist.
        total_supply: Lazy<Balance>,
        /// Maps an account to the number of tokens it controls.
        balances: StorageHashMap<AccountId, Balance>,
        /// An allowance gives one account the power to spend a specified amount of tokens from
        /// the balance of another account.
        allowances: StorageHashMap<(AccountId, AccountId), Balance>,
    }

    /// Signifies that the`from` account has sent `value` tokens to the `to` account.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }

    /// Signifies that the `spender` account has been granted the power to spend `value` tokens from
    /// the `owner` account's balance.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        #[ink(topic)]
        value: Balance,
    }

    impl Psp20 {
        /// Create a new PSP-20 token by allocating an initial supply to the account that is
        /// creating the token. Will emit a [`Transfer`] event.
        #[ink(constructor)]
        pub fn new(initial_supply: Balance) -> Self {
            // Get the AccountId of the caller that is creating the token.
            let caller = Self::env().caller();

            // Create a map to track balances; allocate the initial supply to the caller.
            let mut balances = StorageHashMap::new();
            balances.insert(caller, initial_supply);

            // Notify offchain users of the token's creation.
            Self::env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: initial_supply,
            });

            // Return the new PSP-20 instance.
            Self {
                total_supply: Lazy::new(initial_supply),
                balances,
                allowances: StorageHashMap::new(),
            }
        }

        /* PUBLIC API */

        /// Get the total number of tokens that exists.
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            *self.total_supply
        }

        /// Get the number of token in the account for `owner`.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balance_of_or_zero(&owner)
        }

        /// Get the number of tokens that `spender` may spend from the balance of `owner`.
        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowance_of_or_zero(&owner, &spender)
        }

        /// Transfer `value` tokens from the account that is initiating the transfer to the `to`
        /// account. Returns `true` iff the transfer was successful. Will emit a [`Transfer`] event
        /// on success.
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> bool {
            self.transfer_from_to(self.env().caller(), to, value)
        }

        /// Create a new allowance that gives `spender` the power to spend `value` tokens from the
        /// approving account. If an allowance from the approving account to `sender` already
        /// exists, it will be overwritten by the new allowance. Returns `true` iff the approval was
        /// successful. Will emit an [`Approval`] event on success.
        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> bool {
            // Record the new allowance.
            let owner = self.env().caller();
            self.allowances.insert((owner, spender), value);

            // Notify offchain users of the approval and report success.
            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });
            true
        }

        /// Transfer `value` tokens from the `from` account to the `to` account iff there is an
        /// allowance that allows the account that is initiating the transfer to do so. Returns
        /// `true` iff the transfer was successful. Will emit a [`Transfer`] event on success.
        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> bool {
            // Ensure that a sufficient allowance exists.
            let caller = self.env().caller();
            let allowance = self.allowance_of_or_zero(&from, &caller);
            if allowance < value {
                return false;
            }

            // Decrease the value of the allowance and transfer the tokens.
            self.allowances.insert((from, caller), allowance - value);
            self.transfer_from_to(from, to, value)
        }

        /* PRIVATE METHODS */

        fn balance_of_or_zero(&self, owner: &AccountId) -> Balance {
            *self.balances.get(owner).unwrap_or(&0)
        }

        fn allowance_of_or_zero(&self, owner: &AccountId, spender: &AccountId) -> Balance {
            *self.allowances.get(&(*owner, *spender)).unwrap_or(&0)
        }

        fn transfer_from_to(&mut self, from: AccountId, to: AccountId, value: Balance) -> bool {
            // Ensure that the sender has a sufficient balance.
            let from_balance = self.balance_of_or_zero(&from);
            if from_balance < value {
                return false;
            }

            // Update the sender's balance.
            self.balances.insert(from, from_balance - value);

            // Update the receiver's balance.
            let to_balance = self.balance_of_or_zero(&to);
            self.balances.insert(to, to_balance + value);

            // Notify offchain users of the transfer and report success.
            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });
            true
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_core::env;

        type Event = <Psp20 as ::ink_lang::BaseEvent>::Type;

        use ink_lang as ink;

        /// The default constructor does its job.
        #[ink::test]
        fn new_works() {
            // Constructor works.
            let _psp20 = Psp20::new(100);

            // Transfer event triggered during initial construction.
            let emitted_events = env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(1, emitted_events.len());

            assert_transfer_event(emitted_events, 0, 100)
        }

        /// The total supply was applied.
        #[ink::test]
        fn total_supply_works() {
            // Constructor works.
            let psp20 = Psp20::new(100);
            // Transfer event triggered during initial construction.
            assert_transfer_event(env::test::recorded_events(), 0, 100);
            // Get the token total supply.
            assert_eq!(psp20.total_supply(), 100);
        }

        /// Get the actual balance of an account.
        #[ink::test]
        fn balance_of_works() {
            // Constructor works
            let psp20 = Psp20::new(100);
            // Transfer event triggered during initial construction
            assert_transfer_event(env::test::recorded_events(), 0, 100);
            let accounts =
                env::test::default_accounts::<env::DefaultEnvTypes>().expect("Cannot get accounts");
            // Alice owns all the tokens on deployment
            assert_eq!(psp20.balance_of(accounts.alice), 100);
            // Bob does not owns tokens
            assert_eq!(psp20.balance_of(accounts.bob), 0);
        }

        #[ink::test]
        fn transfer_works() {
            // Constructor works.
            let mut psp20 = Psp20::new(100);
            // Transfer event triggered during initial construction.
            assert_transfer_event(env::test::recorded_events(), 0, 100);
            let accounts =
                env::test::default_accounts::<env::DefaultEnvTypes>().expect("Cannot get accounts");

            assert_eq!(psp20.balance_of(accounts.bob), 0);
            // Alice transfers 10 tokens to Bob.
            assert_eq!(psp20.transfer(accounts.bob, 10), true);
            // The second Transfer event takes place.
            assert_transfer_event(env::test::recorded_events(), 1, 10);
            // Bob owns 10 tokens.
            assert_eq!(psp20.balance_of(accounts.bob), 10);
        }

        #[ink::test]
        fn invalid_transfer_should_fail() {
            // Constructor works.
            let mut psp20 = Psp20::new(100);
            // Transfer event triggered during initial construction.
            assert_transfer_event(env::test::recorded_events(), 0, 100);
            let accounts =
                env::test::default_accounts::<env::DefaultEnvTypes>().expect("Cannot get accounts");

            assert_eq!(psp20.balance_of(accounts.bob), 0);
            // Get contract address.
            let callee = env::account_id::<env::DefaultEnvTypes>().unwrap_or([0x0; 32].into());
            // Create call
            let mut data = env::test::CallData::new(env::call::Selector::new([0x00; 4])); // balance_of
            data.push_arg(&accounts.bob);
            // Push the new execution context to set Bob as caller
            assert_eq!(
                env::test::push_execution_context::<env::DefaultEnvTypes>(
                    accounts.bob,
                    callee,
                    1000000,
                    1000000,
                    data
                ),
                ()
            );

            // Bob fails to transfers 10 tokens to Eve.
            assert_eq!(psp20.transfer(accounts.eve, 10), false);
            // Alice owns all the tokens.
            assert_eq!(psp20.balance_of(accounts.alice), 100);
            assert_eq!(psp20.balance_of(accounts.bob), 0);
            assert_eq!(psp20.balance_of(accounts.eve), 0);
        }

        #[ink::test]
        fn transfer_from_works() {
            // Constructor works.
            let mut psp20 = Psp20::new(100);
            // Transfer event triggered during initial construction.
            assert_transfer_event(env::test::recorded_events(), 0, 100);
            let accounts =
                env::test::default_accounts::<env::DefaultEnvTypes>().expect("Cannot get accounts");

            // Bob fails to transfer tokens owned by Alice.
            assert_eq!(psp20.transfer_from(accounts.alice, accounts.eve, 10), false);
            // Alice approves Bob for token transfers on her behalf.
            assert_eq!(psp20.approve(accounts.bob, 10), true);

            // The approve event takes place.
            assert_eq!(env::test::recorded_events().count(), 2);

            // Get contract address.
            let callee = env::account_id::<env::DefaultEnvTypes>().unwrap_or([0x0; 32].into());
            // Create call.
            let mut data = env::test::CallData::new(env::call::Selector::new([0x00; 4])); // balance_of
            data.push_arg(&accounts.bob);
            // Push the new execution context to set Bob as caller.
            assert_eq!(
                env::test::push_execution_context::<env::DefaultEnvTypes>(
                    accounts.bob,
                    callee,
                    1000000,
                    1000000,
                    data
                ),
                ()
            );

            // Bob transfers tokens from Alice to Eve.
            assert_eq!(psp20.transfer_from(accounts.alice, accounts.eve, 10), true);
            // The third event takes place.
            assert_transfer_event(env::test::recorded_events(), 2, 10);
            // Eve owns tokens.
            assert_eq!(psp20.balance_of(accounts.eve), 10);
        }

        /* TEST HELPER FUNCTIONS */

        /// Assert that the element of `raw_events` at `transfer_index` is a [`Transfer`] event and
        /// has the `expected_value`.
        fn assert_transfer_event<I>(raw_events: I, transfer_index: usize, expected_value: u128)
        where
            I: IntoIterator<Item = env::test::EmittedEvent>,
        {
            // Get the specified event and decode it.
            let raw_event = raw_events
                .into_iter()
                .nth(transfer_index)
                .expect(&format!("No event at index {}", transfer_index));
            let event = <Event as scale::Decode>::decode(&mut &raw_event.data[..])
                .expect("Invalid contract Event");

            // Assert that the event is a [`Transfer`] event and has the `expected_value`.
            if let Event::Transfer(transfer) = event {
                assert_eq!(expected_value, transfer.value);
            } else {
                panic!("Expected a Transfer Event")
            }
        }
    }
}
