//! A smart contract which demonstrates behavior of the `self.env().terminate()`
//! function. It terminates itself once `terminate_me()` is called.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::new_without_default)]

#[ink::contract]
pub mod just_terminates {
    /// No storage is needed for this simple contract.
    #[ink(storage)]
    pub struct JustTerminate {}

    impl JustTerminate {
        /// Creates a new instance of this contract.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        /// Terminates with the caller as beneficiary.
        #[ink(message)]
        pub fn terminate_me(&mut self) {
            self.env().terminate_contract(self.env().caller());
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn terminating_works() {
            // given
            let accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let contract_id = ink::env::test::callee::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                contract_id,
                100,
            );
            let mut contract = JustTerminate::new();

            // when
            let should_terminate = move || contract.terminate_me();

            // then
            ink::env::test::assert_contract_termination::<ink::env::DefaultEnvironment, _>(
                should_terminate,
                accounts.alice,
                100,
            );
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn e2e_contract_terminates(
            mut client: ink_e2e::Client<C, E>,
        ) -> E2EResult<()> {
            // given
            let constructor = contract_terminate::constructors::new();
            let contract_acc_id = client
                .instantiate(&mut ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // when
            let terminate_me = contract_terminate::messages::terminate_me();
            let call_res = client
                .call_raw(
                    &mut ink_e2e::alice(),
                    contract_acc_id.clone(),
                    terminate_me,
                    0,
                    None,
                )
                .await
                .expect("terminate_me messages failed");

            assert!(
                call_res.value.is_empty(),
                "Terminated contract never returns"
            );

            let contains_event = |pallet_name: &str, variant_name: &str| {
                call_res.events.iter().any(|evt| {
                    let evt = evt.unwrap();
                    evt.pallet_name() == pallet_name && evt.variant_name() == variant_name
                })
            };

            // then
            assert!(contains_event("System", "KilledAccount"));
            assert!(contains_event("Balances", "Withdraw"));
            assert!(contains_event("Contracts", "Terminated"));

            Ok(())
        }
    }
}
