// This file is part of Setheum.

// Copyright (C) 2019-2021 Setheum Labs.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use fixed::{types::extra::U32, FixedU128};
use frame_support::traits::Get;
use frame_system::{
	self as system,
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
		SignedPayload, Signer, SigningTypes, SubmitTransaction,
	},
};
use lite_json::json::JsonValue;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	offchain::{
		http,
		storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
		Duration,
	},
	traits::Zero,
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeDebug,
};
use sp_std::vec::Vec;

use primitives::CurrencyId;

#[cfg(test)]
mod tests;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"socw");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// This pallet's configuration trait
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The overarching dispatch call type.
		type Call: From<Call<Self>>;

    /// Currency IDs for OCW price fetch in Setheum.
		type FetchCurrencyIds: Get<Vec<CurrencyId>>;

		#[pallet::constant]
		/// Native (DNAR) currency Stablecoin currency id
		type GetNativeCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// SettinDes (DRAM) dexer currency id
		type DirhamCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The Setter currency id, it should be SETR in Setheum.
		type SetterCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SetUSD currency id, it should be SETUSD in Setheum.
		type GetSetUSDCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SETEUR currency id, it should be SETEUR in Setheum.
		type GetSetEURCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SETGBP currency id, it should be SETGBP in Setheum.
		type GetSetGBPCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SETCHF currency id, it should be SETCHF in Setheum.
		type GetSetCHFCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SETSAR currency id, it should be SETSAR in Setheum.
		type GetSetSARCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// SettinDes (DRAM) dexer currency id
		type RenBTCCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The Setter Peg Basket currency id, it should be SETRPEG in Setheum.
		type SetterPegCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The Peg Fiat USD currency id, it should be USD in Setheum.
		type GetPegUSDCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The Peg Fiat EUR currency id, it should be EUR in Setheum.
		type GetPegEURCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The Peg Fiat GBP currency id, it should be GBP in Setheum.
		type GetPegGBPCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The Peg Fiat CHF currency id, it should be CHF in Setheum.
		type GetPegCHFCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The Peg Fiat SAR currency id, it should be SAR in Setheum.
		type GetPegSARCurrencyId: Get<CurrencyId>;

		/// A fetch duration period after we fetch prices.
		#[pallet::constant]
		type FetchPeriod: Get<Self::BlockNumber>;

		/// A grace period after we send transaction.
		#[pallet::constant]
		type GracePeriod: Get<Self::BlockNumber>;

		/// Number of blocks of cooldown after unsigned transaction is included.
		#[pallet::constant]
		type UnsignedInterval: Get<Self::BlockNumber>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		fn offchain_worker(block_number: T::BlockNumber) {
      let duration = T::FetchPeriod::get();
      
      // fetch prices if the price fetch duration is reached
      if duration > 0.into() && block_number % duration == 0.into() {
        for (currency_id) in T::FetchCurrencyIds::get().iter() {
          // We are going to send both signed and unsigned transactions
          // depending on the block number.
          let should_send = Self::choose_transaction_type(block_number);
          let res = match should_send {
            TransactionType::Signed => Self::fetch_price_and_send_signed(currency_id),
            TransactionType::UnsignedForAny =>
              Self::fetch_price_and_send_unsigned_for_any_account(currency_id, block_number),
            TransactionType::UnsignedForAll =>
              Self::fetch_price_and_send_unsigned_for_all_accounts(currency_id, block_number),
            TransactionType::Raw => Self::fetch_price_and_send_raw_unsigned(currency_id, block_number),
            TransactionType::None => Ok(()),
          };
          if let Err(e) = res {
            log::error!("Error: {}", e);
          }
        }
      }
		}
	}

	/// A public part of the pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit new price to the list.
		#[pallet::weight(0)]
		pub fn submit_price(origin: OriginFor<T>, price: u32) -> DispatchResultWithPostInfo {
			// Retrieve sender of the transaction.
			let who = ensure_signed(origin)?;
			// Add the price to the on-chain list.
			Self::add_price(who, price);
			Ok(().into())
		}

		/// Submit new price to the list via unsigned transaction.
		#[pallet::weight(0)]
		pub fn submit_price_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			price: u32,
		) -> DispatchResultWithPostInfo {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;
			// Add the price to the on-chain list, but mark it as coming from an empty address.
			Self::add_price(Default::default(), price);
			// now increment the block number at which we expect next unsigned transaction.
			let current_block = <system::Pallet<T>>::block_number();
			<NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn submit_price_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			price_payload: PricePayload<T::Public, T::BlockNumber>,
			_signature: T::Signature,
		) -> DispatchResultWithPostInfo {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;
			// Add the price to the on-chain list, but mark it as coming from an empty address.
			Self::add_price(Default::default(), price_payload.price);
			// now increment the block number at which we expect next unsigned transaction.
			let current_block = <system::Pallet<T>>::block_number();
			<NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());
			Ok(().into())
		}
	}

	/// Events for the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event generated when new price is accepted to contribute to the average.
		/// \[currency_id, price, who\]
		NewPrice(u32, T::AccountId),
	}

  #[pallet::validate_unsigned]
  impl<T: Config> ValidateUnsigned for Pallet<T> {
    type Call = Call<T>;

    /// Validate unsigned call to this module.
    ///
    /// By default unsigned transactions are disallowed, but implementing the validator
    /// here we make sure that some particular calls (the ones produced by offchain worker)
    /// are being whitelisted and marked as valid.
    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
      // Firstly let's check that we call the right function.
      if let Call::submit_price_unsigned_with_signed_payload(ref payload, ref signature) =
        call
      {
        let signature_valid =
          SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
        if !signature_valid {
          return InvalidTransaction::BadProof.into()
        }
        Self::validate_transaction_parameters(&payload.block_number, &payload.price)
      } else if let Call::submit_price_unsigned(block_number, new_price) = call {
        Self::validate_transaction_parameters(block_number, new_price)
      } else {
        InvalidTransaction::Call.into()
      }
    }
  }

  /// A vector of recently submitted prices.
  ///
  /// This is used to calculate average price, should have bounded size.
  #[pallet::storage]
  #[pallet::getter(fn prices)]
  pub(super) type Prices<T: Config> = StorageValue<_, Vec<u32>, ValueQuery>;

  /// Defines the block when next unsigned transaction will be accepted.
  ///
  /// To prevent spam of unsigned (and unpayed!) transactions on the network,
  /// we only allow one transaction every `T::UnsignedInterval` blocks.
  /// This storage entry defines when new transaction is going to be accepted.
  #[pallet::storage]
  #[pallet::getter(fn next_unsigned_at)]
  pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;
}

/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PricePayload<Public, BlockNumber> {
block_number: BlockNumber,
price: u32,
public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for PricePayload<T::Public, T::BlockNumber> {
fn public(&self) -> T::Public {
  self.public.clone()
}
}

enum TransactionType {
Signed,
UnsignedForAny,
UnsignedForAll,
Raw,
None,
}

pub trait FetchPriceFor {
    fn get_price_fetch(currency_id: CurrencyId) -> Result<u32, http::Error>;
}

impl<T: Config> FetchPriceFor for Pallet<T> {
	/// Fetch current price and return the result in cents.
	fn get_price_fetch(currency_id: CurrencyId) -> Result<u32, http::Error> {
    let price = Self::fetch_usd();

		return price
	}
}

impl<T: Config> Pallet<T> {
	/// Chooses which transaction type to send.
	///
	/// This function serves mostly to showcase `StorageValue` helper
	/// and local storage usage.
	///
	/// Returns a type of transaction that should be produced in current run.
	fn choose_transaction_type(block_number: T::BlockNumber) -> TransactionType {
		/// A friendlier name for the error that is going to be returned in case we are in the grace
		/// period.
		const RECENTLY_SENT: () = ();

		// Start off by creating a reference to Local Storage value.
		// Since the local storage is common for all offchain workers, it's a good practice
		// to prepend your entry with the module name.
		let val = StorageValueRef::persistent(b"example_ocw::last_send");
		// The Local Storage is persisted and shared between runs of the offchain workers,
		// and offchain workers may run concurrently. We can use the `mutate` function, to
		// write a storage entry in an atomic fashion. Under the hood it uses `compare_and_set`
		// low-level method of local storage API, which means that only one worker
		// will be able to "acquire a lock" and send a transaction if multiple workers
		// happen to be executed concurrently.
		let res = val.mutate(|last_send: Result<Option<T::BlockNumber>, StorageRetrievalError>| {
			match last_send {
				// If we already have a value in storage and the block number is recent enough
				// we avoid sending another transaction at this time.
				Ok(Some(block)) if block_number < block + T::GracePeriod::get() =>
					Err(RECENTLY_SENT),
				// In every other case we attempt to acquire the lock and send a transaction.
				_ => Ok(block_number),
			}
		});

		// The result of `mutate` call will give us a nested `Result` type.
		// The first one matches the return of the closure passed to `mutate`, i.e.
		// if we return `Err` from the closure, we get an `Err` here.
		// In case we return `Ok`, here we will have another (inner) `Result` that indicates
		// if the value has been set to the storage correctly - i.e. if it wasn't
		// written to in the meantime.
		match res {
			// The value has been set correctly, which means we can safely send a transaction now.
			Ok(block_number) => {
				// Depending if the block is even or odd we will send a `Signed` or `Unsigned`
				// transaction.
				// Note that this logic doesn't really guarantee that the transactions will be sent
				// in an alternating fashion (i.e. fairly distributed). Depending on the execution
				// order and lock acquisition, we may end up for instance sending two `Signed`
				// transactions in a row. If a strict order is desired, it's better to use
				// the storage entry for that. (for instance store both block number and a flag
				// indicating the type of next transaction to send).
				let transaction_type = block_number % 3u32.into();
				if transaction_type == Zero::zero() {
					TransactionType::Signed
				} else if transaction_type == T::BlockNumber::from(1u32) {
					TransactionType::UnsignedForAny
				} else if transaction_type == T::BlockNumber::from(2u32) {
					TransactionType::UnsignedForAll
				} else {
					TransactionType::Raw
				}
			},
			// We are in the grace period, we should not send a transaction this time.
			Err(MutateStorageError::ValueFunctionFailed(RECENTLY_SENT)) => TransactionType::None,
			// We wanted to send a transaction, but failed to write the block number (acquire a
			// lock). This indicates that another offchain worker that was running concurrently
			// most likely executed the same logic and succeeded at writing to storage.
			// Thus we don't really want to send the transaction, knowing that the other run
			// already did.
			Err(MutateStorageError::ConcurrentModification(_)) => TransactionType::None,
		}
	}

	/// A helper function to fetch the price and send signed transaction.
	fn fetch_price_and_send_signed() -> Result<(), &'static str> {
		let signer = Signer::<T, T::AuthorityId>::all_accounts();
		if !signer.can_sign() {
			return Err(
				"No local accounts available. Consider adding one via `author_insertKey` RPC.",
			)?
		}
		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// Using `send_signed_transaction` associated type we create and submit a transaction
		// representing the call, we've just created.
		// Submit signed will return a vector of results for all accounts that were found in the
		// local keystore with expected `KEY_TYPE`.
		let results = signer.send_signed_transaction(|_account| {
			// Received price is wrapped into a call to `submit_price` public function of this pallet.
			// This means that the transaction, when executed, will simply call that function passing
			// `price` as an argument.
			Call::submit_price(price)
		});

		for (acc, res) in &results {
			match res {
				Ok(()) => log::info!("[{:?}] Submitted price of {} cents", acc.id, price),
				Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
			}
		}

		Ok(())
	}

	/// A helper function to fetch the price and send a raw unsigned transaction.
	fn fetch_price_and_send_raw_unsigned(block_number: T::BlockNumber) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction")
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// Received price is wrapped into a call to `submit_price_unsigned` public function of this
		// pallet. This means that the transaction, when executed, will simply call that function
		// passing `price` as an argument.
		let call = Call::submit_price_unsigned(block_number, price);

		// Now let's create a transaction out of this call and submit it to the pool.
		// Here we showcase two ways to send an unsigned transaction / unsigned payload (raw)
		//
		// By default unsigned transactions are disallowed, so we need to whitelist this case
		// by writing `UnsignedValidator`. Note that it's EXTREMELY important to carefuly
		// implement unsigned validation logic, as any mistakes can lead to opening DoS or spam
		// attack vectors. See validation logic docs for more details.
		//
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|()| "Unable to submit unsigned transaction.")?;

		Ok(())
	}

	/// A helper function to fetch the price, sign payload and send an unsigned transaction
	fn fetch_price_and_send_unsigned_for_any_account(
		block_number: T::BlockNumber,
	) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction")
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// -- Sign using any account
		let (_, result) = Signer::<T, T::AuthorityId>::any_account()
			.send_unsigned_transaction(
				|account| PricePayload { price, block_number, public: account.public.clone() },
				|payload, signature| {
					Call::submit_price_unsigned_with_signed_payload(payload, signature)
				},
			)
			.ok_or("No local accounts accounts available.")?;
		result.map_err(|()| "Unable to submit transaction")?;

		Ok(())
	}

	/// A helper function to fetch the price, sign payload and send an unsigned transaction
	fn fetch_price_and_send_unsigned_for_all_accounts(
		block_number: T::BlockNumber,
	) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction")
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// -- Sign using all accounts
		let transaction_results = Signer::<T, T::AuthorityId>::all_accounts()
			.send_unsigned_transaction(
				|account| PricePayload { price, block_number, public: account.public.clone() },
				|payload, signature| {
					Call::submit_price_unsigned_with_signed_payload(payload, signature)
				},
			);
		for (_account_id, result) in transaction_results.into_iter() {
			if result.is_err() {
				return Err("Unable to submit transaction")
			}
		}

		Ok(())
	}

	/// Fetch current price and return the result in cents.
	fn fetch_price() -> Result<u32, http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
		// Initiate an external HTTP GET request.
		// This is using high-level wrappers from `sp_runtime`, for the low-level calls that
		// you can find in `sp_io`. The API is trying to be similar to `reqwest`, but
		// since we are running in a custom WASM execution environment we can't simply
		// import the library here.
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=USD");
		// We set the deadline for sending of the request, note that awaiting response can
		// have a separate deadline. Next we send the request, before that it's also possible
		// to alter request headers or stream body content in case of non-GET requests.
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

		// The request is already being processed by the host, we are free to do anything
		// else in the worker (we can send multiple concurrent requests too).
		// At some point however we probably want to check the response though,
		// so we can block current thread and wait for it to finish.
		// Note that since the request is being driven by the host, we don't have to wait
		// for the request to have it complete, we will just not read the response.
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		// Let's check the status code before we proceed to reading the response.
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}

		// Next we want to fully read the response body and collect it to a vector of bytes.
		// Note that the return object allows you to read the body in chunks as well
		// with a way to control the deadline.
		let body = response.body().collect::<Vec<u8>>();

		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;

		let price = match Self::parse_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Parse the price from the given JSON string using `lite-json`.
	///
	/// Returns `None` when parsing failed or `Some(price in cents)` when parsing is successful.
	fn parse_price(price_str: &str) -> Option<u32> {
		let val = lite_json::parse_json(price_str);
		let price = match val.ok()? {
			JsonValue::Object(obj) => {
				let (_, v) = obj.into_iter().find(|(k, _)| k.iter().copied().eq("USD".chars()))?;
				match v {
					JsonValue::Number(number) => number,
					_ => return None,
				}
			},
			_ => return None,
		};

		let exp = price.fraction_length.checked_sub(2).unwrap_or(0);
		Some(price.integer as u32 * 100 + (price.fraction / 10_u64.pow(exp)) as u32)
	}

	/// Add new price to the list.
	fn add_price(who: T::AccountId, price: u32) {
		log::info!("Adding to the average: {}", price);
		<Prices<T>>::mutate(|prices| {
			const MAX_LEN: usize = 64;

			if prices.len() < MAX_LEN {
				prices.push(price);
			} else {
				prices[price as usize % MAX_LEN] = price;
			}
		});

		let average = Self::average_price()
			.expect("The average is not empty, because it was just mutated; qed");
		log::info!("Current average price is: {}", average);
		// here we are raising the NewPrice event
		Self::deposit_event(Event::NewPrice(price, who));
	}

	/// Calculate current average price.
	fn average_price() -> Option<u32> {
		let prices = <Prices<T>>::get();
		if prices.is_empty() {
			None
		} else {
			Some(prices.iter().fold(0_u32, |a, b| a.saturating_add(*b)) / prices.len() as u32)
		}
	}

	fn validate_transaction_parameters(
		block_number: &T::BlockNumber,
		new_price: &u32,
	) -> TransactionValidity {
		// Now let's check if the transaction has any chance to succeed.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if &next_unsigned_at > block_number {
			return InvalidTransaction::Stale.into()
		}
		// Let's make sure to reject transactions from the future.
		let current_block = <system::Pallet<T>>::block_number();
		if &current_block < block_number {
			return InvalidTransaction::Future.into()
		}

		// We prioritize transactions that are more far away from current average.
		//
		// Note this doesn't make much sense when building an actual oracle, but this example
		// is here mostly to show off offchain workers capabilities, not about building an
		// oracle.
		let avg_price = Self::average_price()
			.map(|price| if &price > new_price { price - new_price } else { new_price - price })
			.unwrap_or(0);

		ValidTransaction::with_tag_prefix("ExampleOffchainWorker")
			// We set base priority to 2**20 and hope it's included before any other
			// transactions in the pool. Next we tweak the priority depending on how much
			// it differs from the current average. (the more it differs the more priority it
			// has).
			.priority(T::UnsignedPriority::get().saturating_add(avg_price as _))
			// This transaction does not require anything else to go before into the pool.
			// In theory we could require `previous_unsigned_at` transaction to go first,
			// but it's not necessary in our case.
			//.and_requires()
			// We set the `provides` tag to be the same as `next_unsigned_at`. This makes
			// sure only one transaction produced after `next_unsigned_at` will ever
			// get to the transaction pool and will end up in the block.
			// We can still have multiple transactions compete for the same "spot",
			// and the one with higher priority will replace other one in the pool.
			.and_provides(next_unsigned_at)
			// The transaction is only valid for next 5 blocks. After that it's
			// going to be revalidated by the pool.
			.longevity(5)
			// It's fine to propagate that transaction to other peers, which means it can be
			// created even by nodes that don't produce blocks.
			// Note that sometimes it's better to keep it for yourself (if you are the block
			// producer), since for instance in some schemes others may copy your solution and
			// claim a reward.
			.propagate(true)
			.build()
	}

  /// FETCH SETCURRENCIES COIN PRICES
  ///
  /// 
	/// Fetch current SETR price and return the result in cents.
	fn fetch_setter() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=EURS&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETUSD price and return the result in cents.
	fn fetch_setusd() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=USDT&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETEUR price and return the result in cents.
	fn fetch_seteur() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=EURS&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETGBP price and return the result in cents.
	fn fetch_setgbp() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=BGBP&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETCHF price and return the result in cents.
	fn fetch_setchf() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=XCHF&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETSAR price and return the result in cents.
	fn fetch_setsar() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=POLNX&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

  /// FETCH SETCURRENCIES COIN PEG PRICES
  ///
  /// 
	/// Fetch current SETR price and return the result in cents.
	fn fetch_setter_basket() -> Result<(u32), http::Error> {
    type Fix = FixedU128<U32>;

		let price1 = Self::fetch_usd();
		let price2 = Self::fetch_eur();
		let price3 = Self::fetch_gbp();
		let price4 = Self::fetch_chf();
		let price5 = Self::fetch_sar();
		let price6 = Self::fetch_eth();
		let price7 = Self::fetch_btc();
    
    let weight_to_price1 = Fix::from_num(price1) / Fix::from_num(100) * Fix::from_num(25);
    let peg_rate1 = weight_to_price1.to_num::<u32>();

    let weight_to_price2 = Fix::from_num(price2) / Fix::from_num(4) - Fix::from_num(1);
    let peg_rate2 = weight_to_price2.to_num::<u32>();

    let weight_to_price3 = Fix::from_num(price3) / Fix::from_num(100) * Fix::from_num(25);
    let peg_rate3 = weight_to_price3.to_num::<u32>();

    let weight_to_price4 = Fix::from_num(price4) / Fix::from_num(4) - Fix::from_num(1);
    let peg_rate4 = weight_to_price4.to_num::<u32>();

    let weight_to_price5 = Fix::from_num(price5) / Fix::from_num(100) * Fix::from_num(25);
    let peg_rate5 = weight_to_price5.to_num::<u32>();

    let weight_to_price6 = Fix::from_num(price6) / Fix::from_num(4) - Fix::from_num(1);
    let peg_rate6 = weight_to_price6.to_num::<u32>();

    let weight_to_price7 = Fix::from_num(price7) / Fix::from_num(100) * Fix::from_num(25);
    let peg_rate7 = weight_to_price7.to_num::<u32>();

    let price_fraction = Fix::from_num(peg_rate1)
      + Fix::from_num(peg_rate2)
      + Fix::from_num(peg_rate3)
      + Fix::from_num(peg_rate4) 
      + Fix::from_num(peg_rate5)
      + Fix::from_num(peg_rate6)
      + Fix::from_num(peg_rate7);

    let price = price_fraction.to_num::<u32>();

		return price
	}

  /// FETCH FIAT CURRENCIES PRICES
  ///
  /// 
	/// Fetch current SETUSD price and return the result in cents.
	fn fetch_usd() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // exchangehost fetch - price
		let request =
			http::Request::get("https://api.exchangerate.host/convert?from=USD&to=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_exchangehost_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETEUR price and return the result in cents.
	fn fetch_eur() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // exchangehost fetch - price
		let request =
			http::Request::get("https://api.exchangerate.host/convert?from=EUR&to=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_exchangehost_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETGBP price and return the result in cents.
	fn fetch_gbp() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // exchangehost fetch - price
		let request =
			http::Request::get("https://api.exchangerate.host/convert?from=GBP&to=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_exchangehost_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETCHF price and return the result in cents.
	fn fetch_chf() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // exchangehost fetch - price
		let request =
			http::Request::get("https://api.exchangerate.host/convert?from=CHF&to=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_exchangehost_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current SETSAR price and return the result in cents.
	fn fetch_sar() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // exchangehost fetch - price
		let request =
			http::Request::get("https://api.exchangerate.host/convert?from=SAR&to=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_exchangehost_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current DNAR price and return the result in cents.
	fn fetch_dinar() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current DRAM price and return the result in cents.
	fn fetch_dirham() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=ETH&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current BTC price and return the result in cents.
	fn fetch_btc() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Fetch current BTC price and return the result in cents.
	fn fetch_eth() -> Result<(u32), http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    // cryptocompare fetch - price
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=ETH&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		let price = match Self::parse_cryptocompare_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			},
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Parse the price from the given JSON string using `lite-json`.
	///
	/// Returns `0` when parsing failed or `price in cents` when parsing is successful.
	fn parse_cryptocompare_price(price_str: &str) -> Option<u32> {
		let val = lite_json::parse_json(price_str);
		let price = match val.ok()? {
			JsonValue::Object(obj) => {
				let (_, v) = obj.into_iter().find(|(k, _)| k.iter().copied().eq("USD".chars()))?;
				match v {
					JsonValue::Number(number) => number,
					_ => return None,
				}
			},
			_ => return None,
		};

		let exp = price.fraction_length.checked_sub(2).unwrap_or(0);
		Some(price.integer as u32 * 100 + (price.fraction / 10_u64.pow(exp)) as u32)
	}

	/// Parse the price from the given JSON string using `lite-json`.
	///
	/// Returns `0` when parsing failed or `price in cents` when parsing is successful.
	fn parse_exchangehost_price(price_str: &str) -> Option<u32> {
		let val = lite_json::parse_json(price_str);
		let price = match val.ok()? {
			JsonValue::Object(obj) => {
				let (_, v) = obj.into_iter().find(|(k, _)| k.iter().copied().eq("result".chars()))?;
				match v {
					JsonValue::Number(number) => number,
					_ => return None,
				}
			},
			_ => return None,
		};

		let exp = price.fraction_length.checked_sub(2).unwrap_or(0);
		Some(price.integer as u32 * 100 + (price.fraction / 10_u64.pow(exp)) as u32)
	}
}
