// This file is part of Setheum.

// Copyright (C) 2020-2021 Setheum Labs.
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

//! # Prices Module
//!
//! ## Overview
//!
//! The data from Oracle cannot be used in business, prices module will do some
//! process and feed prices for setheum. Process include:
//!   - specify a fixed price for stable currency
//!   - feed price in USD or related price bewteen two currencies
//!   - lock/unlock the price data get from oracle

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use orml_traits::{DataFeeder, DataProvider, MultiCurrency};
use primitives::{currency::GetDecimals, Balance, CurrencyId};
use sp_runtime::{
	traits::{CheckedDiv, CheckedMul},
	FixedPointNumber,
};
use support::{SetheumDexManager, ExchangeRateProvider, Price, PriceProvider};

mod mock;
mod tests;
pub mod weights;

pub use module::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Convert currency amount (Balance) in the setter basket currency aggregation
		/// to price (Price )value (setter stable currency (SETT))
		type Convert: Convert<(Price, Balance), Balance>;

		/// The data source, such as Oracle.
		type Source: DataProvider<CurrencyId, Price> + DataFeeder<CurrencyId, Price, Self::AccountId>;

		#[pallet::constant]
		/// The Setter currency id, it should be SETT in Setheum.
		type GetSetterCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettUSD currency id, it should be USDJ in Setheum.
		type GetSettUSDCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettGBP currency id, it should be GBPJ in Setheum.
		type GetSettGBPCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettEUR currency id, it should be EURJ in Setheum.
		type GetSettEURCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettKWD currency id, it should be KWDJ in Setheum.
		type GetSettKWDCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettJOD currency id, it should be JODJ in Setheum.
		type GetSettJODCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettBHD currency id, it should be BHDJ in Setheum.
		type GetSettBHDCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettKYD currency id, it should be KYDJ in Setheum.
		type GetSettKYDCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettOMR currency id, it should be OMRJ in Setheum.
		type GetSettOMRCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettCHF currency id, it should be CHFJ in Setheum.
		type GetSettCHFCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The SettGIP currency id, it should be GIPJ in Setheum.
		type GetSettGIPCurrencyId: Get<CurrencyId>;

		#[pallet::constant]
		/// The fixed price of SettUSD currency, it should be 1 USD in Setheum.
		type SettUSDFixedPrice: Get<Price>;

		#[pallet::constant]
		/// The stable currency ids
		type StableCurrencyIds: Get<Vec<CurrencyId>>;

		/// The peg currency of a stablecoin.
		type PegCurrencyIds: GetByKey<Self::CurrencyId, Self::FiatCurrencyId>;

		#[pallet::constant]
		/// The list of valid Fiat currency types that define the stablecoin pegs
		type FiatCurrencyIds: Get<Vec<CurrencyId>>;

		/// The origin which may lock and unlock prices feed to system.
		type LockOrigin: EnsureOrigin<Self::Origin>;

		/// DEX provide liquidity info.
		type DEX = SetheumDexManager<Self::AccountId, CurrencyId, Balance>;

		/// Currency provide the total insurance of LPToken.
		type Currency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid fiat currency id
		InvalidFiatCurrencyType,
		/// Invalid stable currency id
		InvalidStableCurrencyType,
		/// Invalid peg pair (peg-to-currency-by-key-pair)
		InvalidPegPair,
		/// Converting Setter basket currency amount to Price has failed
		PriceConvertFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Lock price. \[currency_id, locked_price\]
		LockPrice(CurrencyId, Price),
		/// Unlock price. \[currency_id\]
		UnlockPrice(CurrencyId),
	}

	/// Mapping from currency id to it's locked price
	#[pallet::storage]
	#[pallet::getter(fn locked_price)]
	pub type LockedPrice<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, Price, OptionQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Lock the price and feed it to system.
		///
		/// The dispatch origin of this call must be `LockOrigin`.
		///
		/// - `currency_id`: currency type.
		#[pallet::weight((T::WeightInfo::lock_price(), DispatchClass::Operational))]
		#[transactional]
		pub fn lock_price(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::LockOrigin::ensure_origin(origin)?;
			<Pallet<T> as PriceProvider<CurrencyId>>::lock_price(currency_id);
			Ok(().into())
		}

		/// Unlock the price and get the price from `PriceProvider` again
		///
		/// The dispatch origin of this call must be `LockOrigin`.
		///
		/// - `currency_id`: currency type.
		#[pallet::weight((T::WeightInfo::unlock_price(), DispatchClass::Operational))]
		#[transactional]
		pub fn unlock_price(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResultWithPostInfo {
			T::LockOrigin::ensure_origin(origin)?;
			<Pallet<T> as PriceProvider<CurrencyId>>::unlock_price(currency_id);
			Ok(().into())
		}
	}
}

impl<T: Config> PriceProvider<CurrencyId> for Pallet<T> {
	/// get stablecoin's fiat peg currency type by id
	fn get_peg_currency_by_currency_id(currency_id: Self::CurrencyId) -> Self::FiatCurrencyId {
		T::PegCurrencyIds::get(&currency_id)
	}

	/// get the price of a stablecoin's fiat peg
	fn get_fiat_price(currency_id: CurrencyId) -> Option<Price>{
		ensure!(
			T::StableCurrencyIds::get().contains(&currency_id),
			Error::<T>::InvalidStableCurrencyType,
		);
		let fiat_id = get_peg_currency_by_currency_id(&currency_id);
		ensure!(
			T::PegCurrencyIds::get(&currency_id) == &fiat_id,
			Error::<T>::InvalidPegPair,
		);
		// if locked price exists, return it, otherwise return latest price from oracle.
		Self::locked_price(fiat_id).or_else(|| T::Source::get(&fiat_id));
	}

	fn get_setheum_usd_fixed_price() -> Option<Price>{
		let currency_id = T::GetSettUSDCurrencyId::get();
		let fiat_id = Self::get_peg_currency_by_currency_id(&currency_id);
		let fixed_price = Self::get_fiat_price(&currency_id);
		fixed_price
	}

	fn get_stablecoin_fixed_price(currency_id: CurrencyId) -> Option<Price>{
		ensure!(
			T::StableCurrencyIds::get().contains(&currency_id),
			Error::<T>::InvalidStableCurrencyType,
		);
		let fiat_id = Self::get_peg_currency_by_currency_id(&currency_id);
		let fixed_price = Self::get_fiat_price(&currency_id, &fiat_id);
		fixed_price
	}

	/// get exchange rate between two currency types
	/// Note: this returns the price for 1 basic unit
	fn get_relative_price(base_currency_id: CurrencyId, quote_currency_id: CurrencyId) -> Option<Price> {
		if let (Some(base_price), Some(quote_price)) =
			(Self::get_price(base_currency_id), Self::get_price(quote_currency_id))
		{
			base_price.checked_div(&quote_price)
		} else {
			None
		}
	}

	fn get_coin_to_peg_relative_price(currency_id: CurrencyId) -> Option<Price>{
		ensure!(
			T::StableCurrencyIds::get().contains(&currency_id),
			Error::<T>::InvalidStableCurrencyType,
		);
		let fiat_id = Self::get_peg_currency_by_currency_id(&currency_id);
		let coin_to_peg = Self::get_relative_price(&currency_id, &fiat_id);
		coin_to_peg
	}

	/// aggregate the setter price.
	/// the final price = total_price_of_basket(all currencies prices combined)-
	/// divided by the amount of currencies in the basket.
	fn aggregate_setter_basket(total_basket_worth: Price, currencies_amount: Balance) -> Oprion<Price>{
		let currency_convert = Self::price_try_from_balance(currencies_amount)?;
		let setter_basket_final = total_basket_worth.saturating_div_int(currency_convert);
		setter_basket_final
	}

	/// get the price of a Setter (SETT basket coin - basket of currencies)
	fn get_setter_basket_peg_price() -> Option<Price>{
		/// pegged to US Dollar (USD)
		let peg_one_currency_id: CurrencyId = T::GetSettUSDCurrencyId::get();
		/// pegged to Pound Sterling (GBP)
		let peg_two_currency_id: CurrencyId = T::GetSettGBPCurrencyId::get();
		/// pegged to Euro (EUR)
		let peg_three_currency_id: CurrencyId = T::GetSettEURCurrencyId::get();
		/// pegged to Kuwaiti Dinar (KWD)
		let peg_four_currency_id: CurrencyId = T::GetSettKWDCurrencyId::get();
		/// pegged to Jordanian Dinar (JOD)
		let peg_five_currency_id: CurrencyId = T::GetSettJODCurrencyId::get();
		/// pegged to Bahraini Dirham (BHD)
		let peg_six_currency_id: CurrencyId = T::GetSettBHDCurrencyId::get();
		/// pegged to Cayman Islands Dollar (KYD)
		let peg_seven_currency_id: CurrencyId = T::GetSettKYDCurrencyId::get();
		/// pegged to Omani Riyal (OMR)
		let peg_eight_currency_id: CurrencyId = T::GetSettOMRCurrencyId::get();
		/// pegged to Swiss Franc (CHF)
		let peg_nine_currency_id: CurrencyId = T::GetSettCHFCurrencyId::get();
		/// pegged to Gibraltar Pound (GIP)
		let peg_ten_currency_id: CurrencyId = T::GetSettGIPCurrencyId::get();

		let peg_one_price = Self::get_stablecoin_fixed_price(peg_one_currency_id);
		let peg_two_price = Self::get_stablecoin_fixed_price(peg_two_currency_id);
		let peg_three_price = Self::get_stablecoin_fixed_price(peg_three_currency_id);
		let peg_four_price = Self::get_stablecoin_fixed_price(peg_four_currency_id);
		let peg_five_price = Self::get_stablecoin_fixed_price(peg_five_currency_id);
		let peg_six_price = Self::get_stablecoin_fixed_price(peg_six_currency_id);
		let peg_seven_price = Self::get_stablecoin_fixed_price(peg_seven_currency_id);
		let peg_eight_price = Self::get_stablecoin_fixed_price(peg_eight_currency_id);
		let peg_nine_price = Self::get_stablecoin_fixed_price(peg_nine_currency_id);
		let peg_ten_price = Self::get_stablecoin_fixed_price(peg_ten_currency_id);

		let total_basket_worth: Price = peg_one_price
										+ peg_two_price
										+ peg_three_price
										+ peg_four_price
										+ peg_five_price
										+ peg_six_price
										+ peg_seven_price
										+ peg_eight_price
										+ peg_nine_price
										+ peg_ten_price;
		let currencies_amount: Balance = 10;
		let basket_setter_price = Self::aggregate_setter_basket(total_basket_worth, currencies_amount);

		basket_setter_price
	}

	/// get the price of Setter currency (SETT)
	fn get_setter_price() -> Option<Price>{
		Self::get_setter_basket_peg_price();
	}

	/// get the exchange rate of specific currency to USD
	/// Note: this returns the price for 1 basic unit
	fn get_price(currency_id: CurrencyId) -> Option<Price> {
		let maybe_feed_price = if currency_id == T::GetSettUSDCurrencyId::get() {
			// if is setter stable currency, return fixed price
			Some(T::SettUSDFixedPrice::get())
		} else if let CurrencyId::DEXShare(symbol_0, symbol_1) = currency_id {
			let token_0 = CurrencyId::Token(symbol_0);
			let token_1 = CurrencyId::Token(symbol_1);
			let (pool_0, _) = T::DEX::get_liquidity_pool(token_0, token_1);
			let total_shares = T::Currency::total_issuance(currency_id);

			return {
				if let (Some(ratio), Some(price_0)) = (
					Price::checked_from_rational(pool_0, total_shares),
					Self::get_price(token_0),
				) {
					ratio
						.checked_mul(&price_0)
						.and_then(|n| n.checked_mul(&Price::saturating_from_integer(2)))
				} else {
					None
				}
			};
		} else {
			// if locked price exists, return it, otherwise return latest price from oracle.
			Self::locked_price(currency_id).or_else(|| T::Source::get(&currency_id))
		};
		let maybe_adjustment_multiplier = 10u128.checked_pow(currency_id.decimals());

		if let (Some(feed_price), Some(adjustment_multiplier)) = (maybe_feed_price, maybe_adjustment_multiplier) {
			Price::checked_from_rational(feed_price.into_inner(), adjustment_multiplier)
		} else {
			None
		}
	}

	fn lock_price(currency_id: CurrencyId) {
		// lock price when get valid price from source
		if let Some(val) = T::Source::get(&currency_id) {
			LockedPrice::<T>::insert(currency_id, val);
			<Pallet<T>>::deposit_event(Event::LockPrice(currency_id, val));
		}
	}

	fn unlock_price(currency_id: CurrencyId) {
		LockedPrice::<T>::remove(currency_id);
		<Pallet<T>>::deposit_event(Event::UnlockPrice(currency_id));
	}
}

impl<T: Config> Pallet<T> {
	/// Convert `Balance` to `Price`.
	fn price_try_from_balance(b: Balance) -> result::Result<Price, Error<T>> {
		TryInto::<Price>::try_into(b).map_err(|_| Error::<T>::PriceConvertFailed)
	}

	/// Convert the absolute value of `Price` to `Balance`.
	fn balance_try_from_price_abs(p: Price) -> result::Result<Balance, Error<T>> {
		TryInto::<Balance>::try_into(p.saturating_abs()).map_err(|_| Error::<T>::PriceConvertFailed)
	}
}
