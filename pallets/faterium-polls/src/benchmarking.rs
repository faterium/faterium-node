//! Benchmarking setup for pallet-faterium-polls

use super::*;

#[allow(unused)]
use crate::Pallet as FateriumPolls;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	create_poll {
		let s in 0 .. 10;
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), Vec::new(), Vec::new(), RewardSettings::None, 100u32.into(), s as u8, true, PollCurrency::Native, 10u32.into(), 20u32.into())
	verify {
		assert_eq!(PollCount::<T>::get(), s.into());
	}

	impl_benchmark_test_suite!(FateriumPolls, crate::mock::new_test_ext(), crate::mock::Test);
}
