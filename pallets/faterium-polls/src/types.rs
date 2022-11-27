//! Various basic types for use in the assets pallet.

use super::*;
use frame_support::pallet_prelude::*;

pub type DepositBalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

pub type IpfsCid = Vec<u8>;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum PollCurrency<AssetId> {
	/// AssetId from the Assets Pallet.
	Asset(AssetId),
	/// Native Balances currency of the network.
	Native,
}

/// Enumeration for the poll reward settings.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum RewardSettings {
	/// No rewards for participators/winners in the poll.
	None,
}

/// Details of a poll.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct PollDetails<Balance, AccountId, AssetId, BlockNumber> {
	/// Account who created this poll.
	pub created_by: AccountId,
	/// IPFS CID with all contextual information regarding this poll.
	pub ipfs_cid: IpfsCid,
	/// Beneficiaries of this poll, who will get winning deposit.
	///
	/// Vector of [Account, Interest], where sum of all percentages never more
	/// than 100%, or 10_000u32 (e.g. 0.05% == 5; 10% == 1000).
	///
	/// If empty, all stakes can be returned to the voters after the end of the poll.
	pub beneficiaries: Vec<(AccountId, u32)>,
	/// Reward settings of the poll.
	pub reward_settings: RewardSettings,
	/// The goal or minimum target amount on one option for the poll to happen.
	pub goal: Balance,
	/// The number of poll options.
	pub options_count: u8,
	/// Info regrading stake on poll options.
	pub tally: Tally<Balance>,
	/// Currency of the poll.
	pub currency: PollCurrency<AssetId>,
	/// Status of the poll.
	pub status: PollStatus<BlockNumber>,
}

impl<Balance: AtLeast32BitUnsigned, AccountId, AssetId, BlockNumber>
	PollDetails<Balance, AccountId, AssetId, BlockNumber>
{
	/// Creates a new PollDetails with Ongoing status and empty Tally.
	pub fn new(
		created_by: AccountId,
		ipfs_cid: IpfsCid,
		beneficiaries: Vec<(AccountId, u32)>,
		reward_settings: RewardSettings,
		goal: Balance,
		options_count: u8,
		currency: PollCurrency<AssetId>,
		start: BlockNumber,
		end: BlockNumber,
	) -> Self {
		Self {
			created_by,
			ipfs_cid,
			beneficiaries,
			reward_settings,
			goal,
			options_count,
			tally: Tally::new(options_count),
			currency,
			status: PollStatus::Ongoing { start, end },
		}
	}

	/// Returns true if struct valid, false otherwise.
	pub fn validate(&self) -> bool {
		// TODO: Validate struct
		true
	}
}

/// Status of a poll, present, cancelled, or past.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum PollStatus<BlockNumber> {
	/// Poll is happening, the args are the block number at which it will start and end.
	Ongoing {
		/// When voting on this poll will begin.
		start: BlockNumber,
		/// When voting on this poll will end.
		end: BlockNumber,
	},
	/// Poll has been cancelled at a given block.
	Cancelled(BlockNumber),
	/// Poll finished at `end`, and has `winning_option`.
	Finished {
		/// What poll option has won.
		winning_option: u8,
		/// When voting on this poll ended.
		end: BlockNumber,
	},
}

/// A vote for a poll of a particular account.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct AccountVote<Balance>(pub Vec<(u8, Balance)>);

impl<Balance: AtLeast32BitUnsigned> AccountVote<Balance> {
	pub fn capital(&self) -> Balance {
		self.0
			.iter()
			.map(|x| x.1.clone())
			.fold(Balance::zero(), |a, b| a.saturating_add(b))
	}
}

/// Info regarding an ongoing poll.
#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct Tally<Balance> {
	/// The sum of all options' stake.
	pub sum: Balance,
	/// The vector of options' stake.
	pub options_votes: Vec<Balance>,
}

impl<Balance: AtLeast32BitUnsigned> Tally<Balance> {
	pub fn new(options_count: u8) -> Self {
		Self {
			sum: Balance::zero(),
			options_votes: (0..options_count).map(|_| Balance::zero()).collect(),
		}
	}

	/// Add an account's vote into the tally. Returns None if invalid option or overflow.
	pub fn add(&mut self, vote: &AccountVote<Balance>) -> Option<usize> {
		self.sum = self.sum.checked_add(&vote.capital())?;
		for v in &vote.0 {
			if let Some(cap) = self.options_votes.get_mut(v.0 as usize) {
				*cap = cap.checked_add(&v.1)?;
			} else {
				return None
			}
		}
		Some(vote.0.len())
	}

	/// Remove an account's vote from the tally. Returns None if invalid option or overflow.
	pub fn remove(&mut self, vote: &AccountVote<Balance>) -> Option<usize> {
		self.sum = self.sum.checked_add(&vote.capital())?;
		for v in &vote.0 {
			if let Some(cap) = self.options_votes.get_mut(v.0 as usize) {
				cap.checked_sub(&v.1)?;
			} else {
				return None
			}
		}
		Some(vote.0.len())
	}
}
