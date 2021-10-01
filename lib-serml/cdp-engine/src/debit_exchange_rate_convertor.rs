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

use super::*;
use primitives::{Balance, CurrencyId};
use sp_runtime::traits::Convert;
use sp_runtime::FixedPointNumber;
use crate::{
	SetterCurrencyId, GetSetUSDCurrencyId, GetSetEURCurrencyId, GetSetGBPCurrencyId
}

pub struct SetterDebitExchangeRateConvertor<T>(sp_std::marker::PhantomData<T>);

impl<T> Convert<(CurrencyId, CurrencyId, Balance), Balance> for SetterDebitExchangeRateConvertor<T>
where
	T: Config,
{
	fn convert((currency_id, balance): (CurrencyId, Balance)) -> Balance {
		let stable_currency_id = T::SetterCurrencyId::get();
		<Module<T>>::get_debit_exchange_rate(collateral_currency_id, stable_currency_id).saturating_mul_int(balance)
	}
}


pub struct SetDollarDebitExchangeRateConvertor<T>(sp_std::marker::PhantomData<T>);

impl<T> Convert<(CurrencyId, CurrencyId, Balance), Balance> for SetDollarDebitExchangeRateConvertor<T>
where
	T: Config,
{
	fn convert((currency_id, balance): (CurrencyId, Balance)) -> Balance {
		let stable_currency_id = T::GetSetUSDCurrencyId::get();
		<Module<T>>::get_debit_exchange_rate(collateral_currency_id, stable_currency_id).saturating_mul_int(balance)
	}
}


pub struct SetEuroDebitExchangeRateConvertor<T>(sp_std::marker::PhantomData<T>);

impl<T> Convert<(CurrencyId, CurrencyId, Balance), Balance> for SetEuroDebitExchangeRateConvertor<T>
where
	T: Config,
{
	fn convert((currency_id, balance): (CurrencyId, Balance)) -> Balance {
		let stable_currency_id = T::GetSetEURCurrencyId::get();
		<Module<T>>::get_debit_exchange_rate(collateral_currency_id, stable_currency_id).saturating_mul_int(balance)
	}
}


pub struct SetPoundDebitExchangeRateConvertor<T>(sp_std::marker::PhantomData<T>);

impl<T> Convert<(CurrencyId, CurrencyId, Balance), Balance> for SetPoundDebitExchangeRateConvertor<T>
where
	T: Config,
{
	fn convert((currency_id, balance): (CurrencyId, Balance)) -> Balance {
		let stable_currency_id = T::GetSetGBPCurrencyId::get();
		<Module<T>>::get_debit_exchange_rate(collateral_currency_id, stable_currency_id).saturating_mul_int(balance)
	}
}
