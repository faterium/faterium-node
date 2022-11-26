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

#[test]
fn vote_with_balances_should_work() {
	new_test_ext().execute_with(|| {
		let _pid = begin_poll();
	});
}

// #[test]
// fn vote_with_balances_should_work() {
// 	new_test_ext().execute_with(|| {
// 		let _pid = begin_poll();
// 	});
// }

#[test]
fn vote_cancellation_should_work() {
	new_test_ext().execute_with(|| {
		let pid = begin_poll();
		let v = AccountVote(vec![(1, 10u64)]);
		assert_ok!(FateriumPolls::vote(Origin::signed(5), pid, v));
		assert_ok!(FateriumPolls::remove_vote(Origin::signed(5), pid));
		assert_eq!(tally(pid), Tally::default());
		// assert_ok!(FateriumPolls::unlock(Origin::signed(5), 5));
		assert_eq!(Balances::locks(5), vec![]);
	});
}
