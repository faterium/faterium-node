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
	inherent::Vec,
	traits::{
		schedule::Named as ScheduleNamed,
		tokens::fungibles::{Balanced, Inspect, Transfer},
		Currency, ExistenceRequirement, Get, LockableCurrency, ReservableCurrency,
	},
	weights::Weight,
	PalletId,
};
use frame_system::Config as SystemConfig;
use scale_info::prelude::*;
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32BitUnsigned, CheckedDiv, Saturating, StaticLookup, Zero,
	},
	ArithmeticError, DispatchError, DispatchResult,
};

/// Balance type alias.
pub(crate) type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
/// Account id lookup type alias.
pub(crate) type AccountIdLookupOf<T> =
	<<T as frame_system::Config>::Lookup as StaticLookup>::Source;
/// Asset id type alias.
pub(crate) type AssetIdOf<T> =
	<<T as Config>::Fungibles as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;
/// Block number type alias.
pub(crate) type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[frame_support::pallet]
pub mod pallet {
	use super::{DispatchResult, *};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	// TODO: Remove without_storage_info macro. And somehow replace Vectors in storages.
	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The fungibles instance used for transfers in assets.
		/// The Balance type should be the same as in balances pallet.
		type Fungibles: Inspect<Self::AccountId, Balance = BalanceOf<Self>>
			+ Transfer<Self::AccountId>
			+ Balanced<Self::AccountId>;

		/// Currency type for this pallet.
		/// The Balance type should be the same as in assets pallet.
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
		PollDetails<BalanceOf<T>, T::AccountId, AssetIdOf<T>, BlockNumberOf<T>>,
	>;

	/// All votes for a particular voter.
	#[pallet::storage]
	#[pallet::getter(fn voting_of)]
	pub type VotingOf<T: Config> =
		StorageMap<_, Blake2_128Concat, (T::AccountId, T::PollIndex), AccountVotes<BalanceOf<T>>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A poll was created.
		Created { poll_id: T::PollIndex, creator: T::AccountId },
		/// A poll has been cancelled.
		Cancelled { poll_id: T::PollIndex },
		/// An account has voted in a poll.
		Voted { voter: T::AccountId, poll_id: T::PollIndex, votes: Votes<BalanceOf<T>> },
		/// An account has voted in a poll.
		VoteRemoved { voter: T::AccountId, poll_id: T::PollIndex },
		/// Voter/beneficiary collected his vote/interest.
		Collected { who: T::AccountId, poll_id: T::PollIndex, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Too high a balance was provided that the account cannot afford.
		InsufficientFunds,
		/// Invalid poll details given.
		InvalidPollDetails,
		/// Invalid poll start or end given.
		InvalidPollPeriod,
		/// Invalid poll currency given.
		InvalidPollCurrency,
		/// Vote given for an invalid poll.
		PollInvalid,
		/// Invalid number of votes given for a poll.
		InvalidPollVotes,
		/// The poll has not yet started.
		PollNotStarted,
		/// Can't collect from Ongoing Poll.
		CollectOnOngoingPoll,
		/// Account is neither a voter nor a beneficiary.
		AccountNotVoterOrBeneficiary,
		/// Nothing to collect or already collected
		NothingToCollect,
		/// The account currently has no votes attached to a poll.
		VotesNotExist,
		/// FATAL ERROR: The pot account cannot afford to transfer requested funds.
		PotInsufficientFunds,
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
		/// Create a poll.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// - `ipfs_cid`: The IPFS CID of the poll.
		/// - `beneficiaries`: Those who will get winning deposit, summary min=0, max=10_000.
		/// - `reward_settings`: Reward settings of the poll.
		/// - `goal`: The goal or minimum target amount on one option for the poll to happen.
		/// - `options_count`: The number of poll options.
		/// - `currency`: Currency of the poll.
		/// - `start`: When voting on this poll will begin.
		/// - `end`: When voting on this poll will end.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn create_poll(
			origin: OriginFor<T>,
			ipfs_cid: IpfsCid,
			beneficiaries: Vec<(AccountIdLookupOf<T>, u32)>,
			reward_settings: RewardSettings,
			goal: BalanceOf<T>,
			options_count: u8,
			currency: PollCurrency<AssetIdOf<T>>,
			start: BlockNumberOf<T>,
			end: BlockNumberOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Lookup for accounts of beneficiaries.
			let mut benfs = vec![];
			for b in beneficiaries {
				let account = T::Lookup::lookup(b.0)?;
				benfs.push(Beneficiary::new(account, b.1));
			}
			// Ensure start and end blocks are valid.
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(start >= now && end > now && end > start, Error::<T>::InvalidPollPeriod);
			// Ensure currency asset exists.
			if let PollCurrency::Asset(asset_id) = currency {
				let total_issuance =
					<T::Fungibles as Inspect<T::AccountId>>::total_issuance(asset_id);
				ensure!(total_issuance > BalanceOf::<T>::zero(), Error::<T>::InvalidPollPeriod);
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

		/// Cancel a poll in emergency.
		///
		/// Can't be called if poll already finished.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// - `poll_id`: The index of the poll to cancel.
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

		/// Vote in a poll.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// - `poll_id`: The index of the poll to vote for.
		/// - `votes`: The votes balances, should match number of options.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn vote(
			origin: OriginFor<T>,
			#[pallet::compact] poll_id: T::PollIndex,
			// TODO: Perhaps it's better to receive a vec of Balances with poll_option index
			// mapping, and then convert it to Votes struct. Instead of receiving zeros.
			votes: Votes<BalanceOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Call inner function.
			Self::try_vote(&who, poll_id, votes.clone())?;
			// Emit an event.
			Self::deposit_event(Event::<T>::Voted { voter: who.clone(), poll_id, votes });
			Ok(())
		}

		/// Remove vote from a poll.
		///
		/// Origin can remove only own vote. If this function called - all account Votes will be
		/// removed from a poll, and all staked balances will be returned to origin.
		///
		/// Can't be called after finish of a poll.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// - `poll_id`: The index of the poll to remove votes.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn remove_vote(
			origin: OriginFor<T>,
			#[pallet::compact] poll_id: T::PollIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Call inner function.
			Self::try_remove_vote(&who, poll_id)?;
			// Emit an event.
			Self::deposit_event(Event::VoteRemoved { voter: who, poll_id });
			Ok(())
		}

		/// Collect a vote stake or/and winning option from a poll.
		///
		/// This function will check if account is one of: in benefitiaries,
		/// or is a voter (poll cancelled or his vote on poll option won/lost).
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// - `poll_id`: The index of the poll to collect.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,2).ref_time())]
		pub fn collect(
			origin: OriginFor<T>,
			#[pallet::compact] poll_id: T::PollIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Call inner function.
			let amount = Self::try_collect(&who, poll_id)?;
			// Emit an event.
			Self::deposit_event(Event::Collected { who, poll_id, amount });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// The account ID of the faterium polls pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	/// Return the amount of money in the balances pot.
	/// The existential deposit is not part of the pot so account never gets deleted.
	pub fn balances_pot() -> BalanceOf<T> {
		T::Currency::free_balance(&Self::account_id())
			// Must never be less than 0 but better be safe.
			.saturating_sub(T::Currency::minimum_balance())
			.into()
	}

	/// Return the amount of money in the asset pot by asset_id.
	pub fn asset_pot(asset_id: AssetIdOf<T>) -> BalanceOf<T> {
		<T::Fungibles as Inspect<T::AccountId>>::balance(asset_id, &Self::account_id()).into()
	}

	/// Returns Ok(PollDetails) if the given poll.status is Ongoing, Error::PollInvalid otherwise.
	fn poll_status(
		poll_id: T::PollIndex,
	) -> Result<PollDetails<BalanceOf<T>, T::AccountId, AssetIdOf<T>, T::BlockNumber>, DispatchError>
	{
		let poll = PollDetailsOf::<T>::get(poll_id).ok_or(Error::<T>::PollInvalid)?;
		match poll.status.is_ongoing() {
			true => Ok(poll),
			_ => Err(Error::<T>::PollInvalid.into()),
		}
	}

	fn check_balance(
		who: &T::AccountId,
		currency: PollCurrency<AssetIdOf<T>>,
		cap: BalanceOf<T>,
	) -> bool {
		match currency {
			PollCurrency::Native => cap <= T::Currency::free_balance(who).into(),
			PollCurrency::Asset(asset_id) =>
				cap <= <T::Fungibles as Inspect<T::AccountId>>::balance(asset_id, who).into(),
		}
	}

	fn transfer_balance(
		source: &T::AccountId,
		dest: &T::AccountId,
		currency: PollCurrency<AssetIdOf<T>>,
		balance: BalanceOf<T>,
	) -> DispatchResult {
		match currency {
			PollCurrency::Native => {
				// TODO: Perhaps we want make some other function here to not pay fees.
				T::Currency::transfer(source, dest, balance, ExistenceRequirement::AllowDeath)?;
			},
			PollCurrency::Asset(asset_id) => {
				// TODO: Perhaps we want make something like `teleport` here to not pay fees.
				<T::Fungibles as Transfer<T::AccountId>>::transfer(
					asset_id, source, dest, balance, false,
				)?;
			},
		};
		Ok(())
	}

	fn begin_block(_now: T::BlockNumber) -> Weight {
		let weight = Weight::zero();
		weight
	}

	/// Actually enact a vote, if legit.
	fn try_vote(
		who: &T::AccountId,
		poll_id: T::PollIndex,
		votes: Votes<BalanceOf<T>>,
	) -> DispatchResult {
		let mut poll = Self::poll_status(poll_id)?;
		// Check if Votes has valid number of options.
		ensure!(votes.validate(poll.options_count), Error::<T>::InvalidPollVotes);
		// Ensure start and end blocks are valid.
		if let PollStatus::Ongoing { start, .. } = poll.status {
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(start <= now, Error::<T>::PollNotStarted);
		}
		// Check if origin has enough funds.
		ensure!(
			Self::check_balance(who, poll.currency, votes.capital()),
			Error::<T>::InsufficientFunds,
		);
		// Actually transfer balance to the pot.
		Self::transfer_balance(who, &Self::account_id(), poll.currency, votes.capital())?;
		// Set or increase Votes on the poll.
		VotingOf::<T>::try_mutate((who, poll_id), |voting| -> DispatchResult {
			if let Some(v) = voting {
				// Shouldn't be possible to fail, but we handle it gracefully.
				v.votes.add(&votes).ok_or(ArithmeticError::Overflow)?;
			} else {
				*voting = Some(AccountVotes { votes: votes.clone(), collected: false });
			}
			// Shouldn't be possible to fail, but we handle it gracefully.
			poll.votes.add(&votes).ok_or(ArithmeticError::Overflow)?;
			Ok(())
		})?;
		// Update poll in storage.
		PollDetailsOf::<T>::insert(poll_id, poll);
		Ok(())
	}

	/// Actually remove a vote from a poll, if legit.
	fn try_remove_vote(who: &T::AccountId, poll_id: T::PollIndex) -> DispatchResult {
		let poll = Self::poll_status(poll_id)?;
		// Get account votes.
		let voter = VotingOf::<T>::get((who, poll_id)).ok_or(Error::<T>::VotesNotExist)?;
		// Check if pot has enough funds.
		ensure!(
			Self::check_balance(who, poll.currency, voter.votes.capital()),
			Error::<T>::PotInsufficientFunds,
		);
		// Actually remove the vote.
		VotingOf::<T>::remove((who, poll_id));
		// Decrease Votes on the poll.
		PollDetailsOf::<T>::try_mutate(poll_id, |poll| -> DispatchResult {
			// Shouldn't be possible to fail, but we handle it gracefully.
			poll.as_mut()
				.ok_or(Error::<T>::PollInvalid)?
				.votes
				.remove(&voter.votes)
				.ok_or(ArithmeticError::Underflow)?;
			Ok(())
		})?;
		// Actually transfer balance from the pot to account.
		Self::transfer_balance(&Self::account_id(), who, poll.currency, voter.votes.capital())?;
		Ok(())
	}

	/// Actually collect a vote or winning option, if the account is legit.
	fn try_collect(
		who: &T::AccountId,
		poll_id: T::PollIndex,
	) -> Result<BalanceOf<T>, DispatchError> {
		// Get poll and check is it finished or cancelled.
		let mut poll = PollDetailsOf::<T>::get(poll_id).ok_or(Error::<T>::PollInvalid)?;
		if poll.status.is_ongoing() {
			return Err(Error::<T>::CollectOnOngoingPoll.into())
		}
		// Find out if origin is a beneficiary or voter.
		let bnf = poll.get_beneficiary(who);
		let voter = VotingOf::<T>::get((who, poll_id));
		if bnf.is_none() && voter.is_none() {
			return Err(Error::<T>::AccountNotVoterOrBeneficiary.into())
		}
		// Init needed variables.
		let currency = poll.currency.clone();
		let win_opt = poll.winning_option();
		let interest_sum = poll.beneficiary_sum();
		let mut bnf_interest_amount = BalanceOf::<T>::zero();
		let mut voter_return_amount = BalanceOf::<T>::zero();
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
		if let Some(voter) = &voter {
			// Check if origin has funds to collect.
			if !voter.collected {
				// FUTURE WORK TODO: Add rewards collect logic here.
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
		// Check if pot has enough funds.
		ensure!(
			Self::check_balance(
				who,
				poll.currency,
				bnf_interest_amount.saturating_add(voter_return_amount)
			),
			Error::<T>::PotInsufficientFunds,
		);
		let mut amount = BalanceOf::<T>::zero();
		if bnf_interest_amount > Zero::zero() {
			amount = amount.saturating_add(bnf_interest_amount);
			// Must never be an error, but better to be safe.
			let bnf = poll.get_mut_beneficiary(who).ok_or(Error::<T>::PollInvalid)?;
			bnf.collected = true;
			// Update poll in storage.
			PollDetailsOf::<T>::insert(poll_id, poll);
		}
		if voter_return_amount > Zero::zero() {
			amount = amount.saturating_add(voter_return_amount);
			// Must never be an error, but better to be safe.
			let mut votes = voter.ok_or(Error::<T>::PollInvalid)?;
			votes.collected = true;
			// Update poll vote in storage.
			VotingOf::<T>::insert((who, poll_id), votes);
		}
		// Actually transfer balance to the pot.
		Self::transfer_balance(&Self::account_id(), who, currency, amount)?;
		Ok(amount)
	}
}
