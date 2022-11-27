//! The tests for normal voting functionality.

use super::*;

#[test]
fn overvoting_should_fail() {
	new_test_ext().execute_with(|| {
		let pid = begin_poll();
		let v = AccountVote(vec![(2, 10u64)]);
		assert_noop!(
			FateriumPolls::vote(Origin::signed(1), pid, v),
			Error::<Test>::InsufficientFunds
		);
	});
}

/// # Successful standard poll
///
/// 1. Author creates poll (pays fee)
/// 2. Voters vote in the poll (pays fee, locks balance)
/// 3. End of the poll (zero commision)
/// 4. Author take balance from winning poll option (pays fee)
/// 5. Voters that lost take own balance from poll (pays fee)
#[test]
fn vote_with_balances_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(11), 0);
		let pid = begin_poll_with_balances(11);
		let v = AccountVote(vec![(2, 10u64)]);
		assert_ok!(FateriumPolls::vote(Origin::signed(11), pid, v));
	});
}

#[test]
fn vote_with_balan_should_work() {
	new_test_ext().execute_with(|| {});
}

#[test]
fn vote_cancellation_should_work() {
	new_test_ext().execute_with(|| {
		let pid = begin_poll_with_balances(5);
		let v = AccountVote(vec![(1, 10u64)]);
		assert_ok!(FateriumPolls::vote(Origin::signed(5), pid, v));
		assert_ok!(FateriumPolls::remove_vote(Origin::signed(5), pid));
		assert_ne!(tally(pid), Tally::default());
		assert_eq!(tally(pid), Tally { sum: 10, options_votes: vec![0, 10, 0] });
		// assert_ok!(FateriumPolls::unlock(Origin::signed(5), 5));
		assert_eq!(Balances::locks(5), vec![]);
	});
}
