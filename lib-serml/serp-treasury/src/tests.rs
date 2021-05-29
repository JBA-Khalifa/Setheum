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

//! Unit tests for the settmint treasury module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};
use sp_runtime::traits::BadOrigin;

#[test]
fn surplus_pool_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SerpTreasuryModule::surplus_pool(), 0);
		assert_ok!(Currencies::deposit(
			GetStableCurrencyId::get(),
			&SerpTreasuryModule::account_id(),
			500
		));
		assert_eq!(SerpTreasuryModule::surplus_pool(), 500);
	});
}

#[test]
fn total_reserves_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 0);
		assert_ok!(Currencies::deposit(BTC, &SerpTreasuryModule::account_id(), 10));
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 10);
	});
}

#[test]
fn on_system_standard_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SerpTreasuryModule::standard_pool(), 0);
		assert_ok!(SerpTreasuryModule::on_system_standard(1000));
		assert_eq!(SerpTreasuryModule::standard_pool(), 1000);
		assert_noop!(
			SerpTreasuryModule::on_system_standard(Balance::max_value()),
			Error::<Runtime>::StandardPoolOverflow,
		);
	});
}

#[test]
fn on_system_surplus_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 0);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 0);
		assert_ok!(SerpTreasuryModule::on_system_surplus(1000));
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 1000);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 1000);
	});
}

#[test]
fn offset_surplus_and_standard_on_finalize_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 0);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 0);
		assert_eq!(SerpTreasuryModule::standard_pool(), 0);
		assert_ok!(SerpTreasuryModule::on_system_surplus(1000));
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 1000);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 1000);
		SerpTreasuryModule::on_finalize(1);
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 1000);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 1000);
		assert_eq!(SerpTreasuryModule::standard_pool(), 0);
		assert_ok!(SerpTreasuryModule::on_system_standard(300));
		assert_eq!(SerpTreasuryModule::standard_pool(), 300);
		SerpTreasuryModule::on_finalize(2);
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 700);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 700);
		assert_eq!(SerpTreasuryModule::standard_pool(), 0);
		assert_ok!(SerpTreasuryModule::on_system_standard(800));
		assert_eq!(SerpTreasuryModule::standard_pool(), 800);
		SerpTreasuryModule::on_finalize(3);
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 0);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 0);
		assert_eq!(SerpTreasuryModule::standard_pool(), 100);
	});
}

#[test]
fn issue_standard_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Currencies::free_balance(USDJ, &ALICE), 1000);
		assert_eq!(SerpTreasuryModule::standard_pool(), 0);

		assert_ok!(SerpTreasuryModule::issue_standard(&ALICE, 1000, true));
		assert_eq!(Currencies::free_balance(USDJ, &ALICE), 2000);
		assert_eq!(SerpTreasuryModule::standard_pool(), 0);

		assert_ok!(SerpTreasuryModule::issue_standard(&ALICE, 1000, false));
		assert_eq!(Currencies::free_balance(USDJ, &ALICE), 3000);
		assert_eq!(SerpTreasuryModule::standard_pool(), 1000);
	});
}

#[test]
fn burn_standard_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Currencies::free_balance(USDJ, &ALICE), 1000);
		assert_eq!(SerpTreasuryModule::standard_pool(), 0);
		assert_ok!(SerpTreasuryModule::burn_standard(&ALICE, 300));
		assert_eq!(Currencies::free_balance(USDJ, &ALICE), 700);
		assert_eq!(SerpTreasuryModule::standard_pool(), 0);
	});
}

#[test]
fn deposit_surplus_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Currencies::free_balance(USDJ, &ALICE), 1000);
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 0);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 0);
		assert_ok!(SerpTreasuryModule::deposit_surplus(&ALICE, 300));
		assert_eq!(Currencies::free_balance(USDJ, &ALICE), 700);
		assert_eq!(Currencies::free_balance(USDJ, &SerpTreasuryModule::account_id()), 300);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 300);
	});
}

#[test]
fn deposit_reserve_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 0);
		assert_eq!(Currencies::free_balance(BTC, &SerpTreasuryModule::account_id()), 0);
		assert_eq!(Currencies::free_balance(BTC, &ALICE), 1000);
		assert_eq!(SerpTreasuryModule::deposit_reserve(&ALICE, BTC, 10000).is_ok(), false);
		assert_ok!(SerpTreasuryModule::deposit_reserve(&ALICE, BTC, 500));
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 500);
		assert_eq!(Currencies::free_balance(BTC, &SerpTreasuryModule::account_id()), 500);
		assert_eq!(Currencies::free_balance(BTC, &ALICE), 500);
	});
}

#[test]
fn withdraw_reserve_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(SerpTreasuryModule::deposit_reserve(&ALICE, BTC, 500));
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 500);
		assert_eq!(Currencies::free_balance(BTC, &SerpTreasuryModule::account_id()), 500);
		assert_eq!(Currencies::free_balance(BTC, &BOB), 1000);
		assert_eq!(SerpTreasuryModule::withdraw_reserve(&BOB, BTC, 501).is_ok(), false);
		assert_ok!(SerpTreasuryModule::withdraw_reserve(&BOB, BTC, 400));
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 100);
		assert_eq!(Currencies::free_balance(BTC, &SerpTreasuryModule::account_id()), 100);
		assert_eq!(Currencies::free_balance(BTC, &BOB), 1400);
	});
}

#[test]
fn get_total_reserves_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(SerpTreasuryModule::deposit_reserve(&ALICE, BTC, 500));
		assert_eq!(SerpTreasuryModule::get_total_reserves(BTC), 500);
	});
}

#[test]
fn get_standard_proportion_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			SerpTreasuryModule::get_standard_proportion(100),
			Ratio::saturating_from_rational(100, Currencies::total_issuance(USDJ))
		);
	});
}

#[test]
fn swap_reserve_not_in_auction_with_exact_stable_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(DEXModule::add_liquidity(
			Origin::signed(ALICE),
			BTC,
			USDJ,
			100,
			1000,
			false
		));
		assert_eq!(SerpTreasuryModule::total_reserves_not_in_auction(BTC), 0);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 0);
		assert_ok!(SerpTreasuryModule::deposit_reserve(&BOB, BTC, 100));
		assert_eq!(SerpTreasuryModule::total_reserves_not_in_auction(BTC), 100);
		assert_noop!(
			SerpTreasuryModule::swap_reserve_not_in_auction_with_exact_stable(BTC, 499, 101, None),
			Error::<Runtime>::ReserveNotEnough,
		);

		assert_ok!(SerpTreasuryModule::swap_reserve_not_in_auction_with_exact_stable(
			BTC, 499, 100, None
		));
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 0);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 499);
	});
}

#[test]
fn swap_exact_reserve_in_auction_to_stable_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(DEXModule::add_liquidity(
			Origin::signed(ALICE),
			BTC,
			USDJ,
			100,
			1000,
			false
		));
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 0);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 0);
		assert_ok!(SerpTreasuryModule::deposit_reserve(&BOB, BTC, 100));
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 100);
		assert_noop!(
			SerpTreasuryModule::swap_exact_reserve_in_auction_to_stable(BTC, 100, 500, None),
			Error::<Runtime>::ReserveNotEnough,
		);
		assert_ok!(SerpTreasuryModule::create_reserve_auctions(
			BTC, 100, 1000, ALICE, true
		));
		assert_eq!(TOTAL_RESERVE_IN_AUCTION.with(|v| *v.borrow_mut()), 100);

		assert_ok!(SerpTreasuryModule::swap_exact_reserve_in_auction_to_stable(
			BTC, 100, 500, None
		));
		assert_eq!(SerpTreasuryModule::total_reserves(BTC), 0);
		assert_eq!(SerpTreasuryModule::surplus_pool(), 500);
	});
}

#[test]
fn create_reserve_auctions_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Currencies::deposit(BTC, &SerpTreasuryModule::account_id(), 10000));
		assert_eq!(SerpTreasuryModule::reserve_auction_maximum_size(BTC), 0);
		assert_noop!(
			SerpTreasuryModule::create_reserve_auctions(BTC, 10001, 1000, ALICE, true),
			Error::<Runtime>::ReserveNotEnough,
		);

		// without reserve auction maximum size
		assert_ok!(SerpTreasuryModule::create_reserve_auctions(
			BTC, 1000, 1000, ALICE, true
		));
		assert_eq!(TOTAL_RESERVE_AUCTION.with(|v| *v.borrow_mut()), 1);
		assert_eq!(TOTAL_RESERVE_IN_AUCTION.with(|v| *v.borrow_mut()), 1000);

		// set reserve auction maximum size
		assert_ok!(SerpTreasuryModule::set_reserve_auction_maximum_size(
			Origin::signed(1),
			BTC,
			300
		));

		// amount < reserve auction maximum size
		// auction + 1
		assert_ok!(SerpTreasuryModule::create_reserve_auctions(
			BTC, 200, 1000, ALICE, true
		));
		assert_eq!(TOTAL_RESERVE_AUCTION.with(|v| *v.borrow_mut()), 2);
		assert_eq!(TOTAL_RESERVE_IN_AUCTION.with(|v| *v.borrow_mut()), 1200);

		// not exceed lots count cap
		// auction + 4
		assert_ok!(SerpTreasuryModule::create_reserve_auctions(
			BTC, 1000, 1000, ALICE, true
		));
		assert_eq!(TOTAL_RESERVE_AUCTION.with(|v| *v.borrow_mut()), 6);
		assert_eq!(TOTAL_RESERVE_IN_AUCTION.with(|v| *v.borrow_mut()), 2200);

		// exceed lots count cap
		// auction + 5
		assert_ok!(SerpTreasuryModule::create_reserve_auctions(
			BTC, 2000, 1000, ALICE, true
		));
		assert_eq!(TOTAL_RESERVE_AUCTION.with(|v| *v.borrow_mut()), 11);
		assert_eq!(TOTAL_RESERVE_IN_AUCTION.with(|v| *v.borrow_mut()), 4200);
	});
}

#[test]
fn auction_surplus_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(SerpTreasuryModule::auction_surplus(Origin::signed(5), 100), BadOrigin,);
		assert_noop!(
			SerpTreasuryModule::auction_surplus(Origin::signed(1), 100),
			Error::<Runtime>::SurplusPoolNotEnough,
		);
		assert_ok!(SerpTreasuryModule::on_system_surplus(100));
		assert_eq!(TOTAL_SURPLUS_AUCTION.with(|v| *v.borrow_mut()), 0);
		assert_ok!(SerpTreasuryModule::auction_surplus(Origin::signed(1), 100));
		assert_eq!(TOTAL_SURPLUS_AUCTION.with(|v| *v.borrow_mut()), 1);
	});
}

#[test]
fn auction_standard_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(SerpTreasuryModule::auction_standard(Origin::signed(5), 100, 200), BadOrigin,);
		assert_noop!(
			SerpTreasuryModule::auction_standard(Origin::signed(1), 100, 200),
			Error::<Runtime>::StandardPoolNotEnough,
		);
		assert_ok!(SerpTreasuryModule::on_system_standard(100));
		assert_eq!(TOTAL_STANDARD_AUCTION.with(|v| *v.borrow_mut()), 0);
		assert_ok!(SerpTreasuryModule::auction_standard(Origin::signed(1), 100, 200));
		assert_eq!(TOTAL_STANDARD_AUCTION.with(|v| *v.borrow_mut()), 1);
	});
}

#[test]
fn set_reserve_auction_maximum_size_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_eq!(SerpTreasuryModule::reserve_auction_maximum_size(BTC), 0);
		assert_noop!(
			SerpTreasuryModule::set_reserve_auction_maximum_size(Origin::signed(5), BTC, 200),
			BadOrigin
		);
		assert_ok!(SerpTreasuryModule::set_reserve_auction_maximum_size(
			Origin::signed(1),
			BTC,
			200
		));

		let update_reserve_auction_maximum_size_event =
			Event::serp_treasury(crate::Event::ReserveAuctionMaximumSizeUpdated(BTC, 200));
		assert!(System::events()
			.iter()
			.any(|record| record.event == update_reserve_auction_maximum_size_event));
	});
}