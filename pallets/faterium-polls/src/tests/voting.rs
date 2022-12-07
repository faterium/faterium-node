//! The tests for normal voting functionality.

use super::*;

#[test]
fn vote_without_balance_should_fail() {
	new_test_ext().execute_with(|| {
		// Creates poll
		let pid = begin_poll(1, vec![], 10);
		// Try to vote without balance
		let v = Votes(vec![0, 10, 0]);
		assert_noop!(
			FateriumPolls::vote(Origin::signed(1), pid, v),
			Error::<Test>::InsufficientFunds
		);
	});
}

#[test]
fn vote_with_balances_should_work() {
	new_test_ext().execute_with(|| {
		let voter = 2;
		// Creates poll
		set_balances(voter);
		let pid = begin_poll(1, vec![], 10);
		// Vote on poll
		let v = Votes(vec![0, 10, 0]);
		assert_ok!(FateriumPolls::vote(Origin::signed(voter), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter), 10);
		assert_eq!(votes(pid), Votes(vec![0, 10, 0]));
		next_block();
		// Remove vote
		assert_ok!(FateriumPolls::remove_vote(Origin::signed(voter), pid));
		assert_eq!(votes(pid), Votes(vec![0, 0, 0]));
		assert_eq!(Balances::free_balance(voter), 20);
		next_block();
		// Vote again
		assert_ok!(FateriumPolls::vote(Origin::signed(voter), pid, v));
		// Finish poll
		fast_forward_to(10);
		assert_eq!(FateriumPolls::poll_count(), 1);
		// Can't remove vote after finish
		assert_noop!(
			FateriumPolls::remove_vote(Origin::signed(voter), pid),
			Error::<Test>::PollAlreadyFinished,
		);
		// Check if winning option is correct.
		let poll = FateriumPolls::poll_details_of(pid).unwrap();
		if let PollStatus::Finished { winning_option, end } = poll.status {
			assert_eq!(winning_option, 1);
			assert_eq!(end, 10);
		} else {
			panic!("poll not finished");
		}
		// Try collect as voter and beneficiary.
		assert_ok!(FateriumPolls::collect(Origin::signed(voter), pid));
	});
}

#[test]
fn emergency_cancel_should_work() {
	new_test_ext().execute_with(|| {
		let voter = 5;
		// Creates poll
		set_balances(voter);
		let pid = begin_poll(1, vec![], 10);
		// Vote on poll
		let v = Votes(vec![0, 10, 0]);
		assert_ok!(FateriumPolls::vote(Origin::signed(voter), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter), 10);
		assert_eq!(votes(pid), Votes(vec![0, 10, 0]));
		next_block();
		// Cancel poll
		assert_ok!(FateriumPolls::emergency_cancel(Origin::signed(1), pid));
		let poll = FateriumPolls::poll_details_of(pid).unwrap();
		// Check status
		assert_eq!(poll.status, PollStatus::Cancelled(3));
		assert_noop!(
			FateriumPolls::vote(Origin::signed(voter), pid, v),
			Error::<Test>::PollAlreadyFinished,
		);
		// Collect as voter
		assert_ok!(FateriumPolls::collect(Origin::signed(voter), pid));
		assert_eq!(Balances::free_balance(voter), 20);
	});
}

#[test]
fn multiple_voters_zero_interest() {
	new_test_ext().execute_with(|| {
		let initial_balance = 100;
		let voter_1 = 2;
		let voter_2 = 3;
		let voter_3 = 4;
		// Creates poll
		set_balances(5);
		let pid = begin_poll(1, vec![], 10);
		// Vote on poll #1
		let v = Votes(vec![0, 10, 0]);
		assert_ok!(Balances::set_balance(Origin::root(), voter_1, initial_balance, 0));
		assert_ok!(FateriumPolls::vote(Origin::signed(voter_1), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter_1), 90);
		// Vote on poll #2
		let v = Votes(vec![10, 0, 50]);
		assert_ok!(Balances::set_balance(Origin::root(), voter_2, initial_balance, 0));
		assert_ok!(FateriumPolls::vote(Origin::signed(voter_2), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter_2), 40);
		// Vote on poll #3
		let v = Votes(vec![10, 10, 40]);
		assert_ok!(Balances::set_balance(Origin::root(), voter_3, initial_balance, 0));
		assert_ok!(FateriumPolls::vote(Origin::signed(voter_3), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter_3), 40);
		// Check that all actual deposit in the poll
		assert_eq!(FateriumPolls::balances_pot(), 130);
		fast_forward_to(10);
		// Collect as voter #1
		assert_ok!(FateriumPolls::collect(Origin::signed(voter_1), pid));
		assert_eq!(Balances::free_balance(voter_1), initial_balance);
		assert_eq!(FateriumPolls::balances_pot(), 120);
		// Collect as voter #2
		assert_ok!(FateriumPolls::collect(Origin::signed(voter_2), pid));
		assert_eq!(Balances::free_balance(voter_2), initial_balance);
		assert_eq!(FateriumPolls::balances_pot(), 60);
		// Collect as voter #3
		assert_ok!(FateriumPolls::collect(Origin::signed(voter_3), pid));
		assert_eq!(Balances::free_balance(voter_3), initial_balance);
		assert_eq!(FateriumPolls::balances_pot(), 0);
	});
}

#[test]
fn multi_voters_multi_bnfs_full_interest() {
	new_test_ext().execute_with(|| {
		let initial_balance = 100;
		// Creates poll
		set_balances(5);
		let bnf_1 = 11;
		let bnf_2 = 12;
		let pid = begin_poll(1, vec![(bnf_1, 5000), (bnf_2, 5000)], 10);
		// Vote on poll #1
		let voter_1 = 3;
		let v = Votes(vec![0, 0, 70]);
		assert_ok!(Balances::set_balance(Origin::root(), voter_1, initial_balance, 0));
		assert_ok!(FateriumPolls::vote(Origin::signed(voter_1), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter_1), 30);
		// Vote on poll #2
		let voter_2 = 4;
		let v = Votes(vec![40, 30, 20]);
		assert_ok!(Balances::set_balance(Origin::root(), voter_2, initial_balance, 0));
		assert_ok!(FateriumPolls::vote(Origin::signed(voter_2), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter_2), 10);
		// Check that all actual deposit in the poll
		assert_eq!(FateriumPolls::balances_pot(), 160);
		fast_forward_to(10);
		// Collect as voter #1 - should loose 70
		assert_noop!(
			FateriumPolls::collect(Origin::signed(voter_1), pid),
			Error::<Test>::NothingToCollect,
		);
		assert_eq!(Balances::free_balance(voter_1), 30);
		// Collect as voter #2 - should loose 20
		assert_ok!(FateriumPolls::collect(Origin::signed(voter_2), pid));
		assert_eq!(Balances::free_balance(voter_2), 80);
		// Collect as beneficiary #1 - should get 45
		assert_ok!(FateriumPolls::collect(Origin::signed(bnf_1), pid));
		assert_eq!(Balances::free_balance(bnf_1), 45);
		// Collect as beneficiary #2 - should get 45
		assert_ok!(FateriumPolls::collect(Origin::signed(bnf_2), pid));
		assert_eq!(Balances::free_balance(bnf_2), 45);
	});
}

#[test]
fn multi_voters_multi_bnfs_partial_interest() {
	new_test_ext().execute_with(|| {
		let initial_balance = 100;
		// Creates poll
		set_balances(5);
		let bnf_1 = 11;
		let bnf_2 = 12;
		let pid = begin_poll(1, vec![(bnf_1, 2500), (bnf_2, 3500)], 10);
		assert_eq!(Balances::free_balance(bnf_1), 0);
		assert_eq!(Balances::free_balance(bnf_2), 0);
		// Vote on poll #1
		let voter_1 = 3;
		let v = Votes(vec![0, 0, 70]);
		assert_ok!(Balances::set_balance(Origin::root(), voter_1, initial_balance, 0));
		assert_ok!(FateriumPolls::vote(Origin::signed(voter_1), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter_1), 30);
		// Vote on poll #2
		let voter_2 = 4;
		let v = Votes(vec![40, 30, 20]);
		assert_ok!(Balances::set_balance(Origin::root(), voter_2, initial_balance, 0));
		assert_ok!(FateriumPolls::vote(Origin::signed(voter_2), pid, v.clone()));
		assert_eq!(Balances::free_balance(voter_2), 10);
		// Check that all actual deposit in the poll
		assert_eq!(FateriumPolls::balances_pot(), 160);
		fast_forward_to(10);
		// Collect as voter #1 - should loose only 60% from 70
		assert_ok!(FateriumPolls::collect(Origin::signed(voter_1), pid));
		assert_eq!(Balances::free_balance(voter_1), 58);
		// Collect as voter #2 - should loose only 60% from 20
		assert_ok!(FateriumPolls::collect(Origin::signed(voter_2), pid));
		assert_eq!(Balances::free_balance(voter_2), 88);
		// Collect as beneficiary #1 - should get 25% from 90
		assert_ok!(FateriumPolls::collect(Origin::signed(bnf_1), pid));
		assert_eq!(Balances::free_balance(bnf_1), 22 /* 22.5 */);
		// Collect as beneficiary #2 - should get 35% from 90
		assert_ok!(FateriumPolls::collect(Origin::signed(bnf_2), pid));
		assert_eq!(Balances::free_balance(bnf_2), 31 /* 31.5 */);
	});
}

#[test]
fn not_reaching_goal_should_fail() {
	new_test_ext().execute_with(|| {
		let voter = 5;
		let bnf = 11;
		// Creates poll
		set_balances(voter);
		let pid = begin_poll(1, vec![(bnf, 2500)], 100);
		// Vote once
		let v = Votes(vec![0, 0, 20]);
		assert_ok!(FateriumPolls::vote(Origin::signed(voter), pid, v.clone()));
		// Finish poll
		fast_forward_to(10);
		// Check if winning option is correct.
		let poll = FateriumPolls::poll_details_of(pid).unwrap();
		assert_eq!(poll.status, PollStatus::Failed(10));
		// Try collect as beneficiary
		assert_noop!(
			FateriumPolls::collect(Origin::signed(bnf), pid),
			Error::<Test>::NothingToCollect,
		);
		// Collect as voter
		assert_ok!(FateriumPolls::collect(Origin::signed(voter), pid));
	});
}

#[test]
fn vote_with_assets_should_work() {
	new_test_ext().execute_with(|| {
		let poll_creator = 1;
		let voter = 2;
		let voter_balance = 20;
		// Creates poll
		set_balances(voter);
		let (pid, asset_id) = begin_poll_with_asset(poll_creator, voter, vec![], voter_balance);
		// Vote on poll
		let v = Votes(vec![0, 10, 0]);
		assert_ok!(FateriumPolls::vote(Origin::signed(voter), pid, v.clone()));
		assert_eq!(Assets::balance(asset_id, voter), voter_balance - 10);
		assert_eq!(votes(pid), Votes(vec![0, 10, 0]));
		next_block();
		// Remove vote
		assert_ok!(FateriumPolls::remove_vote(Origin::signed(voter), pid));
		assert_eq!(votes(pid), Votes(vec![0, 0, 0]));
		assert_eq!(Assets::balance(asset_id, voter), voter_balance);
		next_block();
		// Vote again
		assert_ok!(FateriumPolls::vote(Origin::signed(voter), pid, v));
		// Finish poll
		fast_forward_to(10);
		assert_eq!(FateriumPolls::poll_count(), 1);
		// Can't remove vote after finish
		assert_noop!(
			FateriumPolls::remove_vote(Origin::signed(voter), pid),
			Error::<Test>::PollAlreadyFinished,
		);
		// Check if winning option is correct.
		let poll = FateriumPolls::poll_details_of(pid).unwrap();
		if let PollStatus::Finished { winning_option, end } = poll.status {
			assert_eq!(winning_option, 1);
			assert_eq!(end, 10);
		} else {
			panic!("poll not finished");
		}
		// Try collect as voter and beneficiary.
		assert_ok!(FateriumPolls::collect(Origin::signed(voter), pid));
	});
}
