#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod delegator {
    use accumulator::AccumulatorRef;
    use adder::AdderRef;
    use subber::SubberRef;

    /// Specifies the state of the `delegator` contract.
    ///
    /// In `Adder` state the `delegator` contract will delegate to the `Adder` contract
    /// and in `Subber` state will delegate to the `Subber` contract.
    ///
    /// The initial state is `Adder`.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
    )]
    pub enum Which {
        Adder,
        Subber,
    }

    /// Delegates calls to an `adder` or `subber` contract to mutate
    /// a value in an `accumulator` contract.
    ///
    /// # Note
    ///
    /// In order to instantiate the `delegator` smart contract we first
    /// have to manually put the code of the `accumulator`, `adder`
    /// and `subber` smart contracts, receive their code hashes from
    /// the signalled events and put their code hash into our
    /// `delegator` smart contract.
    ///
    /// The `AccumulatorRef`, `AdderRef` and `SubberRef` are smart contract
    /// reference types that have been automatically generated by ink!.
    #[ink(storage)]
    pub struct Delegator {
        /// Says which of `adder` or `subber` is currently in use.
        which: Which,
        /// The `accumulator` smart contract.
        accumulator: AccumulatorRef,
        /// The `adder` smart contract.
        adder: AdderRef,
        /// The `subber` smart contract.
        subber: SubberRef,
    }

    impl Delegator {
        /// Instantiate a `delegator` contract with the given sub-contract codes.
        #[ink(constructor)]
        pub fn new(
            init_value: i32,
            version: u32,
            accumulator_code_hash: Hash,
            adder_code_hash: Hash,
            subber_code_hash: Hash,
        ) -> Self {
            let total_balance = Self::env().balance();
            let salt = version.to_le_bytes();
            let accumulator = AccumulatorRef::new(init_value)
                .endowment(total_balance / 4)
                .code_hash(accumulator_code_hash)
                .salt_bytes(salt)
                .instantiate()
                .unwrap_or_else(|error| {
                    panic!(
                        "failed at instantiating the Accumulator contract: {:?}",
                        error
                    )
                });
            let adder = AdderRef::new(accumulator.clone())
                .endowment(total_balance / 4)
                .code_hash(adder_code_hash)
                .salt_bytes(salt)
                .instantiate()
                .unwrap_or_else(|error| {
                    panic!("failed at instantiating the Adder contract: {:?}", error)
                });
            let subber = SubberRef::new(accumulator.clone())
                .endowment(total_balance / 4)
                .code_hash(subber_code_hash)
                .salt_bytes(salt)
                .instantiate()
                .unwrap_or_else(|error| {
                    panic!("failed at instantiating the Subber contract: {:?}", error)
                });
            Self {
                which: Which::Adder,
                accumulator,
                adder,
                subber,
            }
        }

        /// Returns the `accumulator` value.
        #[ink(message)]
        pub fn get(&self) -> i32 {
            self.accumulator.get()
        }

        /// Delegates the call to either `Adder` or `Subber`.
        #[ink(message)]
        pub fn change(&mut self, by: i32) {
            match self.which {
                Which::Adder => self.adder.inc(by),
                Which::Subber => self.subber.dec(by),
            }
        }

        /// Switches the `delegator` contract.
        #[ink(message)]
        pub fn switch(&mut self) {
            match self.which {
                Which::Adder => {
                    self.which = Which::Subber;
                }
                Which::Subber => {
                    self.which = Which::Adder;
                }
            }
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test(
            additional_contracts = "accumulator/Cargo.toml adder/Cargo.toml subber/Cargo.toml"
        )]
        async fn e2e_delegator(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // given
            let accumulator_hash: ink_e2e::H256 = client
                .upload(&mut ink_e2e::alice(), accumulator::CONTRACT_PATH, None)
                .await
                .expect("uploading `accumulator` failed")
                .code_hash;

            let adder_hash: ink_e2e::H256 = client
                .upload(&mut ink_e2e::alice(), adder::CONTRACT_PATH, None)
                .await
                .expect("uploading `adder` failed")
                .code_hash;

            let subber_hash: ink_e2e::H256 = client
                .upload(&mut ink_e2e::alice(), subber::CONTRACT_PATH, None)
                .await
                .expect("uploading `subber` failed")
                .code_hash;

            let constructor = delegator::constructors::new(
                1234, // initial value
                1337, // salt
                ink_e2e::utils::runtime_hash_to_ink_hash::<ink::env::DefaultEnvironment>(
                    &accumulator_hash,
                ),
                ink_e2e::utils::runtime_hash_to_ink_hash::<ink::env::DefaultEnvironment>(
                    &adder_hash,
                ),
                ink_e2e::utils::runtime_hash_to_ink_hash::<ink::env::DefaultEnvironment>(
                    &subber_hash,
                ),
            );

            let delegator_acc_id = client
                .instantiate(&mut ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // when
            let value = client
                .call(
                    &mut ink_e2e::bob(),
                    delegator_acc_id.clone(),
                    delegator::messages::get(),
                    0,
                    None,
                )
                .await
                .expect("calling `get` failed")
                .value
                .expect("calling `get` returned a `LangError`");
            assert_eq!(value, 1234);
            let _ = client
                .call(
                    &mut ink_e2e::bob(),
                    delegator_acc_id.clone(),
                    delegator::messages::change(6),
                    0,
                    None,
                )
                .await
                .expect("calling `change` failed");

            // then
            let value = client
                .call(
                    &mut ink_e2e::bob(),
                    delegator_acc_id.clone(),
                    delegator::messages::get(),
                    0,
                    None,
                )
                .await
                .expect("calling `get` failed")
                .value
                .expect("calling `get` returned a `LangError`");
            assert_eq!(value, 1234 + 6);

            // when
            let _ = client
                .call(
                    &mut ink_e2e::bob(),
                    delegator_acc_id.clone(),
                    delegator::messages::switch(),
                    0,
                    None,
                )
                .await
                .expect("calling `switch` failed");
            let _ = client
                .call(
                    &mut ink_e2e::bob(),
                    delegator_acc_id.clone(),
                    delegator::messages::change(3),
                    0,
                    None,
                )
                .await
                .expect("calling `change` failed");

            // then
            let value = client
                .call(
                    &mut ink_e2e::bob(),
                    delegator_acc_id.clone(),
                    delegator::messages::get(),
                    0,
                    None,
                )
                .await
                .expect("calling `get` failed")
                .value
                .expect("calling `get` returned a `LangError`");
            assert_eq!(value, 1234 + 6 - 3);

            Ok(())
        }
    }
}
