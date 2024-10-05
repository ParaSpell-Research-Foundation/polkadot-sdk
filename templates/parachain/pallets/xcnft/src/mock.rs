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

}
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
	type Nonce = u64;
	type Block = MockBlock<Test>;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = RocksDbWeight;
	type AccountData = AccountData<u128>;

}
pub type XcmRouter = ();

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const proposal_time_in_blocks_parameter: u32 = 200000;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type ProposalTimeInBlocks = proposal_time_in_blocks_parameter;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
