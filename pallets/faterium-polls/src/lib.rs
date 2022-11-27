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
	ensure,
	traits::{
		schedule::Named as ScheduleNamed, Currency, Get, LockableCurrency, ReservableCurrency,
	},
	weights::Weight,
	PalletId,
};
use frame_system::Config as SystemConfig;
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32BitUnsigned, CheckedDiv, Saturating, StaticLookup, Zero,
	},
	ArithmeticError, DispatchError, DispatchResult,
};

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
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

		/// The polls' pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Maximum amount of funds that should be placed in a deposit for making a proposal.
		#[pallet::constant]
		type MaxPollBeneficiaries: Get<u32>;
	}

	/// The number of polls that have been made so far.
	#[pallet::storage]
	#[pallet::getter(fn poll_count)]
	pub type PollCount<T: Config> = StorageValue<_, T::PollIndex, ValueQuery>;

	/// Details of polls.
	#[pallet::storage]
	#[pallet::getter(fn poll_details_of)]
	pub(super) type PollDetailsOf<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::PollIndex,
		PollDetails<T::Balance, T::AccountId, T::AssetId, <T as frame_system::Config>::BlockNumber>,
	>;

	/// All votes for a particular voter.
	#[pallet::storage]
	#[pallet::getter(fn voting_of)]
	pub type VotingOf<T: Config> =
		StorageMap<_, Blake2_128Concat, (T::AccountId, T::PollIndex), AccountVotes<T::Balance>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A poll was created.
		Created { poll_id: T::PollIndex, creator: T::AccountId },
		/// A poll has been cancelled.
		Cancelled { poll_id: T::PollIndex },
		/// An account has voted in a poll.
		Voted { voter: T::AccountId, poll_id: T::PollIndex, votes: Votes<T::Balance> },
		/// An account has voted in a poll.
		VoteRemoved { voter: T::AccountId, poll_id: T::PollIndex },
		/// Voter/beneficiary collected his vote/interest.
		Collected { who: T::AccountId, poll_id: T::PollIndex, amount: T::Balance },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Too high a balance was provided that the account cannot afford.
		InsufficientFunds,
		/// The account currently has votes attached to it and the operation cannot succeed until
		/// these are removed through `remove_vote`.
		VotesExist,
		/// Invalid poll details given.
		InvalidPollDetails,
		/// Vote given for an invalid poll.
		PollInvalid,
		/// Invalid number of votes given for a poll.
		InvalidPollVotes,
		/// Can't collect from Ongoing Poll.
		CollectOnOngoingPoll,
		/// Account is neither a voter nor a beneficiary.
		AccountNotVoterOrBeneficiary,
		/// Nothing to collect or already collected
		NothingToCollect,
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
			ipfs_cid: IpfsCid,
			beneficiaries: Vec<(AccountIdLookupOf<T>, u32)>,
			reward_settings: RewardSettings,
			goal: T::Balance,
			options_count: u8,
			currency: PollCurrency<T::AssetId>,
			start: <T as frame_system::Config>::BlockNumber,
			end: <T as frame_system::Config>::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Lookup for accounts of beneficiaries.
			let mut benfs = vec![];
			for b in beneficiaries {
				let account = T::Lookup::lookup(b.0)?;
				benfs.push(Beneficiary::new(account, b.1));
			}
			// Create poll details struct.
			let poll = PollDetails::new(
				who.clone(),
				ipfs_cid,
				benfs,
				reward_settings,
				goal,
				options_count,
				currency,
				start,
				end,
			);
			// Validate poll details.
			ensure!(poll.validate(), Error::<T>::InvalidPollDetails);
			// Get next poll_id from storage.
			let poll_id = PollCount::<T>::get();
			PollDetailsOf::<T>::insert(poll_id, poll);
			// Updates poll count.
			let mut next_id = poll_id;
			next_id.saturating_inc();
			PollCount::<T>::put(next_id);
			// Emit an event.
			Self::deposit_event(Event::Created { poll_id, creator: who });
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn emergency_cancel(
			origin: OriginFor<T>,
			#[pallet::compact] poll_id: T::PollIndex,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// TODO: Get and check poll by poll_id
			// TODO: Check if origin is entitled to cancel the poll
			// TODO: Cancel dispatch
			// TODO: Update Polls storage and set status to Cancelled

			Self::deposit_event(Event::<T>::Cancelled { poll_id });
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn vote(
			origin: OriginFor<T>,
			#[pallet::compact] poll_id: T::PollIndex,
			// TODO: Perhaps it's better to receive a vec of Balances with poll_option index
			// mapping, and then convert it to Votes struct. Instead of receiving zeros.
			votes: Votes<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Call inner function.
			Self::try_vote(&who, poll_id, votes.clone())?;
			// Emit an event.
			Self::deposit_event(Event::<T>::Voted { voter: who.clone(), poll_id, votes });
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn remove_vote(
			origin: OriginFor<T>,
			#[pallet::compact] poll_id: T::PollIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// TODO: Get and check poll by poll_id
			// TODO: Remove vote

			// Emit an event.
			Self::deposit_event(Event::VoteRemoved { voter: who, poll_id });
			Ok(())
		}

		/// TODO: Write comment
		/// Check if account is: in benefitiaries / or is a voter (won or lost) / already
		/// - Beneficiary=true, Interest=1% of winning poll option amount
		/// - Voter=true, Interest=99% of his Vote amount
		/// Check if he is the voter
		/// - Check benefitiaries interest
		/// - If benefitiaries sum is 100% - voters should return with Err
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn collect(
			origin: OriginFor<T>,
			#[pallet::compact] poll_id: T::PollIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Get poll and check is it finished or cancelled.
			let poll = PollDetailsOf::<T>::get(poll_id).ok_or(Error::<T>::PollInvalid)?;
			if poll.status.is_ongoing() {
				return Err(Error::<T>::CollectOnOngoingPoll.into())
			}
			// Find out if origin is a beneficiary or voter.
			let bnf = poll.find_beneficiary(&who);
			let voter = VotingOf::<T>::get((&who, poll_id));
			if bnf.is_none() && voter.is_none() {
				return Err(Error::<T>::AccountNotVoterOrBeneficiary.into())
			}
			// Init needed variables.
			let win_opt = poll.winning_option();
			let interest_sum = poll.beneficiary_sum();
			let mut bnf_interest_amount = T::Balance::zero();
			let mut voter_return_amount = T::Balance::zero();
			// Check if win_opt is available.
			if let Some(win_option) = win_opt {
				// Check if origin is a beneficiary.
				if let Some(bnf) = bnf {
					// Check if origin has funds to collect.
					if !bnf.collected {
						bnf_interest_amount = poll.votes.0[win_option as usize]
							.saturating_mul(bnf.interest.into())
							.checked_div(&(100u32 * 100u32).into())
							.ok_or_else(|| ArithmeticError::Underflow)?;
					}
				}
			}
			// Check if origin is a voter.
			if let Some(voter) = voter {
				// Check if origin has funds to collect.
				if !voter.collected {
					// TODO: Add rewards collect logic here.
					for (i, bal) in voter.votes.0.iter().enumerate() {
						if win_opt.is_some() && i == win_opt.unwrap() as usize {
							let amount = bal
								.saturating_mul(interest_sum.into())
								.checked_div(&(100u32 * 100u32).into())
								.ok_or_else(|| ArithmeticError::Underflow)?;
							voter_return_amount = voter_return_amount.saturating_add(amount);
						} else {
							voter_return_amount = voter_return_amount.saturating_add(*bal);
						}
					}
				}
			}
			// Check is there anything that origin can collect.
			if bnf_interest_amount.is_zero() && voter_return_amount.is_zero() {
				return Err(Error::<T>::NothingToCollect.into())
			}
			let mut amount = T::Balance::zero();
			if bnf_interest_amount > Zero::zero() {
				// TODO: update_collected_beneficiary_in_poll_details();
				amount = amount.saturating_add(bnf_interest_amount);
			}
			if voter_return_amount > Zero::zero() {
				// TODO: update_collected_account_vote();
				amount = amount.saturating_add(voter_return_amount);
			}
			// TODO: send_money(account, currency, amount);
			// Emit an event.
			Self::deposit_event(Event::Collected { who, poll_id, amount });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// The account ID of the treasury pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	/// Return the amount of money in the pot.
	// The existential deposit is not part of the pot so account never gets deleted.
	pub fn pot() -> T::Balance {
		T::Currency::free_balance(&Self::account_id())
			// Must never be less than 0 but better be safe.
			.saturating_sub(T::Currency::minimum_balance())
			.into()
	}

	fn begin_block(_now: T::BlockNumber) -> Weight {
		let weight = Weight::zero();
		weight
	}

	/// Actually enact a vote, if legit.
	fn try_vote(
		who: &T::AccountId,
		poll_id: T::PollIndex,
		votes: Votes<T::Balance>,
	) -> DispatchResult {
		let mut poll = Self::poll_status(poll_id)?;
		ensure!(votes.validate(poll.options_count), Error::<T>::InvalidPollVotes);
		ensure!(
			votes.capital() <= T::Currency::free_balance(who).into(),
			Error::<T>::InsufficientFunds
		);
		VotingOf::<T>::try_mutate((who, poll_id), |voting| -> DispatchResult {
			if let Some(v) = voting {
				v.votes.add(&votes);
			} else {
				*voting = Some(AccountVotes { votes: votes.clone(), collected: false });
			}
			// Shouldn't be possible to fail, but we handle it gracefully.
			poll.votes.add(&votes).ok_or(ArithmeticError::Overflow)?;
			Ok(())
		})?;
		// TODO: Transfer or lock.
		// Extend the lock to `balance` (rather than setting it) since we don't know what other
		// votes are in place.
		// T::Currency::extend_lock(DEMOCRACY_ID, who, vote.balance(), WithdrawReasons::TRANSFER);
		PollDetailsOf::<T>::insert(poll_id, poll);
		Ok(())
	}

	/// Returns Ok(PollDetails) if the given poll.status is Ongoing, Error::PollInvalid otherwise.
	fn poll_status(
		poll_id: T::PollIndex,
	) -> Result<PollDetails<T::Balance, T::AccountId, T::AssetId, T::BlockNumber>, DispatchError> {
		let poll = PollDetailsOf::<T>::get(poll_id).ok_or(Error::<T>::PollInvalid)?;
		match poll.status.is_ongoing() {
			true => Ok(poll),
			_ => Err(Error::<T>::PollInvalid.into()),
		}
	}
}
