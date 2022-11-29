//! Various basic types for use in the assets pallet.

use super::*;
use frame_support::pallet_prelude::*;

pub type DepositBalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

pub type IpfsCid = Vec<u8>;

#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
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
	/// Vector of [Account, Interest, IsCollected], where sum of all percentages never
	/// more than 100%, or 10_000u32 (e.g. 5 = 0.05%; 1000 = 10%).
	///
	/// If empty, all stakes can be returned to the voters after the end of the poll.
	pub beneficiaries: Vec<Beneficiary<AccountId>>,
	/// Reward settings of the poll.
	pub reward_settings: RewardSettings,
	/// The goal or minimum target amount on one option for the poll to happen.
	pub goal: Balance,
	/// The number of poll options.
	pub options_count: u8,
	/// Info regrading stake on poll options.
	pub votes: Votes<Balance>,
	/// Currency of the poll.
	pub currency: PollCurrency<AssetId>,
	/// Status of the poll.
	pub status: PollStatus<BlockNumber>,
}

impl<Balance: AtLeast32BitUnsigned + Copy, AccountId: Clone + Eq, AssetId, BlockNumber>
	PollDetails<Balance, AccountId, AssetId, BlockNumber>
{
	/// Creates a new PollDetails with Ongoing status and empty Tally.
	pub fn new(
		created_by: AccountId,
		ipfs_cid: IpfsCid,
		beneficiaries: Vec<Beneficiary<AccountId>>,
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
			votes: Votes::new(options_count),
			currency,
			status: PollStatus::Ongoing { start, end },
		}
	}

	/// Returns true if struct valid, false otherwise.
	pub fn validate(&self) -> bool {
		// IPFS CID v0 is 46 bytes; IPFS CID v1 is 59 bytes.
		let len = self.ipfs_cid.len();
		if len != 46 && len != 59 {
			return false
		}
		if self.options_count > 10 {
			return false
		}
		if self.beneficiaries.len() > 0 {
			let sum = self.beneficiary_sum();
			if sum > 10_000u32 {
				return false
			}
			if sum == 0u32 {
				return false
			}
		}
		true
	}

	/// Finds and returns beneficiary by account id.
	pub fn get_beneficiary(&self, account: &AccountId) -> Option<Beneficiary<AccountId>> {
		self.beneficiaries.iter().find(|&x| x.who.eq(account)).cloned()
	}

	/// Finds and returns mutable beneficiary by account id.
	pub fn get_mut_beneficiary(
		&mut self,
		account: &AccountId,
	) -> Option<&mut Beneficiary<AccountId>> {
		self.beneficiaries.iter_mut().find(|x| x.who.eq(account))
	}

	pub fn beneficiary_sum(&self) -> u32 {
		self.beneficiaries.iter().fold(0u32, |a, b| a.saturating_add(b.interest))
	}

	pub fn winning_option(&self) -> Option<u8> {
		match self.status {
			PollStatus::Finished { winning_option, .. } => Some(winning_option),
			_ => None,
		}
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct Beneficiary<AccountId> {
	/// AccountId of the beneficiary.
	pub who: AccountId,
	/// Beneficiary interest, can't be more than 10_000u32.
	/// Can be converted to percentage (e.g. 5 = 0.05%; 1000 = 10%).
	pub interest: u32,
	/// Is beneficiary collected winning option from the poll.
	pub collected: bool,
}

impl<AccountId> Beneficiary<AccountId> {
	pub fn new(who: AccountId, interest: u32) -> Self {
		Self { who, interest, collected: false }
	}
}

/// Status of a poll, present, cancelled, or past.
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
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

impl<BlockNumber> PollStatus<BlockNumber> {
	pub fn is_ongoing(&self) -> bool {
		match self {
			PollStatus::Ongoing { .. } => true,
			_ => false,
		}
	}
}

/// A vote for a poll of a particular account.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct AccountVotes<Balance> {
	pub votes: Votes<Balance>,
	pub collected: bool,
}

/// A vote for a poll.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct Votes<Balance>(pub Vec<Balance>);

impl<Balance: AtLeast32BitUnsigned + Copy> Votes<Balance> {
	pub fn new(options_count: u8) -> Self {
		Self((0..options_count).map(|_| Balance::zero()).collect())
	}

	pub fn validate(&self, options_count: u8) -> bool {
		self.0.len() == options_count as usize
	}

	pub fn capital(&self) -> Balance {
		self.0.iter().fold(Balance::zero(), |a, b| a.saturating_add(*b))
	}

	/// Add an account's vote into the tally. Returns None if invalid option or overflow.
	pub fn add(&mut self, votes: &Votes<Balance>) -> Option<()> {
		if votes.0.len() != self.0.len() {
			return None
		}
		for (i, b) in votes.0.iter().enumerate() {
			self.0[i] = self.0[i].checked_add(&b)?;
		}
		Some(())
	}

	/// Remove an account's vote from the tally. Returns None if invalid option or overflow.
	pub fn remove(&mut self, votes: &Votes<Balance>) -> Option<()> {
		if votes.0.len() != self.0.len() {
			return None
		}
		for (i, b) in votes.0.iter().enumerate() {
			self.0[i] = self.0[i].checked_sub(&b)?;
		}
		Some(())
	}
}
