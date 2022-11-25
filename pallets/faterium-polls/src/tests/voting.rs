use super::*;
use frame_support::assert_noop;

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		begin_poll();
		// Read pallet storage and assert an expected result.
		assert_eq!(FateriumPolls::poll_count(), 1);
		// Ensure the expected error is thrown when no value is present.
		let vote = AccountVote::Standard { option: 2, balance: 10 };
		assert_noop!(FateriumPolls::vote(Origin::signed(1), 0, vote), Error::<Test>::NoneValue);
	});
}

#[test]
fn params_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(FateriumPolls::poll_count(), 0);
		assert_eq!(Balances::free_balance(0), 0);
		assert_eq!(Balances::total_issuance(), 0);
	});
}

#[test]
fn create_vote_close() {
	new_test_ext().execute_with(|| {
		begin_poll();

		// TODO: Test this function
	});
}
