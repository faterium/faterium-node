//! Various basic types for use in the assets pallet.

use super::*;
use frame_support::pallet_prelude::*;
// traits::{fungible, tokens::BalanceConversion},
// use sp_runtime::{traits::Convert, FixedPointNumber, FixedPointOperand, FixedU128};

pub(super) type DepositBalanceOf<T> =
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
	pub(super) id: PollId,
	pub(super) created_by: AccountId,
	pub(super) ipfs_cid: IpfsCid,
	pub(super) options_count: u8,
	pub(super) currency: PollCurrency<AssetId>,
	pub(super) beneficiaries: Vec<(AccountId, u32)>,
	pub(super) reward_settings: RewardSettings,
	pub(super) goal: Balance,
	pub(super) poll_start: BlockNumber,
	pub(super) poll_end: BlockNumber,
}

/// A vote for a referendum of a particular account.
#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum AccountVote<Balance> {
	/// A standard vote, one-way (approve or reject) with a given amount of conviction.
	Standard { option: u8, balance: Balance },
}
