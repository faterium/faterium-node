//! # Faterium Polls Pallet
//!
//! A way for creators to decide what community wants and raise money for a project or idea.
//! Authors themselves determine which currency they want to use for voting and what percentage
//! they will receive after the end of the poll.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod tests;
mod types;

pub use pallet::*;
pub use types::*;

use codec::HasCompact;
use frame_support::{
	traits::{schedule::Named as ScheduleNamed, Currency, LockableCurrency, ReservableCurrency},
	weights::Weight,
};
use frame_system::Config as SystemConfig;
// use pallet_democracy::Config as DemocracyConfig;
use sp_runtime::traits::{AtLeast32BitUnsigned, Saturating, StaticLookup};

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	// TODO: Remove without_storage_info macro.
	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The units in which we record balances.
		type Balance: Member
			+ Parameter
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TypeInfo
			+ From<DepositBalanceOf<Self>>;

		/// Identifier for the class of asset.
		type AssetId: Member
			+ Parameter
			+ Default
			+ Copy
			+ HasCompact
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TypeInfo;

		/// Currency type for this pallet.
		type Currency: ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		/// Identifier and index for polls.
		type PollIndex: Member
			+ Parameter
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ HasCompact
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TypeInfo;

		/// The Scheduler.
		type Scheduler: ScheduleNamed<Self::BlockNumber, Self::Call, Self::PalletsOrigin>;

		/// Overarching type of all pallets origins.
		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
	}

	/// The number of polls that have been made so far.
	#[pallet::storage]
	#[pallet::getter(fn poll_count)]
	pub type PollCount<T: Config> = StorageValue<_, T::PollIndex, ValueQuery>;

	/// Details of a poll.
	#[pallet::storage]
	pub(super) type Polls<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::PollIndex,
		PollDetails<
			T::PollIndex,
			T::Balance,
			T::AccountId,
			T::AssetId,
			<T as frame_system::Config>::BlockNumber,
		>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A poll was created.
		Created { poll_id: T::PollIndex, creator: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Weight: see `begin_block`
		fn on_initialize(n: T::BlockNumber) -> Weight {
			Self::begin_block(n)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn create_poll(
			origin: OriginFor<T>,
			_ipfs_cid: IpfsCid,
			_options_count: u8,
			_currency: PollCurrency<T::AssetId>,
			_beneficiaries: Vec<(AccountIdLookupOf<T>, u32)>,
			_reward_settings: RewardSettings,
			_goal: T::Balance,
			_poll_start: <T as frame_system::Config>::BlockNumber,
			_poll_end: <T as frame_system::Config>::BlockNumber,
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let creator = ensure_signed(origin)?;

			// TODO: Check and validate params

			// Update storage.
			let poll_id = <PollCount<T>>::get();

			// TODO: Create Poll struct
			// TODO: Save Poll to the storage

			// TODO: Update storage and save a new poll

			// Updates poll count.
			let mut next_id = poll_id;
			next_id.saturating_inc();
			<PollCount<T>>::put(next_id);

			// Emit an event.
			Self::deposit_event(Event::Created { poll_id, creator });
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn vote(
			origin: OriginFor<T>,
			#[pallet::compact] _poll_id: T::PollIndex,
			_vote: AccountVote<T::Balance>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// TODO: Validate vote struct
			// TODO: Get and check poll by poll_id

			// TODO: Remove error and emit an event
			Err(Error::<T>::NoneValue.into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn begin_block(_now: T::BlockNumber) -> Weight {
		let weight = Weight::zero();
		weight
	}
}
