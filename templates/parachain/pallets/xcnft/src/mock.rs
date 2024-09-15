use cumulus_primitives_core::ParaId;
use sp_core::H256;

use frame_support::{derive_impl, weights::constants::RocksDbWeight, parameter_types, traits::Everything};
use frame_system::{mocking::MockBlock, GenesisConfig};
use sp_runtime::{traits::ConstU64, traits::BlakeTwo256, BuildStorage, traits::IdentityLookup};

use pallet_balances::AccountData;
use frame_system as system;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
#[frame_support::runtime]
mod test_runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type XcnftModule = crate;
	#[runtime::pallet_index(2)]
	pub type Balances = pallet_balances;
}
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
	type Nonce = u64;
	type Block = MockBlock<Test>;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = RocksDbWeight;
	type AccountData = AccountData<u128>;

}

impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
	pub const RegistryStringLimit: u32 = 255;
	pub const RegistryJsonLimit: u32 = 2048;
	pub const RegistryCollectionLimit: u32 = 255;
	pub const RegistryParaIDLimit: u32 = 9999;
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 10;
	pub const MaxParachains: u32 = 100;
	pub const MaxPayloadSize: u32 = 1024;

	//Parachain ID - this has to be set by parachain team
	pub const RegistryParaId: ParaId = ParaId::new(2);

	//Max Amount of collections we will store per parachain
	pub const RegistryPerParachainCollectionLimit: u8 = 255;

	//Max Amount of NFTs we will store per parachain
	pub const RegistryNFTsPerParachainLimit: u32 = 255*255;
}
pub type XcmRouter = ();

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type Currency = Balances;
	type StringLimit = RegistryStringLimit;
	type JsonLimit = RegistryJsonLimit;
	type CollectionLimit = RegistryCollectionLimit;
	type ParaIDLimit = RegistryParaIDLimit;
	type CollectionsPerParachainLimit = RegistryPerParachainCollectionLimit;
	type NFTsPerParachainLimit = RegistryNFTsPerParachainLimit;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
