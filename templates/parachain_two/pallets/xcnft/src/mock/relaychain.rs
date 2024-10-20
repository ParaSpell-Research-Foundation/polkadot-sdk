
#![cfg(test)]
use crate::{
	self as pallet_parachain_xcnft,
	test::relay::currency::{CENTS, MILLICENTS},
};
use cumulus_primitives_core::relay_chain::CandidateHash;
use frame_support::{assert_ok, traits::AsEnsureOriginWithArg};
use polkadot_runtime_common::{
	paras_sudo_wrapper,
	xcm_sender::{ChildParachainRouter, ExponentialPrice},
};
use sp_runtime::BuildStorage;
use crate::test::*;
use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	pallet_prelude::{DispatchResult, Get},
	parameter_types,
	traits::{
		ConstU128, ConstU16, ConstU32, ConstU64, Currency, Everything, GenesisBuild, Nothing,
		ProcessMessage, ProcessMessageError,
	},
	weights::{constants::RocksDbWeight, IdentityFee, Weight, WeightMeter},
	
};
use frame_support::traits::KeyOwnerProofSystem;
use polkadot_parachain::primitives::ValidationCode;
use sp_runtime::traits::AccountIdConversion;

use crate::test::relay::currency::DOLLARS;
use cumulus_primitives_core::{
	relay_chain::{AuthorityDiscoveryId, SessionIndex, ValidatorIndex},
	ChannelStatus, GetChannelInfo, ParaId,
};
use polkadot_runtime_parachains::disputes::SlashingHandler;
use frame_support::traits::ValidatorSetWithIdentification;
use frame_system::{EnsureRoot, EnsureRootWithSuccess, EnsureSigned};
use sp_runtime::{transaction_validity::TransactionPriority, Permill};
use std::{cell::RefCell, collections::HashMap};
pub mod currency {
	use node_primitives::Balance;

	pub const MILLICENTS: Balance = 1_000_000_000;
	pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
	pub const DOLLARS: Balance = 100 * CENTS;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
	}
}
use sp_runtime::{
	generic,
	testing::Header,
	traits::{AccountIdLookup, BlakeTwo256, IdentityLookup},
	BoundedVec, DispatchError, MultiSignature,
};
pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as sp_runtime::traits::Verify>::Signer;
pub type AccountId = <AccountPublic as sp_runtime::traits::IdentifyAccount>::AccountId;
use crate::test::para::ParachainInfo;
use frame_support::traits::ValidatorSet;
use sp_core::H256;
use xcm_builder::{EnsureXcmOrigin, NativeAsset};
//pub type AccountId = u64;
use pallet_nfts::PalletFeatures;

pub use polkadot_runtime_parachains::{
	configuration,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
	origin, shared,disputes, inclusion, paras, scheduler, session_info, assigner, assigner_parachains, assigner_on_demand,hrmp,
	dmp as parachains_dmp, paras::ParaGenesisArgs, schedule_para_initialize,
	disputes::slashing as parachains_slashing,
};
use primitives::ValidatorId;
pub type BlockNumber = u64;
pub type Index = u32;
use xcm::v3::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, ChildParachainAsNative,
	ChildParachainConvertsVia, CurrencyAdapter as XcmCurrencyAdapter, FixedWeightBounds,
	IsConcrete, SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
	TakeWeightCredit, UsingComponents,
};
use xcm_executor::XcmExecutor;
use sp_runtime::KeyTypeId;
type Origin = <Test as frame_system::Config>::RuntimeOrigin;
use crate::Config;

type Balance = u128;

pub fn root_user() -> Origin {
	RuntimeOrigin::root()
}
pub fn who(who: AccountId) -> Origin {
	RuntimeOrigin::signed(who)
}

pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Test>,
	frame_system::CheckSpecVersion<Test>,
	frame_system::CheckTxVersion<Test>,
	frame_system::CheckGenesis<Test>,
	frame_system::CheckEra<Test>,
	frame_system::CheckNonce<Test>,
	frame_system::CheckWeight<Test>,
);

type UncheckedExtrinsics = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsics,
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances,
		ParasOrigin: origin,
		MessageQueue: pallet_message_queue,
		XcmPallet: pallet_xcm,
		ParaInclusion: inclusion,
		Paras: paras,
		Xnft: pallet_xnft,
		Disputes: disputes,
		Scheduler: scheduler,
		Configuration: configuration,
		ParasShared: shared,
		ParasSudoWrapperCall:paras_sudo_wrapper,
		Dmp: parachains_dmp,
		NFT:pallet_nfts,
		CumulusXcm: cumulus_pallet_xcm,
		DmpQueue: cumulus_pallet_dmp_queue,
		XcmpQueue: cumulus_pallet_xcmp_queue,
		Hrmp: hrmp,
		Assigner: assigner,
		ParachainsAssigner: assigner_parachains,
		OnDemandAssigner: assigner_on_demand,
		//ParasSlashing: parachains_slashing,
		Offences: pallet_offences,
	}

);
impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsics;
	type OverarchingCall = RuntimeCall;
}
impl paras_sudo_wrapper::Config for Test {}
impl assigner_on_demand::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type TrafficDefaultValue = ();
	type WeightInfo = assigner_on_demand::TestWeightInfo;
}

pub struct ChannelInfo;
impl GetChannelInfo for ChannelInfo {
	fn get_channel_status(_id: ParaId) -> ChannelStatus {
		ChannelStatus::Ready(10, 10)
	}
	fn get_channel_max(_id: ParaId) -> Option<usize> {
		Some(usize::max_value())
	}
}
impl assigner_parachains::Config for Test {}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ();
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const CollectionDeposit: Balance = 100 * DOLLARS;
	pub const ItemDeposit: Balance = 1 * DOLLARS;
	pub const KeyLimit: u32 = 32;
	pub const ValueLimit: u32 = 256;
	pub const ApprovalsLimit: u32 = 20;
	pub const ItemAttributesApprovalsLimit: u32 = 20;
	pub const MaxTips: u32 = 10;

}
impl pallet_offences::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = ();
	type OnOffenceHandler = ();
}
parameter_types! {
	pub Features: PalletFeatures = PalletFeatures::all_enabled();
	pub const MaxAttributesPerCall: u32 = 10;
}

impl pallet_nfts::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Locker = ();
	type CollectionDeposit = ConstU64<2>;
	type ItemDeposit = ConstU64<1>;
	type MetadataDepositBase = ConstU64<1>;
	type AttributeDepositBase = ConstU64<1>;
	type DepositPerByte = ConstU64<1>;
	type StringLimit = ConstU32<50>;
	type KeyLimit = ConstU32<50>;
	type ValueLimit = ConstU32<50>;
	type ApprovalsLimit = ConstU32<10>;
	type ItemAttributesApprovalsLimit = ConstU32<2>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = ();
	type MaxAttributesPerCall = ConstU32<2>;
	type Features = Features;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as sp_runtime::traits::Verify>::Signer;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 10;
}

impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = u64;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type MaxHolds = ();
	type RuntimeHoldReason = RuntimeHoldReason;
}
parameter_types! {
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

pub struct TestNextSessionRotation;

impl frame_support::traits::EstimateNextSessionRotation<u32> for TestNextSessionRotation {
	fn average_session_length() -> u32 {
		10
	}

	fn estimate_current_session_progress(_now: u32) -> (Option<Permill>, Weight) {
		(None, Weight::zero())
	}

	fn estimate_next_session_rotation(_now: u32) -> (Option<u32>, Weight) {
		(None, Weight::zero())
	}
}

impl paras::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = polkadot_runtime_parachains::paras::TestWeightInfo;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ParaInclusion;
	type NextSessionRotation = ();
	type OnNewHead = ();
}

thread_local! {
	pub static BACKING_REWARDS: RefCell<HashMap<ValidatorIndex, usize>>
		= RefCell::new(HashMap::new());

	pub static AVAILABILITY_REWARDS: RefCell<HashMap<ValidatorIndex, usize>>
		= RefCell::new(HashMap::new());
}

pub struct TestRewardValidators;

impl inclusion::RewardValidators for TestRewardValidators {
	fn reward_backing(v: impl IntoIterator<Item = ValidatorIndex>) {
		BACKING_REWARDS.with(|r| {
			let mut r = r.borrow_mut();
			for i in v {
				*r.entry(i).or_insert(0) += 1;
			}
		})
	}
	fn reward_bitfields(v: impl IntoIterator<Item = ValidatorIndex>) {
		AVAILABILITY_REWARDS.with(|r| {
			let mut r = r.borrow_mut();
			for i in v {
				*r.entry(i).or_insert(0) += 1;
			}
		})
	}
}

impl inclusion::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type DisputesHandler = Disputes;
	type RewardValidators = TestRewardValidators;
	type MessageQueue = MessageQueue;
}

pub struct ValidatorIdOf;
impl sp_runtime::traits::Convert<AccountId, Option<AccountId>> for ValidatorIdOf {
	fn convert(a: AccountId) -> Option<AccountId> {
		Some(a)
	}
}

pub struct MockValidatorSet;

impl ValidatorSet<AccountId> for MockValidatorSet {
	type ValidatorId = AccountId;
	type ValidatorIdOf = ValidatorIdOf;
	fn session_index() -> SessionIndex {
		0
	}
	fn validators() -> Vec<Self::ValidatorId> {
		Vec::new()
	}
}

pub struct FoolIdentificationOf;
impl sp_runtime::traits::Convert<AccountId, Option<()>> for FoolIdentificationOf {
	fn convert(_: AccountId) -> Option<()> {
		Some(())
	}
}

impl ValidatorSetWithIdentification<AccountId> for MockValidatorSet {
	type Identification = ();
	type IdentificationOf = FoolIdentificationOf;
}
impl assigner::Config for Test {
	type ParachainsAssignmentProvider = ParachainsAssigner;
	type OnDemandAssignmentProvider = OnDemandAssigner;
}
impl scheduler::Config for Test {
	type AssignmentProvider = Assigner;
}

impl SlashingHandler<BlockNumber> for Test {
	fn punish_for_invalid(
		session: SessionIndex,
		_: CandidateHash,
		losers: impl IntoIterator<Item = ValidatorIndex>,
		backers: impl IntoIterator<Item = ValidatorIndex>,
	) {
		PUNISH_VALIDATORS_FOR
			.with(|r| r.borrow_mut().push((session, losers.into_iter().collect())));
		PUNISH_BACKERS_FOR.with(|r| r.borrow_mut().push((session, backers.into_iter().collect())));
	}

	fn punish_against_valid(
		session: SessionIndex,
		_: CandidateHash,
		losers: impl IntoIterator<Item = ValidatorIndex>,
		_backers: impl IntoIterator<Item = ValidatorIndex>,
	) {
		PUNISH_VALIDATORS_AGAINST
			.with(|r| r.borrow_mut().push((session, losers.into_iter().collect())))
	}

	fn initializer_initialize(_now: BlockNumber) -> Weight {
		Weight::zero()
	}

	fn initializer_finalize() {}

	fn initializer_on_new_session(_: SessionIndex) {}
}


impl disputes::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RewardValidators = Self;
	type SlashingHandler = Self;
	type WeightInfo = disputes::TestWeightInfo;
}
// impl parachains_slashing::Config for Test {
// 	type KeyOwnerProofSystem = ();
// 	type KeyOwnerProof =
// 		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, ValidatorId)>>::Proof;
// 	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
// 		KeyTypeId,
// 		ValidatorId,
// 	)>>::IdentificationTuple;
// 	type HandleReports = parachains_slashing::SlashingReportHandler<
// 		Self::KeyOwnerIdentification,
// 		Offences,
// 		ReportLongevity,
// 	>;
// 	type WeightInfo = parachains_slashing::TestWeightInfo;
// 	type BenchmarkingConfig = parachains_slashing::BenchConfig<200>;
// }

thread_local! {
	pub static REWARD_VALIDATORS: RefCell<Vec<(SessionIndex, Vec<ValidatorIndex>)>> = RefCell::new(Vec::new());
	pub static PUNISH_VALIDATORS_FOR: RefCell<Vec<(SessionIndex, Vec<ValidatorIndex>)>> = RefCell::new(Vec::new());
	pub static PUNISH_VALIDATORS_AGAINST: RefCell<Vec<(SessionIndex, Vec<ValidatorIndex>)>> = RefCell::new(Vec::new());
	pub static PUNISH_BACKERS_FOR: RefCell<Vec<(SessionIndex, Vec<ValidatorIndex>)>> = RefCell::new(Vec::new());
}

impl disputes::RewardValidators for Test {
	fn reward_dispute_statement(
		session: SessionIndex,
		validators: impl IntoIterator<Item = ValidatorIndex>,
	) {
		REWARD_VALIDATORS.with(|r| r.borrow_mut().push((session, validators.into_iter().collect())))
	}
}


thread_local! {
	pub static DISCOVERY_AUTHORITIES: RefCell<Vec<AuthorityDiscoveryId>> = RefCell::new(Vec::new());
}

pub fn discovery_authorities() -> Vec<AuthorityDiscoveryId> {
	DISCOVERY_AUTHORITIES.with(|r| r.borrow().clone())
}

impl session_info::AuthorityDiscoveryConfig for Test {
	fn authorities() -> Vec<AuthorityDiscoveryId> {
		discovery_authorities()
	}
}

impl session_info::Config for Test {
	type ValidatorSet = MockValidatorSet;
}

impl hrmp::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = pallet_balances::Pallet<Test>;
	type WeightInfo = hrmp::TestWeightInfo;
	type ChannelManager = EnsureRoot<AccountId>;
}

impl shared::Config for Test {}

impl configuration::Config for Test {
	type WeightInfo = configuration::TestWeightInfo;
}

impl cumulus_pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ChannelInfo;
	type VersionWrapper = ();
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = ();
	type WeightInfo = ();
	type PriceForSiblingDelivery = ();
}
parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub const AnyNetwork: Option<NetworkId> = None;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorMultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

pub type SovereignAccountOf = (ChildParachainConvertsVia<ParaId, AccountId>,);
parameter_types! {
	pub const MaxAssetsForTransfer: usize = 2;
	pub const BaseXcmWeight: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
}

parameter_types! {
	/// The amount of weight an XCM operation takes. This is a safe overestimate.
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = Concrete(RelayLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

pub type XcmRouter = (
	// Only one router so far - use DMP to communicate with child parachains.
	ChildParachainRouter<
		Test,
		XcmPallet,
		ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, Dmp>,
	>,
);

pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecutionFrom<Everything>);

parameter_types! {
	pub const UnitWeightCost: Weight = Weight::from_parts(10, 10);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}
impl cumulus_pallet_dmp_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

impl parachains_dmp::Config for Test {}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type OriginConverter = ();
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = ();
	type SubscriptionService = ();
	type PalletInstancesInfo = ();
	type AssetTransactor = ();
	type Trader = ();
	type FeeManager = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = ();
	
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, AnyNetwork>;

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
}

impl origin::Config for Test {}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Weight::from_parts(1_000_000_000, 1_000_000);
	pub const MessageQueueHeapSize: u32 = 65_536;
	pub const MessageQueueMaxStale: u32 = 16;
}

pub struct MessageProcessor;
impl ProcessMessage for MessageProcessor {
	type Origin = AggregateMessageOrigin;

	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		meter: &mut WeightMeter,
		id: &mut [u8; 32],
	) -> Result<bool, ProcessMessageError> {
		let para = match origin {
			AggregateMessageOrigin::Ump(UmpQueueId::Para(para)) => para,
		};
		xcm_builder::ProcessXcmMessage::<Junction, xcm_executor::XcmExecutor<XcmConfig>, RuntimeCall>::process_message(
			message,
			Junction::Parachain(para.into()),
			meter,
			id,
		)
	}
}

impl pallet_message_queue::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Size = u32;
	type HeapSize = MessageQueueHeapSize;
	type MaxStale = MessageQueueMaxStale;
	type ServiceWeight = MessageQueueServiceWeight;
	type MessageProcessor = MessageProcessor;
	type QueueChangeHandler = ();
	type QueuePausedQuery = ();
	type WeightInfo = ();
}

impl pallet_xnft::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmSender = XcmRouter;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
}

pub fn new_test_ext(state: MockGenesisConfig) -> sp_io::TestExternalities {
	use sp_keystore::{testing::MemoryKeystore, KeystoreExt, KeystorePtr};
	use sp_std::sync::Arc;

	sp_tracing::try_init_simple();

	BACKING_REWARDS.with(|r| r.borrow_mut().clear());
	AVAILABILITY_REWARDS.with(|r| r.borrow_mut().clear());

	let mut t = state.system.build_storage().unwrap();
	state.configuration.assimilate_storage(&mut t).unwrap();
	state.paras.assimilate_storage(&mut t).unwrap();


	let mut ext: sp_io::TestExternalities = t.into();
	ext.register_extension(KeystoreExt(Arc::new(MemoryKeystore::new()) as KeystorePtr));

	ext
}
pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		storage.into()
	}
}

pub fn assert_last_events<E>(generic_events: E)
where
	E: DoubleEndedIterator<Item = RuntimeEvent> + ExactSizeIterator,
{
	for (i, (got, want)) in frame_system::Pallet::<Test>::events()
		.into_iter()
		.rev()
		.map(|e| e.event)
		.zip(generic_events.rev().map(<Test as frame_system::Config>::RuntimeEvent::from))
		.rev()
		.enumerate()
	{
		assert_eq!((i, got), (i, want));
	}
}

#[derive(Default)]
pub struct MockGenesisConfig {
	pub system: frame_system::GenesisConfig<Test>,
	pub configuration: configuration::GenesisConfig<Test>,
	pub paras: paras::GenesisConfig<Test>,
}

pub fn sudo_establish_hrmp_channel(
	sender: ParaId,
	recipient: ParaId,
	max_capacity: u32,
	max_message_size: u32,
) -> DispatchResult {
	Hrmp::init_open_channel(sender, recipient, max_capacity, max_message_size);
	Hrmp::accept_open_channel(recipient, sender);
	Ok(())
}

pub(crate) fn register_parachain_with_balance(id: ParaId, balance: Balance) {
	let validation_code: ValidationCode = vec![1].into();
	assert_ok!(schedule_para_initialize::<Test>(
		id,
		paras::ParaGenesisArgs {
			para_kind: paras::ParaKind::Parachain,
			genesis_head: vec![1].into(),
			validation_code: validation_code.clone(),
		},
	));

	assert_ok!(Paras::add_trusted_validation_code(RuntimeOrigin::root(), validation_code));
	<Test as hrmp::Config>::Currency::make_free_balance_be(
		&id.into_account_truncating(),
		balance.try_into().unwrap(),
	);
}
