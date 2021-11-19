#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Compact, Decode, Encode};
use frame_support::pallet_prelude::InvalidTransaction;
pub use frame_support::{
	construct_runtime, log, parameter_types,
	traits::{
		Contains, ContainsLengthBound, Currency as PalletCurrency, EnsureOrigin, Everything, Get, Imbalance,
		InstanceFilter, IsSubType, IsType, KeyOwnerProofSystem, LockIdentifier, Nothing, OnUnbalanced, Randomness,
		SortedMembers, U128CurrencyToVote, WithdrawReasons,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	PalletId, RuntimeDebug, StorageValue,
};
use sp_std::prelude::*;
use sp_core::{
	crypto::KeyTypeId,
	u32_trait::{_2, _3, _4},
	H160, OpaqueMetadata, Decode,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, BadOrigin, BlakeTwo256, Block as BlockT, Convert, SaturatedConversion, StaticLookup,
	},
	transaction_validity::{TransactionSource, TransactionValidity, TransactionPriority},
	ApplyExtrinsicResult, DispatchResult, FixedPointNumber, curve::PiecewiseLinear,
};
use sp_runtime::traits::{
	NumberFor,
	Zero,
	SaturatedConversion,
	OpaqueKeys,
};
pub use sp_runtime::{
	Perbill, Percent, Permill, Perquintill,
	DispatchResult,
};
use sp_api::impl_runtime_apis;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_grandpa::fg_primitives;
use frame_election_provider_support::onchain;
pub use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
pub use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;

use evm::Runtime;
use module_support::TransactionPayment;
use frame_system::Call;
use frame_system::Event;

use sp_version::RuntimeVersion;
#[cfg(feature = "std")]
use sp_version::NativeVersion;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use pallet_timestamp::Call as TimestampCall;
pub use pallet_balances::Call as BalancesCall;

pub use frame_system::{ensure_root, EnsureOneOf, EnsureRoot, RawOrigin};
use orml_traits::{
	create_median_value_data_provider, parameter_type_with_key, DataFeeder, DataProviderExtended, MultiCurrency,
};
use orml_tokens::CurrencyAdapter;
use orml_authority::EnsureDelayed;
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};

use hex_literal::hex;
use module_evm::Runner;
use module_evm::{CallInfo, CreateInfo};
use module_evm_accounts::EvmAddressMapping;
pub use module_evm_manager::EvmCurrencyIdMapping;
use module_currencies::{BasicCurrencyAdapter, Currency};
use module_transaction_payment::{Multiplier, TargetedFeeAdjustment};

// re-exports

pub use pallet_staking::StakerStatus;

pub use authority::AuthorityConfigImpl;
pub use constants::{fee::*, time::*};
use primitives::{evm::EthereumTransactionMessage, currency::*;};
pub use primitives::{
	evm::EstimateResourcesRequest, AccountId, AccountIndex, Address, AirDropCurrencyId, Amount, AuctionId,
	AuthoritysOriginId, Balance, BlockNumber, CurrencyId, DataProviderId, EraIndex, Hash, Moment, Nonce,
	ReserveIdentifier, Share, Signature, TokenSymbol, TradingPair,
};
pub use runtime_common::{
	BlockLength, BlockWeights, GasToWeight, OffchainSolutionWeightLimit,
	Price, Rate, Ratio, SystemContractsFilter, ExchangeRate, TimeStampedPrice,
	cent, dollar, microcent, millicent, nanocent, deposit, ProxyType,

	EnsureRootOrAllShuraCouncil, EnsureRootOrHalfShuraCouncil,
	EnsureRootOrOneThirdsShuraCouncil, EnsureRootOrTwoThirdsShuraCouncil,
	EnsureRootOrThreeFourthsShuraCouncil, ShuraCouncilInstance, ShuraCouncilMembershipInstance,

	EnsureRootOrAllFinancialCouncil, EnsureRootOrHalfFinancialCouncil,
	EnsureRootOrOneThirdsFinancialCouncil, EnsureRootOrTwoThirdsFinancialCouncil,
	EnsureRootOrThreeFourthsFinancialCouncil, FinancialCouncilInstance, FinancialCouncilMembershipInstance,

	EnsureRootOrAllPublicFundCouncil, EnsureRootOrHalfPublicFundCouncil,
	EnsureRootOrOneThirdsPublicFundCouncil, EnsureRootOrTwoThirdsPublicFundCouncil,
	EnsureRootOrThreeFourthsPublicFundCouncil, PublicFundCouncilInstance, PublicFundCouncilMembershipInstance,

	EnsureRootOrAllAlSharifFundCouncil, EnsureRootOrHalfAlSharifFundCouncil,
	EnsureRootOrOneThirdsAlSharifFundCouncil, EnsureRootOrTwoThirdsAlSharifFundCouncil,
	EnsureRootOrThreeFourthsAlSharifFundCouncil, AlSharifFundCouncilInstance, AlSharifFundCouncilMembershipInstance,

	EnsureRootOrAllFoundationFundCouncil, EnsureRootOrHalfFoundationFundCouncil,
	EnsureRootOrOneThirdsFoundationFundCouncil, EnsureRootOrTwoThirdsFoundationFundCouncil,
	EnsureRootOrThreeFourthsFoundationFundCouncil, FoundationFundCouncilInstance, FoundationFundCouncilMembershipInstance,

	EnsureRootOrAllTechnicalCommittee, EnsureRootOrHalfTechnicalCommittee,
	EnsureRootOrOneThirdsTechnicalCommittee, EnsureRootOrTwoThirdsTechnicalCommittee,
	EnsureRootOrThreeFourthsTechnicalCommittee, TechnicalCommitteeInstance, TechnicalCommitteeMembershipInstance,

	OperatorMembershipInstanceSetheum, SETM, SETM, SETUSD, DNAR,
};


mod weights;

mod authority;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod constants;

// Pallet accounts of runtime
parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"set/trsy");
	pub const PublicFundTreasuryPalletId: PalletId = PalletId(*b"set/spft");
	pub const AlSharifFundTreasuryPalletId: PalletId = PalletId(*b"set/asft");
	pub const FoundationFundTreasuryPalletId: PalletId = PalletId(*b"set/sfft");
	pub const LoansPalletId: PalletId = PalletId(*b"set/loan");
	pub const DEXPalletId: PalletId = PalletId(*b"set/sdex");
	pub const CDPTreasuryPalletId: PalletId = PalletId(*b"set/cdpt");
	pub const SerpTreasuryPalletId: PalletId = PalletId(*b"set/serp");
	pub const NftPalletId: PalletId = PalletId(*b"set/sNFT");
}

pub fn get_all_module_accounts() -> Vec<AccountId> {
	vec![
		TreasuryPalletId::get().into_account(),
		PublicFundTreasuryPalletId::get().into_account(),
		AlSharifFundTreasuryPalletId::get().into_account(),
		FoundationFundTreasuryPalletId::get().into_account(),
		LoansPalletId::get().into_account(),
		DEXPalletId::get().into_account(),
		CDPTreasuryPalletId::get().into_account(),
		SerpTreasuryPalletId::get().into_account(),
		ZeroAccountId::get(),		 	// ACCOUNT 0
		BuyBackPoolAccountId::get(), 	// ACCOUNT 1
		CashDropPoolAccountId::get(), 	// ACCOUNT 2
		ThreeAccountId::get(),			// ACCOUNT 3
	]
}

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub babe: Babe,
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
			pub authority_discovery: AuthorityDiscovery,
		}
	}
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("setheum"),
	impl_name: create_runtime_str!("setheum"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY, // 1 in 4 blocks will be BABE
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
	};

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const BlockHashCount: BlockNumber = 2400; // 2 hours
	pub const SS58Prefix: u8 = 258;
}


pub struct BaseCallFilter;
impl Contains<Call> for BaseCallFilter {
	fn contains(call: &Call) -> bool {
		!module_transaction_pause::PausedTransactionFilter::<Runtime>::contains(call)
			&& !matches!(call, Call::Democracy(pallet_democracy::Call::propose(..)),)
	}
}

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = BaseCallFilter;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = (Indices, EvmAccounts);
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Nonce;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Maximum weight of each block.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = (
		module_evm::CallKillAccount<Runtime>,
		module_evm_accounts::CallKillAccount<Runtime>,
	);
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// This is a hook that is use when setCode is called - not require unless using cumulus.
	type OnSetCode = ();
}


parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Config for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type WeightInfo = ();
}

parameter_types! {
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = SECS_PER_BLOCK;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

pallet_staking_reward_curve::build! {
	// 2.58% min, 25.8% max, 50% ideal stake
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_800,
		max_inflation: 0_258_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_500,
	);
}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = 3; // 3 hours
	pub const BondingDuration: pallet_staking::EraIndex = 4; // 12 hours
	pub const SlashDeferDuration: pallet_staking::EraIndex = 2; // 6 hours
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
}

impl pallet_staking::Config for Runtime {
	const MAX_NOMINATIONS: u32 = 16;
	type Currency = Balances;
	type UnixTime = Timestamp;
	type CurrencyToVote = U128CurrencyToVote;
	type RewardRemainder = SetheumTreasury;
	type Event = Event;
	type Slash = SetheumTreasury; // send the slashed funds to the Setheum treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type SlashCancelOrigin = EnsureRootOrTwoThirdsTechnicalCommittee;
	type SessionInterface = Self;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type WeightInfo = ();
	type ElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type GenesisElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
}

impl onchain::Config for Runtime {
	type BlockWeights = BlockWeights;
	type AccountId = AccountId;
	type BlockNumber = BlockNumber;
	type Accuracy = sp_runtime::Perbill;
	type DataProvider = Staking;
}


impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::IdentificationTuple;
	type HandleEquivocation =
		pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;
	type WeightInfo = ();
	type DisabledValidators = Session;
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::IdentificationTuple;
	type HandleEquivocation =
		pallet_grandpa::EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = (Staking, ImOnline);
}

impl pallet_offences::Config for Runtime {
	type Event = Event;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pallet_authority_discovery::Config for Runtime {}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type ValidatorSet = Historical;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = ();
}

parameter_types! {
	pub const BasicDeposit: Balance =      100 * SETM;
	pub const FieldDeposit: Balance =        1 * SETM;
	pub const SubAccountDeposit: Balance =  20 * SETM;
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxAdditionalFields = MaxAdditionalFields;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = ();
	type ForceOrigin = EnsureRootOrTwoThirdsTechnicalCommittee;
	type RegistrarOrigin = EnsureRootOrTwoThirdsTechnicalCommittee;
	type WeightInfo = ();
}


parameter_types! {
	pub const IndexDeposit: Balance = 1 * SETM;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Event = Event;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type WeightInfo = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = SETM;
	pub const GetSerpCurrencyId: CurrencyId = SERP;
	pub const GetDinarCurrencyId: CurrencyId = DNAR;
	pub const SetterCurrencyId: CurrencyId = SETR;
	pub const GetSetUSDId: CurrencyId = SETUSD;
	pub StableCurrencyIds: Vec<CurrencyId> = vec![
		SETR,
		SETUSD,
	];
}

impl module_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type StableCurrencyIds = StableCurrencyIds;
	type SerpTreasury = SerpTreasury;
	type WeightInfo = weights::module_currencies::WeightInfo<Runtime>;
	type AddressMapping = EvmAddressMapping<Runtime>;
	type EVMBridge = EVMBridge;
	type SweepOrigin = EnsureRootOrOneShuraCouncil;
	type OnDust = module_currencies::TransferDust<Runtime, TreasuryAccount>;
}

parameter_types! {
	pub const MinimumCount: u32 = 1;
	pub const ExpiresIn: Moment = 1000 * 60 * 60; // 60 mins
	pub ZeroAccountId: AccountId = AccountId::from([0u8; 32]);
	pub const MaxHasDispatchedSize: u32 = 40;
}

type SetheumDataProvider = orml_oracle::Instance1;
impl orml_oracle::Config<SetheumDataProvider> for Runtime {
	type Event = Event;
	type OnNewData = ();
	type CombineData = orml_oracle::DefaultCombineData<Runtime, MinimumCount, ExpiresIn, SetheumDataProvider>;
	type Time = Timestamp;
	type OracleKey = CurrencyId;
	type OracleValue = Price;
	type RootOperatorAccountId = ZeroAccountId;
	type Members = OperatorMembershipSetheum;
	type MaxHasDispatchedSize = MaxHasDispatchedSize;
	type WeightInfo = weights::orml_oracle::WeightInfo<Runtime>;
}

create_median_value_data_provider!(
	AggregatedDataProvider,
	CurrencyId,
	Price,
	TimeStampedPrice,
	[SetheumOracle]
);
// Aggregated data provider cannot feed.
impl DataFeeder<CurrencyId, Price, AccountId> for AggregatedDataProvider {
	fn feed_value(_: AccountId, _: CurrencyId, _: Price) -> DispatchResult {
		Err("Not supported".into())
	}
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		get_all_module_accounts().contains(a)
	}
}

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			CurrencyId::Token(symbol) => match symbol {
				// TODO: Update to lower amounts when prices increase to a reasonable rate.
				// ...aavvvvVVVVVVVVV
				// TokenSymbol::SETUSD => cent(*currency_id),
				// TokenSymbol::SETR => cent(*currency_id),
				// TokenSymbol::SERP => cent(*currency_id),
				// TokenSymbol::DNAR => cent(*currency_id),
				// TokenSymbol::SETM => cent(*currency_id),

				TokenSymbol::SETUSD => dollar(*currency_id),
				TokenSymbol::SETR => 1 * millicent(*currency_id),
				TokenSymbol::SERP => 1 * dollar(*currency_id),
				TokenSymbol::DNAR => 1 * dollar(*currency_id),
				TokenSymbol::SETM => 1 * dollar(*currency_id),
			},
			CurrencyId::DexShare(dex_share_0, _) => {
				let currency_id_0: CurrencyId = (*dex_share_0).into();

				// initial dex share amount is calculated based on currency_id_0,
				// use the ED of currency_id_0 as the ED of lp token.
				if currency_id_0 == GetNativeCurrencyId::get() {
					NativeTokenExistentialDeposit::get()
				} else if let CurrencyId::Erc20(_) = currency_id_0 {
					// LP token with erc20
					1
				} else {
					Self::get(&currency_id_0)
				}
			},
			CurrencyId::Erc20(_) => Balance::max_value(), // not handled by orml-tokens
		}
	};
}

parameter_types! {
	pub TreasuryAccount: AccountId = TreasuryPalletId::get().into_account();
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = weights::orml_tokens::WeightInfo<Runtime>;
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = orml_tokens::TransferDust<Runtime, TreasuryAccount>;
	type MaxLocks = MaxLocks;
	type DustRemovalWhitelist = DustRemovalWhitelist;
}

pub struct EnsureRootOrTreasury;
impl EnsureOrigin<Origin> for EnsureRootOrTreasury {
	type Success = AccountId;

	fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
		Into::<Result<RawOrigin<AccountId>, Origin>>::into(o).and_then(|o| match o {
			RawOrigin::Root => Ok(TreasuryPalletId::get().into_account()),
			RawOrigin::Signed(caller) => {
				if caller == TreasuryPalletId::get().into_account() {
					Ok(caller)
				} else {
					Err(Origin::from(Some(caller)))
				}
			}
			r => Err(Origin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> Origin {
		Origin::from(RawOrigin::Signed(Default::default()))
	}
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(10) * RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

parameter_types! {
	pub SetUSDFixedPrice: Price = Price::saturating_from_rational(1, 1); // $1
	pub SetterFixedPrice: Price = Price::saturating_from_rational(2, 1); // $2
}

impl module_prices::Config for Runtime {
	type Event = Event;
	type Source = AggregatedDataProvider;
	type GetSetUSDId = GetSetUSDId;
	type SetterCurrencyId = SetterCurrencyId;
	type SetUSDFixedPrice = SetUSDFixedPrice;
	type SetterFixedPrice = SetterFixedPrice;
	type LockOrigin = EnsureRootOrTwoThirdsFinancialCouncil;
	type DEX = Dex;
	type Currency = Currencies;
	type CurrencyIdMapping = EvmCurrencyIdMapping<Runtime>;
	type WeightInfo = weights::module_prices::WeightInfo<Runtime>;
}

impl module_transaction_pause::Config for Runtime {
	type Event = Event;
	type UpdateOrigin = EnsureRootOrThreeFourthsShuraCouncil;
	type WeightInfo = weights::module_transaction_pause::WeightInfo<Runtime>;
}

parameter_types! {
	pub MinimumIncrementSize: Rate = Rate::saturating_from_rational(2, 100); // 2%
	pub const AuctionTimeToClose: BlockNumber = 15 * MINUTES;
	pub const AuctionDurationSoftCap: BlockNumber = 2 * HOURS;
	pub DefaultSwapParitalPathList: Vec<Vec<CurrencyId>> = vec![
		vec![SETUSD],
		vec![SETM, SETUSD],
		vec![SERP, SETUSD],
		vec![DNAR, SETUSD],
		vec![SETR, SETUSD],
	];
}

impl auction_manager::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type Auction = Auction;
	type MinimumIncrementSize = MinimumIncrementSize;
	type AuctionTimeToClose = AuctionTimeToClose;
	type AuctionDurationSoftCap = AuctionDurationSoftCap;
	type GetSetUSDId = GetSetUSDId;
	type CDPTreasury = CdpTreasury;
	type DEX = Dex;
	type PriceSource = module_prices::PriorityLockedPriceProvider<Runtime>;
	type UnsignedPriority = runtime_common::AuctionManagerUnsignedPriority;
	type EmergencyShutdown = EmergencyShutdown;
	type DefaultSwapParitalPathList = DefaultSwapParitalPathList;
	type WeightInfo = weights::auction_manager::WeightInfo<Runtime>;
}

impl module_loans::Config for Runtime {
	type Event = Event;
	type Convert = cdp_engine::DebitExchangeRateConvertor<Runtime>;
	type Currency = Currencies;
	type RiskManager = CdpEngine;
	type CDPTreasury = CdpTreasury;
	type PalletId = LoansPalletId;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as sp_runtime::traits::Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(
		Call,
		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
	)> {
		// take the biggest period possible.
		let period = BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let extra: SignedExtra = (
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			module_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			module_evm::SetEvmOrigin::<Runtime>::new(),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
}

parameter_types! {
	pub CollateralCurrencyIds: Vec<CurrencyId> = vec![SETM, SERP, DNAR, SETR];
	pub DefaultLiquidationRatio: Ratio = Ratio::saturating_from_rational(110, 100);
	pub DefaultDebitExchangeRate: ExchangeRate = ExchangeRate::saturating_from_rational(1, 10);
	pub DefaultLiquidationPenalty: Rate = Rate::saturating_from_rational(5, 100);
	pub MinimumDebitValue: Balance = dollar(SETUSD);
	pub MaxSwapSlippageCompareToOracle: Ratio = Ratio::saturating_from_rational(15, 100);
}

impl cdp_engine::Config for Runtime {
	type Event = Event;
	type PriceSource = module_prices::PriorityLockedPriceProvider<Runtime>;
	type CollateralCurrencyIds = CollateralCurrencyIds;
	type DefaultLiquidationRatio = DefaultLiquidationRatio;
	type DefaultDebitExchangeRate = DefaultDebitExchangeRate;
	type DefaultLiquidationPenalty = DefaultLiquidationPenalty;
	type MinimumDebitValue = MinimumDebitValue;
	type GetSetUSDId = GetSetUSDId;
	type CDPTreasury = CdpTreasury;
	type UpdateOrigin = EnsureRootOrHalfFinancialCouncil;
	type MaxSwapSlippageCompareToOracle = MaxSwapSlippageCompareToOracle;
	type UnsignedPriority = runtime_common::CdpEngineUnsignedPriority;
	type EmergencyShutdown = EmergencyShutdown;
	type DefaultSwapParitalPathList = DefaultSwapParitalPathList;
	type WeightInfo = weights::cdp_engine::WeightInfo<Runtime>;
}

parameter_types! {
	pub DepositPerAuthorization: Balance = dollar(SETM);
}

impl serp_setmint::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type DepositPerAuthorization = DepositPerAuthorization;
	type WeightInfo = weights::serp_setmint::WeightInfo<Runtime>;
}

impl emergency_shutdown::Config for Runtime {
	type Event = Event;
	type CollateralCurrencyIds = CollateralCurrencyIds;
	type PriceSource = Prices;
	type CDPTreasury = CdpTreasury;
	type AuctionManagerHandler = AuctionManager;
	type ShutdownOrigin = EnsureRootOrHalfShuraCouncil;
	type WeightInfo = weights::emergency_shutdown::WeightInfo<Runtime>;
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (3, 1000);	// 0.3%
	pub const GetStableCurrencyExchangeFee: (u32, u32) = (1, 1000);	// 0.1%
	pub const TradingPathLimit: u32 = 4;
	pub EnabledTradingPairs: Vec<TradingPair> = vec![
		TradingPair::from_currency_ids(SETUSD, SETM).unwrap(),
		TradingPair::from_currency_ids(SETUSD, DNAR).unwrap(),
		TradingPair::from_currency_ids(SETUSD, RENBTC).unwrap(),
	];
}

impl module_dex::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type StableCurrencyIds = StableCurrencyIds;
	type GetExchangeFee = GetExchangeFee;
	type GetStableCurrencyExchangeFee = GetStableCurrencyExchangeFee;
	type TradingPathLimit = TradingPathLimit;
	type PalletId = DEXPalletId;
	type CurrencyIdMapping = EvmCurrencyIdMapping<Runtime>;
	type DEXIncentives = Incentives;
	type WeightInfo = weights::module_dex::WeightInfo<Runtime>;
	type ListingOrigin = EnsureRootOrHalfShuraCouncil;
}

impl module_airdrop::Config for Runtime {
	type Event = Event;
}

parameter_types! {
	pub DefaultSwapPathList: Vec<Vec<CurrencyId>> = vec![vec![SETR, DNAR], vec![SETUSD, SETR, DNAR]];
	pub StableCurrencyInflationPeriod: u64 = 200; // Every 10 minutes
	pub SetterMinimumClaimableTransferAmounts: Balance = 2;
	pub SetterMaximumClaimableTransferAmounts: Balance = 200;
	pub SetDollarMinimumClaimableTransferAmounts: Balance = 2;
	pub SetDollarMaximumClaimableTransferAmounts: Balance = 200;
}

impl serp_treasury::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type StableCurrencyIds = StableCurrencyIds;
	type StableCurrencyInflationPeriod = StableCurrencyInflationPeriod;
	type GetStableCurrencyMinimumSupply = GetStableCurrencyMinimumSupply;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type GetSerpCurrencyId = GetSerpCurrencyId;
	type GetDinarCurrencyId = GetDinarCurrencyId;
	type SetterCurrencyId = SetterCurrencyId;
	type GetSetUSDId = GetSetUSDId;
	type CashDropPoolAccountId = CashDropPoolAccountId;
	type PublicFundAccountId = PublicFundAccountId;
	type AlSharifFundAccountId = AlSharifFundAccountId;
	type CDPTreasuryAccountId = CDPTreasuryAccountId;
	type SetheumTreasuryAccountId = SetheumTreasuryAccountId;
	type DefaultSwapPathList = DefaultSwapPathList;
	type Dex = MockDEX;
	type MaxSwapSlippageCompareToOracle = MaxSwapSlippageCompareToOracle;
	type TradingPathLimit = TradingPathLimit;
	type PriceSource = MockPriceSource;
	type SetterMinimumClaimableTransferAmounts = SetterMinimumClaimableTransferAmounts;
	type SetterMaximumClaimableTransferAmounts = SetterMaximumClaimableTransferAmounts;
	type SetDollarMinimumClaimableTransferAmounts = SetDollarMinimumClaimableTransferAmounts;
	type SetDollarMaximumClaimableTransferAmounts = SetDollarMaximumClaimableTransferAmounts;
	type UpdateOrigin = EnsureSignedBy<Root, AccountId>;
	type PalletId = SerpTreasuryPalletId;
	type WeightInfo = weights::serp_treasury::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxAuctionsCount: u32 = 100;
}

impl cdp_treasury::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type GetSetUSDId = GetSetUSDId;
	type AuctionManagerHandler = AuctionManager;
	type UpdateOrigin = EnsureRootOrHalfFinancialCouncil;
	type DEX = Dex;
	type MaxAuctionsCount = MaxAuctionsCount;
	type PalletId = CDPTreasuryPalletId;
	type SerpTreasury = SerpTreasury;
	type WeightInfo = weights::cdp_treasury::WeightInfo<Runtime>;
}

parameter_types! {
	// Sort by fee charge order
	pub DefaultFeeSwapPathList: Vec<Vec<CurrencyId>> = vec![vec![SETUSD, SETM], vec![SETUSD, DNAR], vec![SETUSD, RENBTC]];
}

type NegativeImbalance = <Balances as PalletCurrency<AccountId>>::NegativeImbalance;
pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
            // for fees, 50% to treasury, 50% to author
            let mut split = fees.ration(50, 50);
            if let Some(tips) = fees_then_tips.next() {
                // for tips, if any, 60% to treasury, 40% to author (though this can be anything)
                tips.ration_merge_into(60, 40, &mut split);
            }
            Treasury::on_unbalanced(split.0);
            Author::on_unbalanced(split.1);
        }
	}
}

impl module_transaction_payment::Config for Runtime {
	type NativeCurrencyId = GetNativeCurrencyId;
	type DefaultFeeSwapPathList = DefaultFeeSwapPathList;
	type Currency = Balances;
	type MultiCurrency = Currencies;
	type OnTransactionPayment = DealWithFees;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = WeightToFee;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
	type DEX = Dex;
	type MaxSwapSlippageCompareToOracle = MaxSwapSlippageCompareToOracle;
	type TradingPathLimit = TradingPathLimit;
	type PriceSource = module_prices::RealTimePriceProvider<Runtime>;
	type WeightInfo = weights::module_transaction_payment::WeightInfo<Runtime>;
}

impl module_evm_accounts::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type AddressMapping = EvmAddressMapping<Runtime>;
	type TransferAll = Currencies;
	type WeightInfo = weights::module_evm_accounts::WeightInfo<Runtime>;
}

impl module_evm_manager::Config for Runtime {
	type Currency = Balances;
	type EVMBridge = EVMBridge;
}

parameter_types! {
	pub const ChainId: u64 = 258;
	pub NetworkContractSource: H160 = H160::from_low_u64_be(0);
}

#[cfg(feature = "with-ethereum-compatibility")]
parameter_types! {
	pub NativeTokenExistentialDeposit: Balance = 10 * cent(SETM);
	pub const NewContractExtraBytes: u32 = 0;
	pub const StorageDepositPerByte: Balance = 0;
	pub const DeveloperDeposit: Balance = 0;
	pub const DeploymentFee: Balance = 0;
}

#[cfg(not(feature = "with-ethereum-compatibility"))]
parameter_types! {
	pub NativeTokenExistentialDeposit: Balance = 10 * cent(SETM);
	pub const NewContractExtraBytes: u32 = 10_000;
	pub StorageDepositPerByte: Balance = deposit(0, 1);
	pub DeveloperDeposit: Balance = dollar(SETM);
	pub DeploymentFee: Balance = dollar(SETM);
}

pub type MultiCurrencyPrecompile = runtime_common::MultiCurrencyPrecompile<
	AccountId,
	EvmAddressMapping<Runtime>,
	EvmCurrencyIdMapping<Runtime>,
	Currencies,
>;

pub type NFTPrecompile =
	runtime_common::NFTPrecompile<AccountId, EvmAddressMapping<Runtime>, EvmCurrencyIdMapping<Runtime>, NFT>;
pub type StateRentPrecompile =
	runtime_common::StateRentPrecompile<AccountId, EvmAddressMapping<Runtime>, EvmCurrencyIdMapping<Runtime>, EVM>;
pub type OraclePrecompile = runtime_common::OraclePrecompile<
	AccountId,
	EvmAddressMapping<Runtime>,
	EvmCurrencyIdMapping<Runtime>,
	module_prices::RealTimePriceProvider<Runtime>,
>;
pub type ScheduleCallPrecompile = runtime_common::ScheduleCallPrecompile<
	AccountId,
	EvmAddressMapping<Runtime>,
	EvmCurrencyIdMapping<Runtime>,
	Scheduler,
	module_transaction_payment::ChargeTransactionPayment<Runtime>,
	Call,
	Origin,
	OriginCaller,
	Runtime,
>;
pub type DexPrecompile =
	runtime_common::DexPrecompile<AccountId, EvmAddressMapping<Runtime>, EvmCurrencyIdMapping<Runtime>, Dex>;

#[cfg(feature = "with-ethereum-compatibility")]
static ISTANBUL_CONFIG: evm::Config = evm::Config::istanbul();

impl module_evm::Config for Runtime {
	type AddressMapping = EvmAddressMapping<Runtime>;
	type Currency = Balances;
	type TransferAll = Currencies;
	type NewContractExtraBytes = NewContractExtraBytes;
	type StorageDepositPerByte = StorageDepositPerByte;
	type Event = Event;
	type Precompiles = runtime_common::AllPrecompiles<
		SystemContractsFilter,
		MultiCurrencyPrecompile,
		NFTPrecompile,
		StateRentPrecompile,
		OraclePrecompile,
		ScheduleCallPrecompile,
		DexPrecompile,
	>;
	type ChainId = ChainId;
	type GasToWeight = GasToWeight;
	type ChargeTransactionPayment = module_transaction_payment::ChargeTransactionPayment<Runtime>;
	type NetworkContractOrigin = EnsureRootOrTwoThirdsTechnicalCommittee;
	type NetworkContractSource = NetworkContractSource;
	type DeveloperDeposit = DeveloperDeposit;
	type DeploymentFee = DeploymentFee;
	type TreasuryAccount = TreasuryAccount;
	type FreeDeploymentOrigin = EnsureRootOrHalfShuraCouncil;
	type Runner = module_evm::runner::stack::Runner<Self>;
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type WeightInfo = weights::module_evm::WeightInfo<Runtime>;

	#[cfg(feature = "with-ethereum-compatibility")]
	fn config() -> &'static evm::Config {
		&ISTANBUL_CONFIG
	}
}

impl module_evm_bridge::Config for Runtime {
	type EVM = EVM;
}

parameter_types! {
	pub CreateClassDeposit: Balance = 20 * dollar(SETM);
	pub CreateTokenDeposit: Balance = 2 * dollar(SETM);
	pub MaxAttributesBytes: u32 = 2048;
}

impl module_nft::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type CreateClassDeposit = CreateClassDeposit;
	type CreateTokenDeposit = CreateTokenDeposit;
	type DataDepositPerByte = DataDepositPerByte;
	type PalletId = NftPalletId;
	type MaxAttributesBytes = MaxAttributesBytes;
	type WeightInfo = weights::module_nft::WeightInfo<Runtime>;
}

parameter_types! {
	pub MaxClassMetadata: u32 = 1024;
	pub MaxTokenMetadata: u32 = 1024;
}

impl orml_nft::Config for Runtime {
	type ClassId = u32;
	type TokenId = u64;
	type ClassData = module_nft::ClassData<Balance>;
	type TokenData = module_nft::TokenData<Balance>;
	type MaxClassMetadata = MaxClassMetadata;
	type MaxTokenMetadata = MaxTokenMetadata;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	pub AnnouncementDepositBase: Balance = deposit(1, 8);
	pub AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxPending: u16 = 32;
}

impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			// Always allowed Call::Utility no matter type.
			// Only transactions allowed by Proxy.filter can be executed,
			// otherwise `BadOrigin` will be returned in Call::Utility.
			_ if matches!(c, Call::Utility(..)) => true,
			ProxyType::Any => true,
			ProxyType::CancelProxy => matches!(c, Call::Proxy(pallet_proxy::Call::reject_announcement(..))),
			ProxyType::Governance => {
				matches!(
					c,
					Call::Authority(..)
						| Call::Democracy(..)
						| Call::ShuraCouncil(..)
						| Call::FinancialCouncil(..)
						| Call::PublicFundCouncil(..)
						| Call::AlSharifFundCouncil(..)
						| Call::FoundationFundCouncil(..)
						| Call::TechnicalCommittee(..)
						| Call::Treasury(..)
						| Call::TreasuryBounties(..)
						| Call::TreasuryTips(..)
						| Call::PublicFund(..)
						| Call::PublicFundBounties(..)
						| Call::PublicFundTips(..)
						| Call::AlSharifFund(..)
						| Call::AlSharifFundBounties(..)
						| Call::AlSharifFundTips(..)
						| Call::FoundationFund(..)
						| Call::FoundationFundBounties(..)
						| Call::FoundationFundTips(..)
				)
			}
			ProxyType::Auction => {
				matches!(c, Call::Auction(orml_auction::Call::bid(..)))
			}
			ProxyType::Swap => {
				matches!(
					c,
					Call::Dex(module_dex::Call::swap_with_exact_supply(..))
						| Call::Dex(module_dex::Call::swap_with_exact_target(..))
				)
			}
			ProxyType::Loan => {
				matches!(
					c,
					Call::Setmint(serp_setmint::Call::adjust_loan(..))
						| Call::Setmint(serp_setmint::Call::close_loan_has_debit_by_dex(..))
				)
			}
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = ();
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
	pub const CashCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::CASH);
	pub const MaxGatewayAuthorityCount: u32 = 8;
	pub const PercentThresholdForGatewayAuthoritySignature: Perbill = Perbill::from_percent(50);
}

parameter_types! {
	// note: if we add other native tokens (SETUSD) we have to set native
	// existential deposit to 0 or check for other tokens on account pruning
	pub const NativeTokenExistentialDeposit: Balance =       1 * SETM;
	pub const MaxNativeTokenExistentialDeposit: Balance = 1000 * SETM;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type Event = Event;
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = (); // burn
	type ExistentialDeposit = NativeTokenExistentialDeposit;
	type AccountStore = frame_system::Pallet<Runtime>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}


parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(10) * BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = ();
}

impl orml_authority::Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type Scheduler = Scheduler;
	type AsOriginId = AuthoritysOriginId;
	type AuthorityConfig = AuthorityConfigImpl;
	type WeightInfo = ();
}


impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const ShuraCouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const ShuraCouncilMaxProposals: u32 = 100;
	pub const ShuraCouncilMaxMembers: u32 = 100;
}

impl pallet_collective::Config<ShuraCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = ShuraCouncilMotionDuration;
	type MaxProposals = ShuraCouncilMaxProposals;
	type MaxMembers = ShuraCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

impl pallet_membership::Config<ShuraCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrThreeFourthsShuraCouncil;
	type RemoveOrigin = EnsureRootOrThreeFourthsShuraCouncil;
	type SwapOrigin = EnsureRootOrThreeFourthsShuraCouncil;
	type ResetOrigin = EnsureRootOrThreeFourthsShuraCouncil;
	type PrimeOrigin = EnsureRootOrThreeFourthsShuraCouncil;
	type MembershipInitialized = ShuraCouncil;
	type MembershipChanged = ShuraCouncil;
	type MaxMembers = ShuraCouncilMaxMembers;
	type WeightInfo = ();
}

parameter_types! {
	pub const FinancialCouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const FinancialCouncilMaxProposals: u32 = 100;
	pub const FinancialCouncilMaxMembers: u32 = 100;
}

impl pallet_collective::Config<FinancialCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = FinancialCouncilMotionDuration;
	type MaxProposals = FinancialCouncilMaxProposals;
	type MaxMembers = FinancialCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

impl pallet_membership::Config<FinancialCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type RemoveOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type SwapOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type ResetOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type PrimeOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type MembershipInitialized = FinancialCouncil;
	type MembershipChanged = FinancialCouncil;
	type MaxMembers = FinancialCouncilMaxMembers;
	type WeightInfo = ();
}

parameter_types! {
	pub const PublicFundCouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const PublicFundCouncilMaxProposals: u32 = 100;
	pub const PublicFundCouncilMaxMembers: u32 = 100;
}

impl pallet_collective::Config<PublicFundCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = PublicFundCouncilMotionDuration;
	type MaxProposals = PublicFundCouncilMaxProposals;
	type MaxMembers = PublicFundCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

impl pallet_membership::Config<PublicFundCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrThreeFourthsPublicFundCouncil;
	type RemoveOrigin = EnsureRootOrThreeFourthsPublicFundCouncil;
	type SwapOrigin = EnsureRootOrThreeFourthsPublicFundCouncil;
	type ResetOrigin = EnsureRootOrThreeFourthsPublicFundCouncil;
	type PrimeOrigin = EnsureRootOrThreeFourthsPublicFundCouncil;
	type MembershipInitialized = PublicFundCouncil;
	type MembershipChanged = PublicFundCouncil;
	type MaxMembers = PublicFundCouncilMaxMembers;
	type WeightInfo = ();
}

parameter_types! {
	pub const AlSharifFundCouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const AlSharifFundCouncilMaxProposals: u32 = 100;
	pub const AlSharifFundCouncilMaxMembers: u32 = 100;
}

impl pallet_collective::Config<AlSharifFundCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = AlSharifFundCouncilMotionDuration;
	type MaxProposals = AlSharifFundCouncilMaxProposals;
	type MaxMembers = AlSharifFundCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

impl pallet_membership::Config<AlSharifFundCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrThreeFourthsAlSharifFundCouncil;
	type RemoveOrigin = EnsureRootOrThreeFourthsAlSharifFundCouncil;
	type SwapOrigin = EnsureRootOrThreeFourthsAlSharifFundCouncil;
	type ResetOrigin = EnsureRootOrThreeFourthsAlSharifFundCouncil;
	type PrimeOrigin = EnsureRootOrThreeFourthsAlSharifFundCouncil;
	type MembershipInitialized = AlSharifFundCouncil;
	type MembershipChanged = AlSharifFundCouncil;
	type MaxMembers = AlSharifFundCouncilMaxMembers;
	type WeightInfo = ();
}

parameter_types! {
	pub const FoundationFundCouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const FoundationFundCouncilMaxProposals: u32 = 100;
	pub const FoundationFundCouncilMaxMembers: u32 = 100;
}

impl pallet_collective::Config<FoundationFundCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = FoundationFundCouncilMotionDuration;
	type MaxProposals = FoundationFundCouncilMaxProposals;
	type MaxMembers = FoundationFundCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

impl pallet_membership::Config<FoundationFundCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrThreeFourthsFoundationFundCouncil;
	type RemoveOrigin = EnsureRootOrThreeFourthsFoundationFundCouncil;
	type SwapOrigin = EnsureRootOrThreeFourthsFoundationFundCouncil;
	type ResetOrigin = EnsureRootOrThreeFourthsFoundationFundCouncil;
	type PrimeOrigin = EnsureRootOrThreeFourthsFoundationFundCouncil;
	type MembershipInitialized = FoundationFundCouncil;
	type MembershipChanged = FoundationFundCouncil;
	type MaxMembers = FoundationFundCouncilMaxMembers;
	type WeightInfo = ();
}

parameter_types! {
	pub const TechnicalCommitteeMotionDuration: BlockNumber = 7 * DAYS;
	pub const TechnicalCommitteeMaxProposals: u32 = 100;
	pub const TechnicalCouncilMaxMembers: u32 = 100;
}

impl pallet_collective::Config<TechnicalCommitteeInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = TechnicalCommitteeMotionDuration;
	type MaxProposals = TechnicalCommitteeMaxProposals;
	type MaxMembers = TechnicalCouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

impl pallet_membership::Config<TechnicalCommitteeMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type RemoveOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type SwapOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type ResetOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type PrimeOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = TechnicalCouncilMaxMembers;
	type WeightInfo = ();
}

parameter_types! {
	pub const OracleMaxMembers: u32 = 100;
}

impl pallet_membership::Config<OperatorMembershipInstanceSetheum> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrTwoThirdsFinancialCouncil;
	type RemoveOrigin = EnsureRootOrTwoThirdsFinancialCouncil;
	type SwapOrigin = EnsureRootOrTwoThirdsFinancialCouncil;
	type ResetOrigin = EnsureRootOrTwoThirdsFinancialCouncil;
	type PrimeOrigin = EnsureRootOrTwoThirdsFinancialCouncil;
	type MembershipInitialized = ();
	type MembershipChanged = SetheumOracle;
	type MaxMembers = OracleMaxMembers;
	type WeightInfo = ();
}

impl pallet_utility::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type WeightInfo = ();
}

parameter_types! {
	pub MultisigDepositBase: Balance = 500 * millicent(SETM);
	pub MultisigDepositFactor: Balance = 100 * millicent(SETM);
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type DepositBase = MultisigDepositBase;
	type DepositFactor = MultisigDepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = ();
}

pub struct ShuraCouncilProvider;
impl SortedMembers<AccountId> for ShuraCouncilProvider {
	fn sorted_members() -> Vec<AccountId> {
		ShuraCouncil::members()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(_: &AccountId) {
		todo!()
	}
}

impl ContainsLengthBound for ShuraCouncilProvider {
	fn max_len() -> usize {
		100
	}
	fn min_len() -> usize {
		0
	}
}


pub struct PublicFundCouncilProvider;
impl SortedMembers<AccountId> for PublicFundCouncilProvider {
	fn sorted_members() -> Vec<AccountId> {
		PublicFundCouncil::members()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(_: &AccountId) {
		todo!()
	}
}

impl ContainsLengthBound for PublicFundCouncilProvider {
	fn max_len() -> usize {
		100
	}
	fn min_len() -> usize {
		0
	}
}


pub struct AlSharifFundCouncilProvider;
impl SortedMembers<AccountId> for AlSharifFundCouncilProvider {
	fn sorted_members() -> Vec<AccountId> {
		AlSharifFundCouncil::members()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(_: &AccountId) {
		todo!()
	}
}

impl ContainsLengthBound for AlSharifFundCouncilProvider {
	fn max_len() -> usize {
		100
	}
	fn min_len() -> usize {
		0
	}
}


pub struct FoundationFundCouncilProvider;
impl SortedMembers<AccountId> for FoundationFundCouncilProvider {
	fn sorted_members() -> Vec<AccountId> {
		FoundationFundCouncil::members()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(_: &AccountId) {
		todo!()
	}
}

impl ContainsLengthBound for FoundationFundCouncilProvider {
	fn max_len() -> usize {
		100
	}
	fn min_len() -> usize {
		0
	}
}


parameter_types! {
	pub const TreasuryProposalBond: Permill = Permill::from_percent(5);
	pub TreasuryProposalBondMinimum: Balance = dollar(SETM);
	pub const TreasurySpendPeriod: BlockNumber = 21 * DAYS;
	pub const TreasuryBurn: Permill = Permill::from_percent(0);

	pub const TreasuryTipCountdown: BlockNumber = DAYS;
	pub const TreasuryTipFindersFee: Percent = Percent::from_percent(10);
	pub TreasuryTipReportDepositBase: Balance = deposit(1, 0);
	pub const TreasurySevenDays: BlockNumber = 7 * DAYS;
	pub const TreasuryZeroDay: BlockNumber = 0;
	pub const OneDay: BlockNumber = DAYS;
	pub TreasuryBountyDepositBase: Balance = dollar(SETM);
	pub const TreasuryBountyDepositPayoutDelay: BlockNumber = DAYS;
	pub const TreasuryBountyUpdatePeriod: BlockNumber = 21 * DAYS;
	pub const TreasuryBountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub TreasuryBountyValueMinimum: Balance = 5 * dollar(SETM);
	pub TreasuryDataDepositPerByte: Balance = deposit(0, 1);
	pub const TreasuryMaximumReasonLength: u32 = 16384;
	pub const TreasuryMaxApprovals: u32 = 100;
}

type SetheumTreasuryInstance = pallet_treasury::Instance1;
impl pallet_treasury::Config<SetheumTreasuryInstance> for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EnsureRootOrHalfShuraCouncil;
	type RejectOrigin = EnsureRootOrHalfShuraCouncil;
	type Event = Event;
	type OnSlash = Treasury;
	type ProposalBond = TreasuryProposalBond;
	type ProposalBondMinimum = TreasuryProposalBondMinimum;
	type SpendPeriod = TreasurySpendPeriod;
	type Burn = TreasuryBurn;
	type BurnDestination = ();
	type SpendFunds = TreasuryBounties;
	type WeightInfo = ();
	type MaxApprovals = TreasuryMaxApprovals;
}

type SetheumTreasuryBountiesInstance = pallet_bounties::Instance1;
impl pallet_bounties::Config<SetheumTreasuryBountiesInstance> for Runtime {
	type Event = Event;
	type BountyDepositBase = TreasuryBountyDepositBase;
	type BountyDepositPayoutDelay = TreasuryBountyDepositPayoutDelay;
	type BountyUpdatePeriod = TreasuryBountyUpdatePeriod;
	type BountyCuratorDeposit = TreasuryBountyCuratorDeposit;
	type BountyValueMinimum = TreasuryBountyValueMinimum;
	type DataDepositPerByte = TreasuryDataDepositPerByte;
	type MaximumReasonLength = TreasuryMaximumReasonLength;
	type WeightInfo = ();
}

type SetheumTreasuryTipsInstance = pallet_tips::Instance1;
impl pallet_tips::Config<SetheumTreasuryTipsInstance> for Runtime {
	type Event = Event;
	type DataDepositPerByte = TreasuryDataDepositPerByte;
	type MaximumReasonLength = TreasuryMaximumReasonLength;
	type Tippers = ShuraCouncilProvider;
	type TipCountdown = TreasuryTipCountdown;
	type TipFindersFee = TreasuryTipFindersFee;
	type TipReportDepositBase = TreasuryTipReportDepositBase;
	type WeightInfo = ();
}


parameter_types! {
	pub const PublicFundProposalBond: Permill = Permill::from_percent(5);
	pub PublicFundProposalBondMinimum: Balance = dollar(SETM);
	pub const PublicFundSpendPeriod: BlockNumber = 21 * DAYS;
	pub const PublicFundBurn: Permill = Permill::from_percent(0);

	pub const PublicFundTipCountdown: BlockNumber = DAYS;
	pub const PublicFundTipFindersFee: Percent = Percent::from_percent(10);
	pub PublicFundTipReportDepositBase: Balance = deposit(1, 0);
	pub const PublicFundSevenDays: BlockNumber = 7 * DAYS;
	pub const PublicFundZeroDay: BlockNumber = 0;
	pub const OneDay: BlockNumber = DAYS;
	pub PublicFundBountyDepositBase: Balance = dollar(SETM);
	pub const PublicFundBountyDepositPayoutDelay: BlockNumber = DAYS;
	pub const PublicFundBountyUpdatePeriod: BlockNumber = 21 * DAYS;
	pub const PublicFundBountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub PublicFundBountyValueMinimum: Balance = 5 * dollar(SETM);
	pub PublicFundDataDepositPerByte: Balance = deposit(0, 1);
	pub const PublicFundMaximumReasonLength: u32 = 16384;
	pub const PublicFundMaxApprovals: u32 = 100;
}

type PublicFundTreasuryInstance = pallet_treasury::Instance2;
impl pallet_treasury::Config<PublicFundTreasuryInstance> for Runtime {
	type PalletId = PublicFundTreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EnsureRootOrHalfPublicFundCouncil;
	type RejectOrigin = EnsureRootOrHalfPublicFundCouncil;
	type Event = Event;
	type OnSlash = PublicFundTreasury;
	type ProposalBond = PublicFundProposalBond;
	type ProposalBondMinimum = PublicFundProposalBondMinimum;
	type SpendPeriod = PublicFundSpendPeriod;
	type Burn = PublicFundBurn;
	type BurnDestination = ();
	type SpendFunds = PublicFundBounties;
	type WeightInfo = ();
	type MaxApprovals = PublicFundMaxApprovals;
}

type PublicFundBountiesInstance = pallet_bounties::Instance2;
impl pallet_bounties::Config<PublicFundBountiesInstance> for Runtime {
	type Event = Event;
	type BountyDepositBase = PublicFundBountyDepositBase;
	type BountyDepositPayoutDelay = PublicFundBountyDepositPayoutDelay;
	type BountyUpdatePeriod = PublicFundBountyUpdatePeriod;
	type BountyCuratorDeposit = PublicFundBountyCuratorDeposit;
	type BountyValueMinimum = PublicFundBountyValueMinimum;
	type DataDepositPerByte = PublicFundDataDepositPerByte;
	type MaximumReasonLength = PublicFundMaximumReasonLength;
	type WeightInfo = ();
}

type PublicFundTipsInstance = pallet_tips::Instance2;
impl pallet_tips::Config<PublicFundTipsInstance> for Runtime {
	type Event = Event;
	type DataDepositPerByte = PublicFundDataDepositPerByte;
	type MaximumReasonLength = PublicFundMaximumReasonLength;
	type Tippers = PublicFundCouncilProvider;
	type TipCountdown = PublicFundTipCountdown;
	type TipFindersFee = PublicFundTipFindersFee;
	type TipReportDepositBase = PublicFundTipReportDepositBase;
	type WeightInfo = ();
}


parameter_types! {
	pub const AlSharifFundProposalBond: Permill = Permill::from_percent(5);
	pub AlSharifFundProposalBondMinimum: Balance = dollar(SETM);
	pub const AlSharifFundSpendPeriod: BlockNumber = 21 * DAYS;
	pub const AlSharifFundBurn: Permill = Permill::from_percent(0);

	pub const AlSharifFundTipCountdown: BlockNumber = DAYS;
	pub const AlSharifFundTipFindersFee: Percent = Percent::from_percent(10);
	pub AlSharifFundTipReportDepositBase: Balance = deposit(1, 0);
	pub const AlSharifFundSevenDays: BlockNumber = 7 * DAYS;
	pub const AlSharifFundZeroDay: BlockNumber = 0;
	pub const OneDay: BlockNumber = DAYS;
	pub AlSharifFundBountyDepositBase: Balance = dollar(SETM);
	pub const AlSharifFundBountyDepositPayoutDelay: BlockNumber = DAYS;
	pub const AlSharifFundBountyUpdatePeriod: BlockNumber = 21 * DAYS;
	pub const AlSharifFundBountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub AlSharifFundBountyValueMinimum: Balance = 5 * dollar(SETM);
	pub AlSharifFundDataDepositPerByte: Balance = deposit(0, 1);
	pub const AlSharifFundMaximumReasonLength: u32 = 16384;
	pub const AlSharifFundMaxApprovals: u32 = 100;
}

type AlSharifFundTreasuryInstance = pallet_treasury::Instance3;
impl pallet_treasury::Config<AlSharifFundTreasuryInstance> for Runtime {
	type PalletId = AlSharifFundTreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EnsureRootOrHalfAlSharifFundCouncil; // TODO: Update to `EnsureAlSharifFoundationOrHalfAlSharifFundCouncil`
	type RejectOrigin = EnsureRootOrHalfAlSharifFundCouncil;  // TODO: Update to `EnsureAlSharifFoundationOrHalfAlSharifFundCouncil`
	type Event = Event;
	type OnSlash = AlSharifFundTreasury;
	type ProposalBond = AlSharifFundProposalBond;
	type ProposalBondMinimum = AlSharifFundProposalBondMinimum;
	type SpendPeriod = AlSharifFundSpendPeriod;
	type Burn = AlSharifFundBurn;
	type BurnDestination = ();
	type SpendFunds = AlSharifFundBounties;
	type WeightInfo = ();
	type MaxApprovals = AlSharifFundMaxApprovals;
}

type AlSharifFundBountiesInstance = pallet_bounties::Instance3;
impl pallet_bounties::Config<AlSharifFundBountiesInstance> for Runtime {
	type Event = Event;
	type BountyDepositBase = AlSharifFundBountyDepositBase;
	type BountyDepositPayoutDelay = AlSharifFundBountyDepositPayoutDelay;
	type BountyUpdatePeriod = AlSharifFundBountyUpdatePeriod;
	type BountyCuratorDeposit = AlSharifFundBountyCuratorDeposit;
	type BountyValueMinimum = AlSharifFundBountyValueMinimum;
	type DataDepositPerByte = AlSharifFundDataDepositPerByte;
	type MaximumReasonLength = AlSharifFundMaximumReasonLength;
	type WeightInfo = ();
}

type AlSharifFundTipsInstance = pallet_tips::Instance3;
impl pallet_tips::Config<AlSharifFundTipsInstance> for Runtime {
	type Event = Event;
	type DataDepositPerByte = AlSharifFundDataDepositPerByte;
	type MaximumReasonLength = AlSharifFundMaximumReasonLength;
	type Tippers = AlSharifFundCouncilProvider;
	type TipCountdown = AlSharifFundTipCountdown;
	type TipFindersFee = AlSharifFundTipFindersFee;
	type TipReportDepositBase = AlSharifFundTipReportDepositBase;
	type WeightInfo = ();
}


parameter_types! {
	pub const FoundationFundProposalBond: Permill = Permill::from_percent(5);
	pub FoundationFundProposalBondMinimum: Balance = dollar(SETM);
	pub const FoundationFundSpendPeriod: BlockNumber = 21 * DAYS;
	pub const FoundationFundBurn: Permill = Permill::from_percent(0);

	pub const FoundationFundTipCountdown: BlockNumber = DAYS;
	pub const FoundationFundTipFindersFee: Percent = Percent::from_percent(10);
	pub FoundationFundTipReportDepositBase: Balance = deposit(1, 0);
	pub const FoundationFundSevenDays: BlockNumber = 7 * DAYS;
	pub const FoundationFundZeroDay: BlockNumber = 0;
	pub const OneDay: BlockNumber = DAYS;
	pub FoundationFundBountyDepositBase: Balance = dollar(SETM);
	pub const FoundationFundBountyDepositPayoutDelay: BlockNumber = DAYS;
	pub const FoundationFundBountyUpdatePeriod: BlockNumber = 21 * DAYS;
	pub const FoundationFundBountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub FoundationFundBountyValueMinimum: Balance = 5 * dollar(SETM);
	pub FoundationFundDataDepositPerByte: Balance = deposit(0, 1);
	pub const FoundationFundMaximumReasonLength: u32 = 16384;
	pub const FoundationFundMaxApprovals: u32 = 100;
}

type FoundationFundTreasuryInstance = pallet_treasury::Instance4;
impl pallet_treasury::Config<FoundationFundTreasuryInstance> for Runtime {
	type PalletId = FoundationFundTreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EnsureRootOrHalfFoundationFundCouncil; // TODO: Update to `EnsureSetheumFoundationOrHalfFoundationFundCouncil`
	type RejectOrigin = EnsureRootOrHalfFoundationFundCouncil;  // TODO: Update to `EnsureSetheumFoundationOrHalfFoundationFundCouncil`
	type Event = Event;
	type OnSlash = FoundationFundTreasury;
	type ProposalBond = FoundationFundProposalBond;
	type ProposalBondMinimum = FoundationFundProposalBondMinimum;
	type SpendPeriod = FoundationFundSpendPeriod;
	type Burn = FoundationFundBurn;
	type BurnDestination = ();
	type SpendFunds = FoundationFundBounties;
	type WeightInfo = ();
	type MaxApprovals = FoundationFundMaxApprovals;
}

type FoundationFundBountiesInstance = pallet_bounties::Instance4;
impl pallet_bounties::Config<FoundationFundBountiesInstance> for Runtime {
	type Event = Event;
	type BountyDepositBase = FoundationFundBountyDepositBase;
	type BountyDepositPayoutDelay = AFoundationFundBountyDepositPayoutDelay;
	type BountyUpdatePeriod = FoundationFundBountyUpdatePeriod;
	type BountyCuratorDeposit = FoundationFundBountyCuratorDeposit;
	type BountyValueMinimum = FoundationFundBountyValueMinimum;
	type DataDepositPerByte = FoundationFundDataDepositPerByte;
	type MaximumReasonLength = FoundationFundMaximumReasonLength;
	type WeightInfo = ();
}

type FoundationFundTipsInstance = pallet_tips::Instance4;
impl pallet_tips::Config<FoundationFundTipsInstance> for Runtime {
	type Event = Event;
	type DataDepositPerByte = FoundationFundDataDepositPerByte;
	type MaximumReasonLength = FoundationFundMaximumReasonLength;
	type Tippers = FoundationFundCouncilProvider;
	type TipCountdown = FoundationFundTipCountdown;
	type TipFindersFee = FoundationFundTipFindersFee;
	type TipReportDepositBase = FoundationFundTipReportDepositBase;
	type WeightInfo = ();
}


parameter_types! {
	pub ConfigDepositBase: Balance = 10 * cent(SETM);
	pub FriendDepositFactor: Balance = cent(SETM);
	pub const MaxFriends: u16 = 9;
	pub RecoveryDeposit: Balance = 10 * cent(SETM);
}

impl pallet_recovery::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 2 * HOURS;
	pub const VotingPeriod: BlockNumber = HOURS;
	pub const FastTrackVotingPeriod: BlockNumber = HOURS;
	pub MinimumDeposit: Balance = 100 * cent(SETM);
	pub const EnactmentPeriod: BlockNumber = MINUTES;
	pub const CooloffPeriod: BlockNumber = MINUTES;
	pub PreimageByteDeposit: Balance = 10 * millicent(SETM);
	pub const InstantAllowed: bool = true;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
	type Proposal = Call;
	type Event = Event;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = EnsureRootOrHalfShuraCouncil;
	/// A majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = EnsureRootOrHalfShuraCouncil;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = EnsureRootOrAllShuraCouncil;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = EnsureRootOrTwoThirdsTechnicalCommittee;
	type InstantOrigin = EnsureRootOrAllTechnicalCommittee;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = EnsureRootOrTwoThirdsShuraCouncil;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EnsureRootOrAllTechnicalCommittee;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cooloff period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, ShuraCouncilInstance>;
	type CooloffPeriod = CooloffPeriod;
	type PreimageByteDeposit = PreimageByteDeposit;
	type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, ShuraCouncilInstance>;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = MaxVotes;
	//TODO: might need to customize weight for Setheum
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type MaxProposals = MaxProposals;
}

impl orml_auction::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type AuctionId = AuctionId;
	type Handler = AuctionManager;
	type WeightInfo = weights::orml_auction::WeightInfo<Runtime>;
}

impl pallet_randomness_collective_flip::Config for Runtime {}


// Create the runtime by composing the FRAME pallets that were previously configured.

// workaround for a weird bug in macro
use pallet_session::historical as pallet_session_historical;

// TODO: Implementation of `From` is preferred since it gives you `Into<_>` for free where the reverse isn't true.
// After this TODO will be resolved, remove the suppresion of `from-over-into` warnings in the Makefile.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		// Core
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 1,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 2,
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 3,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 4,
		Prices: module_prices::{Pallet, Storage, Call, Event<T>} = 110,
		Dex: module_dex::{Pallet, Storage, Call, Event<T>, Config<T>} = 111,

		// Utility
		Utility: pallet_utility::{Pallet, Call, Event} = 30,
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 31,
		Recovery: pallet_recovery::{Pallet, Call, Storage, Event<T>} = 32,
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 33,

		// ORML Core
		Auction: orml_auction::{Pallet, Storage, Call, Event<T>} = 100,
		OrmlNFT: orml_nft::{Pallet, Storage, Config<T>} = 102,

		// Governance
		ShuraCouncil: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 50,
		ShuraCouncilMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 51,
		FinancialCouncil: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 52,
		FinancialCouncilMembership: pallet_membership::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>} = 53,
		PublicFundCouncil: pallet_collective::<Instance3>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 52,
		PublicFundCouncilMembership: pallet_membership::<Instance3>::{Pallet, Call, Storage, Event<T>, Config<T>} = 53,
		AlSharifFundCouncil: pallet_collective::<Instance4>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 52,
		AlSharifFundCouncilMembership: pallet_membership::<Instance4>::{Pallet, Call, Storage, Event<T>, Config<T>} = 53,
		FoundationFundCouncil: pallet_collective::<Instance5>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 52,
		FoundationFundCouncilMembership: pallet_membership::<Instance5>::{Pallet, Call, Storage, Event<T>, Config<T>} = 53,
		TechnicalCommittee: pallet_collective::<Instance6>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 56,
		TechnicalCommitteeMembership: pallet_membership::<Instance6>::{Pallet, Call, Storage, Event<T>, Config<T>} = 57,

		Authority: orml_authority::{Pallet, Call, Storage, Event<T>, Origin<T>} = 70,
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 72,

		// Oracle
		//
		// NOTE: OperatorMembership must be placed after Oracle or else will have race condition on initialization
		SetheumOracle: orml_oracle::<Instance1>::{Pallet, Storage, Call, Event<T>} = 80,
		OperatorMembershipSetheum: pallet_membership::<Instance7>::{Pallet, Call, Storage, Event<T>, Config<T>} = 82,

		// SERP
		AuctionManager: auction_manager::{Pallet, Storage, Call, Event<T>, ValidateUnsigned} = 120,
		Loans: module_loans::{Pallet, Storage, Call, Event<T>} = 121,
		Setmint: serp_setmint::{Pallet, Storage, Call, Event<T>} = 122,
		SerpTreasury: serp_treasury::{Pallet, Storage, Call, Config, Event<T>} = 123,
		CdpTreasury: cdp_treasury::{Pallet, Storage, Call, Config, Event<T>} = 123,
		CdpEngine: cdp_engine::{Pallet, Storage, Call, Event<T>, Config, ValidateUnsigned} = 124,
		EmergencyShutdown: emergency_shutdown::{Pallet, Storage, Call, Event<T>} = 125,

		// Treasury
		Treasury: pallet_treasury::<Instance1>::{Pallet, Call, Storage, Config, Event<T>} = 20,
		PublicFund: pallet_treasury::<Instance2>::{Pallet, Call, Storage, Config, Event<T>} = 20,
		AlSharifFund: pallet_treasury::<Instance3>::{Pallet, Call, Storage, Config, Event<T>} = 20,
		FoundationFund: pallet_treasury::<Instance4>::{Pallet, Call, Storage, Config, Event<T>} = 20,
		// Bounties
		TreasuryBounties: pallet_bounties::<Instance1>::{Pallet, Call, Storage, Event<T>} = 21,
		PublicFundBounties: pallet_bounties::<Instance2>::{Pallet, Call, Storage, Event<T>} = 21,
		AlSharifFundBounties: pallet_bounties::<Instance3>::{Pallet, Call, Storage, Event<T>} = 21,
		FoundationFundBounties: pallet_bounties::<Instance4>::{Pallet, Call, Storage, Event<T>} = 21,
		// Tips
		TreasuryTips: pallet_tips::<Instance1>::{Pallet, Call, Storage, Event<T>} = 22,
		PublicFundTips: pallet_tips::<Instance2>::{Pallet, Call, Storage, Event<T>} = 22,
		AlSharifFundTips: pallet_tips::<Instance3>::{Pallet, Call, Storage, Event<T>} = 22,
		FoundationFundTips: pallet_tips::<Instance4>::{Pallet, Call, Storage, Event<T>} = 22,

		// Extras
		NFT: module_nft::{Pallet, Call, Event<T>} = 141,
		AirDrop: module_airdrop::{Pallet, Call, Storage, Event<T>, Config<T>} = 142,

		// Account lookup
		Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,

		// Tokens & Fees
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 6,
		Currencies: module_currencies::{Pallet, Call, Event<T>} = 7,
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>} = 8,
		TransactionPayment: module_transaction_payment::{Pallet, Call, Storage} = 9,
		TransactionPause: module_transaction_pause::{Pallet, Call, Storage, Event<T>} = 3,

		// Authorization
		Authority: orml_authority::{Pallet, Call, Event<T>, Origin<T>} = 10,

		// Identity
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 40,

		// Smart contracts
		EVM: module_evm::{Pallet, Config<T>, Call, Storage, Event<T>} = 21,
		EvmAccounts: module_evm_accounts::{Pallet, Call, Storage, Event<T>} = 20,
		EVMBridge: module_evm_bridge::{Pallet} = 22,
		EvmManager: module_evm_manager::{Pallet, Storage} = 183,

		// Consensus
		Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent} = 30,
		Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned} = 31,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event, ValidateUnsigned} = 32,
		Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>} = 33,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 34,
		Historical: pallet_session_historical::{Pallet} = 35,
		Offences: pallet_offences::{Pallet, Storage, Event} = 36,
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>} = 37,
		AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config} = 38,
	}
);

// Storage migrations required for runtime upgrade v7 -> v8
/// Babe config migration
impl pallet_babe::migrations::BabePalletPrefix for Runtime {
	fn pallet_prefix() -> &'static str {
		"Babe"
	}
}

pub struct BabeEpochConfigMigrations;
impl frame_support::traits::OnRuntimeUpgrade for BabeEpochConfigMigrations {
	fn on_runtime_upgrade() -> Weight {
		log::info!("Migrating Babe pallet - adding epoch config");
		pallet_babe::migrations::add_epoch_configuration::<Runtime>(
			BABE_GENESIS_EPOCH_CONFIG
		)
	}
}

/// Migrate from `PalletVersion` to the new `StorageVersion`
pub struct MigratePalletVersionToStorageVersion;
impl frame_support::traits::OnRuntimeUpgrade for MigratePalletVersionToStorageVersion {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		log::info!("Migrating PalletVersion to new StorageVersion pattern");
		frame_support::migrations::migrate_from_pallet_version_to_storage_version::<AllPalletsWithSystem>(
				&RocksDbWeight::get()
		)
	}
}

/// Migrate staking
pub struct MigratePalletStakingV5toV7;
impl frame_support::traits::OnRuntimeUpgrade for MigratePalletStakingV5toV7 {
	fn on_runtime_upgrade() -> Weight {
		log::info!("Migrating staking from V5 to V7");
		let mut weight = 0;
		weight += pallet_staking::migrations::v6::migrate::<Runtime>();
		weight += pallet_staking::migrations::v7::migrate::<Runtime>();
		weight
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct ConvertEthereumTx;

impl Convert<(Call, SignedExtra), Result<EthereumTransactionMessage, InvalidTransaction>> for ConvertEthereumTx {
	fn convert((call, extra): (Call, SignedExtra)) -> Result<EthereumTransactionMessage, InvalidTransaction> {
		match call {
			Call::EVM(module_evm::Call::eth_call(action, input, value, gas_limit, storage_limit, valid_until)) => {
				if System::block_number() > valid_until {
					return Err(InvalidTransaction::Stale);
				}

				let era: frame_system::CheckEra<Runtime> = extra.3;
				if era != frame_system::CheckEra::from(sp_runtime::generic::Era::Immortal) {
					// require immortal
					return Err(InvalidTransaction::BadProof);
				}

				let nonce: frame_system::CheckNonce<Runtime> = extra.4;
				// TODO: this is a hack access private nonce field
				// remove this after https://github.com/paritytech/substrate/pull/9810
				let nonce = nonce
					.using_encoded(|mut encoded| Compact::<Nonce>::decode(&mut encoded))
					.map_err(|_| InvalidTransaction::BadProof)?;

				let tip: module_transaction_payment::ChargeTransactionPayment<Runtime> = extra.6;
				let tip = tip.0;

				Ok(EthereumTransactionMessage {
					nonce: nonce.into(),
					tip,
					gas_limit,
					storage_limit,
					action,
					value,
					input,
					chain_id: ChainId::get(),
					genesis: System::block_hash(0),
					valid_until,
				})
			}
			_ => Err(InvalidTransaction::BadProof),
		}
	}
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	module_transaction_payment::ChargeTransactionPayment<Runtime>,
	module_evm::SetEvmOrigin<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPallets,
	(BabeEpochConfigMigrations, MigratePalletVersionToStorageVersion, MigratePalletStakingV5toV7)
>;

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
			sp_consensus_babe::BabeGenesisConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: BABE_GENESIS_EPOCH_CONFIG.c,
				genesis_authorities: Babe::authorities(),
				randomness: Babe::randomness(),
				allowed_slots: BABE_GENESIS_EPOCH_CONFIG.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot_number: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
			) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
			) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
				)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl orml_oracle_rpc_runtime_api::OracleApi<
		Block,
		DataProviderId,
		CurrencyId,
		TimeStampedPrice,
	> for Runtime {
		fn get_value(provider_id: DataProviderId ,key: CurrencyId) -> Option<TimeStampedPrice> {
			match provider_id {
				DataProviderId::Setheum => SetheumOracle::get_no_op(&key),
				DataProviderId::Aggregated => <AggregatedDataProvider as DataProviderExtended<_, _>>::get_no_op(&key)
			}
		}

		fn get_all_values(provider_id: DataProviderId) -> Vec<(CurrencyId, Option<TimeStampedPrice>)> {
			match provider_id {
				DataProviderId::Setheum => SetheumOracle::get_all_values(),
				DataProviderId::Aggregated => <AggregatedDataProvider as DataProviderExtended<_, _>>::get_all_values()
			}
		}
	}

	impl module_evm_rpc_runtime_api::EVMRuntimeRPCApi<Block, Balance> for Runtime {
		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: Balance,
			gas_limit: u64,
			storage_limit: u32,
			estimate: bool,
		) -> Result<CallInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as module_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			module_evm::runner::stack::Runner::<Runtime>::call(
				from,
				from,
				to,
				data,
				value,
				gas_limit,
				storage_limit,
				config.as_ref().unwrap_or(<Runtime as module_evm::Config>::config()),
			)
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: Balance,
			gas_limit: u64,
			storage_limit: u32,
			estimate: bool,
		) -> Result<CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as module_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			module_evm::runner::stack::Runner::<Runtime>::create(
				from,
				data,
				value,
				gas_limit,
				storage_limit,
				config.as_ref().unwrap_or(<Runtime as module_evm::Config>::config()),
			)
		}

		fn get_estimate_resources_request(extrinsic: Vec<u8>) -> Result<EstimateResourcesRequest, sp_runtime::DispatchError> {
			let utx = UncheckedExtrinsic::decode(&mut &*extrinsic)
				.map_err(|_| sp_runtime::DispatchError::Other("Invalid parameter extrinsic, decode failed"))?;

			let request = match utx.0.function {
				Call::EVM(module_evm::Call::call(to, data, value, gas_limit, storage_limit)) => {
					Some(EstimateResourcesRequest {
						from: None,
						to: Some(to),
						gas_limit: Some(gas_limit),
						storage_limit: Some(storage_limit),
						value: Some(value),
						data: Some(data),
					})
				}
				Call::EVM(module_evm::Call::create(data, value, gas_limit, storage_limit)) => {
					Some(EstimateResourcesRequest {
						from: None,
						to: None,
						gas_limit: Some(gas_limit),
						storage_limit: Some(storage_limit),
						value: Some(value),
						data: Some(data),
					})
				}
				_ => None,
			};

			request.ok_or(sp_runtime::DispatchError::Other("Invalid parameter extrinsic, not evm Call"))
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use orml_benchmarking::{list_benchmark as orml_list_benchmark};

			use module_nft::benchmarking::Pallet as NftBench;

			let mut list = Vec::<BenchmarkList>::new();


			list_benchmark!(list, extra, module_nft, NftBench::<Runtime>);

			orml_list_benchmark!(list, extra, module_dex, benchmarking::dex);
			orml_list_benchmark!(list, extra, auction_manager, benchmarking::auction_manager);
			orml_list_benchmark!(list, extra, cdp_engine, benchmarking::cdp_engine);
			orml_list_benchmark!(list, extra, emergency_shutdown, benchmarking::emergency_shutdown);
			orml_list_benchmark!(list, extra, module_evm, benchmarking::evm);
			orml_list_benchmark!(list, extra, serp_setmint, benchmarking::serp_setmint);
			orml_list_benchmark!(list, extra, cdp_treasury, benchmarking::cdp_treasury);
			orml_list_benchmark!(list, extra, module_transaction_pause, benchmarking::transaction_pause);
			orml_list_benchmark!(list, extra, module_transaction_payment, benchmarking::transaction_payment);
			orml_list_benchmark!(list, extra, module_prices, benchmarking::prices);
			orml_list_benchmark!(list, extra, module_evm_accounts, benchmarking::evm_accounts);
			orml_list_benchmark!(list, extra, module_currencies, benchmarking::currencies);

			orml_list_benchmark!(list, extra, orml_tokens, benchmarking::tokens);
			orml_list_benchmark!(list, extra, orml_auction, benchmarking::auction);

			orml_list_benchmark!(list, extra, orml_authority, benchmarking::authority);
			orml_list_benchmark!(list, extra, orml_oracle, benchmarking::oracle);

			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};
			use orml_benchmarking::{add_benchmark as orml_add_benchmark};

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {}

			use module_nft::benchmarking::Pallet as NftBench;


			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				// frame_system::Number::<Runtime>::hashed_key().to_vec(),
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			add_benchmark!(params, batches, pallet_balances, Balances);
			add_benchmark!(params, batches, pallet_timestamp, Timestamp);

			add_benchmark!(params, batches, module_nft, NftBench::<Runtime>);
			orml_add_benchmark!(params, batches, module_dex, benchmarking::dex);
			orml_add_benchmark!(params, batches, auction_manager, benchmarking::auction_manager);
			orml_add_benchmark!(params, batches, cdp_engine, benchmarking::cdp_engine);
			orml_add_benchmark!(params, batches, emergency_shutdown, benchmarking::emergency_shutdown);
			orml_add_benchmark!(params, batches, module_evm, benchmarking::evm);
			orml_add_benchmark!(params, batches, serp_setmint, benchmarking::serp_setmint);
			orml_add_benchmark!(params, batches, cdp_treasury, benchmarking::cdp_treasury);
			orml_add_benchmark!(params, batches, module_transaction_pause, benchmarking::transaction_pause);
			orml_add_benchmark!(params, batches, module_transaction_payment, benchmarking::transaction_payment);
			orml_add_benchmark!(params, batches, module_prices, benchmarking::prices);
			orml_add_benchmark!(params, batches, module_evm_accounts, benchmarking::evm_accounts);
			orml_add_benchmark!(params, batches, module_currencies, benchmarking::currencies);

			orml_add_benchmark!(params, batches, orml_tokens, benchmarking::tokens);
			orml_add_benchmark!(params, batches, orml_auction, benchmarking::auction);

			orml_add_benchmark!(params, batches, orml_authority, benchmarking::authority);
			orml_add_benchmark!(params, batches, orml_oracle, benchmarking::oracle);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}
