use super::*;
use crate::{AccountVote, Error, PollCurrency, RewardSettings};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(FateriumPolls::create_poll(
			Origin::signed(1),
			vec![],
			3,
			PollCurrency::Native,
			vec![],
			RewardSettings::None,
			100,
			10,
			20
		));
		// Read pallet storage and assert an expected result.
		assert_eq!(FateriumPolls::poll_count(), 1);
		// Ensure the expected error is thrown when no value is present.
		let vote = AccountVote::Standard { option: 2, balance: 10 };
		assert_noop!(FateriumPolls::vote(Origin::signed(1), 0, vote), Error::<Test>::NoneValue);
	});
}
