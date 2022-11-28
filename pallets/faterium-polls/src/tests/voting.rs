//! The tests for normal voting functionality.

use super::*;

#[test]
fn overvoting_should_fail() {
	new_test_ext().execute_with(|| {
		let pid = begin_poll();
		let v = Votes(vec![0, 10, 0]);
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
		let pid = begin_poll_with_balances(5);
		let v = Votes(vec![0, 10, 0]);
		assert_ok!(FateriumPolls::vote(Origin::signed(5), pid, v));
		assert_eq!(Balances::free_balance(5), 10);
		assert_eq!(votes(pid), Votes(vec![0, 10, 0]));
		assert_ok!(FateriumPolls::remove_vote(Origin::signed(5), pid));
		assert_eq!(votes(pid), Votes(vec![0, 0, 0]));
		assert_eq!(Balances::free_balance(5), 20);
	});
}

#[test]
fn vote_cancellation_should_work() {
	new_test_ext().execute_with(|| {});
}
