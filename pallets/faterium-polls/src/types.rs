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

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct PollDetails<PollId, Balance, AccountId, AssetId, BlockNumber> {
	pub id: PollId,
	pub created_by: AccountId,
	pub ipfs_cid: IpfsCid,
	pub options_count: u8,
	pub currency: PollCurrency<AssetId>,
	pub beneficiaries: Vec<(AccountId, u32)>,
	pub reward_settings: RewardSettings,
	pub goal: Balance,
	pub tally: Tally<Balance>,
	pub status: PollStatus<BlockNumber>,
}

/// Status of a poll, present or past.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum PollStatus<BlockNumber> {
	/// Poll is happening, the arg is the block number at which it will end.
	Ongoing {
		/// When voting on this poll will begin.
		start: BlockNumber,
		/// When voting on this poll will end.
		end: BlockNumber,
	},
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
	/// The sum of all options.
	pub sum: Balance,
	/// The vector of sorted options' stake.
	pub options_votes: Vec<Balance>,
}

impl<Balance: AtLeast32BitUnsigned> Tally<Balance> {
	/// Add an account's vote into the tally. Returns None if invalid option or overflow.
	pub fn add(&mut self, vote: &AccountVote<Balance>) -> Option<usize> {
		self.sum = self.sum.checked_add(&vote.capital())?;
		for v in &vote.0 {
			if let Some(cap) = self.options_votes.get_mut(v.0 as usize) {
				cap.checked_add(&v.1)?;
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
