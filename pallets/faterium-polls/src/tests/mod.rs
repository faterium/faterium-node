//! The crate's tests.

mod voting;

use crate::{self as pallet_faterium_polls, *};
use codec::Encode;
use frame_support::{
	assert_noop, assert_ok, parameter_types,
	traits::{ConstU16, ConstU32, ConstU64, EqualPrivilegeOnly, Hooks},
	weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	DispatchResult, Perbill,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type PollIndex = u64;
type Balance = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Scheduler: pallet_scheduler,
		FateriumPolls: pallet_faterium_polls,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(1_000_000));
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Test {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = frame_system::EnsureRoot<u64>;
	type MaxScheduledPerBlock = ();
	type WeightInfo = ();
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type PreimageProvider = ();
	type NoPreimagePostponement = ();
}

impl pallet_balances::Config for Test {
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type MaxLocks = ConstU32<10>;
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type WeightInfo = ();
}

impl pallet_faterium_polls::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type AssetId = u32;
	type Currency = pallet_balances::Pallet<Self>;
	type PollIndex = PollIndex;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	t.into()
}

fn next_block() {
	System::set_block_number(System::block_number() + 1);
	Scheduler::on_initialize(System::block_number());
	FateriumPolls::begin_block(System::block_number());
}

fn fast_forward_to(n: u64) {
	while System::block_number() < n {
		next_block();
	}
}

fn set_balance_proposal(value: Balance) -> Vec<u8> {
	Call::Balances(pallet_balances::Call::set_balance { who: 42, new_free: value, new_reserved: 0 })
		.encode()
}

fn propose_set_balance(who: u64, value: Balance) -> DispatchResult {
	set_balance_proposal(value);
	FateriumPolls::create_poll(
		Origin::signed(who),
		vec![],
		3,
		PollCurrency::Native,
		vec![],
		RewardSettings::None,
		100,
		10,
		20,
	)
}

fn begin_poll() -> PollIndex {
	System::set_block_number(0);
	assert_ok!(propose_set_balance(1, 2));
	fast_forward_to(2);
	0
}

#[test]
fn params_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(FateriumPolls::poll_count(), 0);
		assert_eq!(Balances::free_balance(0), 0);
		assert_eq!(Balances::total_issuance(), 0);
	});
}

fn tally(pid: PollIndex) -> Tally<Balance> {
	FateriumPolls::poll_details_of(pid).unwrap().tally
}
