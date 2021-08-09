use super::*;
use std::cell::Cell;
use std::panic::catch_unwind;
use std::panic::AssertUnwindSafe;

#[test]
fn test_defer() {
	let drops = Cell::new(0);
	defer!(drops.set(1000));
	assert_eq!(drops.get(), 0);
}

#[cfg(feature = "use_std")]
#[test]
fn test_defer_success_1() {
	let drops = Cell::new(0);
	{
		defer_on_success!(drops.set(1));
		assert_eq!(drops.get(), 0);
	}
	assert_eq!(drops.get(), 1);
}

#[cfg(feature = "use_std")]
#[test]
fn test_defer_success_2() {
	let drops = Cell::new(0);
	let _ = catch_unwind(AssertUnwindSafe(|| {
		defer_on_success!(drops.set(1));
		panic!("failure")
	}));
	assert_eq!(drops.get(), 0);
}

#[cfg(feature = "use_std")]
#[test]
fn test_defer_unwind_1() {
	let drops = Cell::new(0);
	let _ = catch_unwind(AssertUnwindSafe(|| {
		defer_on_unwind!(drops.set(1));
		assert_eq!(drops.get(), 0);
		panic!("failure")
	}));
	assert_eq!(drops.get(), 1);
}

#[cfg(feature = "use_std")]
#[test]
fn test_defer_unwind_2() {
	let drops = Cell::new(0);
	{
		defer_on_unwind!(drops.set(1));
	}
	assert_eq!(drops.get(), 0);
}

#[test]
fn test_only_dropped_by_closure_when_run() {
	let value_drops = Cell::new(0);
	let value = guard((), |()| value_drops.set(1 + value_drops.get()));
	let closure_drops = Cell::new(0);
	let guard = guard(value, |_| closure_drops.set(1 + closure_drops.get()));
	assert_eq!(value_drops.get(), 0);
	assert_eq!(closure_drops.get(), 0);
	drop(guard);
	assert_eq!(value_drops.get(), 1);
	assert_eq!(closure_drops.get(), 1);
}

#[cfg(feature = "use_std")]
#[test]
fn test_dropped_once_when_not_run() {
	let value_drops = Cell::new(0);
	let value = guard((), |()| value_drops.set(1 + value_drops.get()));
	let captured_drops = Cell::new(0);
	let captured = guard((), |()| captured_drops.set(1 + captured_drops.get()));
	let closure_drops = Cell::new(0);
	let guard = guard_on_unwind(value, |value| {
		drop(value);
		drop(captured);
		closure_drops.set(1 + closure_drops.get())
	});
	assert_eq!(value_drops.get(), 0);
	assert_eq!(captured_drops.get(), 0);
	assert_eq!(closure_drops.get(), 0);
	drop(guard);
	assert_eq!(value_drops.get(), 1);
	assert_eq!(captured_drops.get(), 1);
	assert_eq!(closure_drops.get(), 0);
}

#[test]
fn test_into_inner() {
	let dropped = Cell::new(false);
	let value = guard(42, |_| dropped.set(true));
	let guard = guard(value, |_| dropped.set(true));
	let inner = ScopeGuard::into_inner(guard);
	assert_eq!(dropped.get(), false);
	assert_eq!(*inner, 42);
}
