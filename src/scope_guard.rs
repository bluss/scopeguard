#[cfg(not(any(test, feature = "use_std")))]
extern crate core as std;

use std::fmt;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::ptr;

use crate::{Always, OnSuccess, OnUnwind, Strategy};

/// `ScopeGuard` is a scope guard that may own a protected value.
///
/// If you place a guard in a local variable, the closure can run regardless how
/// you leave the scope — through regular return or panic (except if panic or
/// other code aborts; so as long as destructors run). It is run only once.
///
/// The `S` parameter for [`Strategy`](trait.Strategy.html) determines if the
/// closure actually runs.
///
/// The guard's closure will be called with the held value in the destructor.
///
/// The `ScopeGuard` implements `Deref` so that you can access the inner value.
pub struct ScopeGuard<T, F, S = Always>
	where F: FnOnce(T),
		S: Strategy,
{
	value: ManuallyDrop<T>,
	dropfn: ManuallyDrop<F>,
	// fn(S) -> S is used, so that the S is not taken into account for auto traits.
	strategy: PhantomData<fn(S) -> S>,
}

impl<T, F, S> ScopeGuard<T, F, S>
	where F: FnOnce(T),
		S: Strategy,
{
	/// Create a `ScopeGuard` that owns `v` (accessible through deref) and calls
	/// `dropfn` when its destructor runs.
	///
	/// The `Strategy` decides whether the scope guard's closure should run.
	#[inline]
	pub fn with_strategy(v: T, dropfn: F) -> ScopeGuard<T, F, S> {
		ScopeGuard {
			value: ManuallyDrop::new(v),
			dropfn: ManuallyDrop::new(dropfn),
			strategy: PhantomData,
		}
	}

	/// "Defuse" the guard and extract the value without calling the closure.
	///
	/// ```
	/// extern crate scopeguard;
	///
	/// use scopeguard::{guard, ScopeGuard};
	///
	/// fn conditional() -> bool { true }
	///
	/// fn main() {
	///     let mut guard = guard(Vec::new(), |mut v| v.clear());
	///     guard.push(1);
	///
	///     if conditional() {
	///         // a condition maybe makes us decide to
	///         // “defuse” the guard and get back its inner parts
	///         let value = ScopeGuard::into_inner(guard);
	///     } else {
	///         // guard still exists in this branch
	///     }
	/// }
	/// ```
	#[inline]
	pub fn into_inner(guard: Self) -> T {
		// Cannot move out of `Drop`-implementing types,
		// so `ptr::read` the value and forget the guard.
		let mut guard = ManuallyDrop::new(guard);
		unsafe {
			let value = ptr::read(&*guard.value);
			// Drop the closure after `value` has been read, so that if the
			// closure's `drop` function panics, unwinding still tries to drop
			// `value`.
			ManuallyDrop::drop(&mut guard.dropfn);
			value
		}
	}
}

/// Create a new `ScopeGuard` owning `v` and with deferred closure `dropfn`.
#[inline]
pub fn guard<T, F>(v: T, dropfn: F) -> ScopeGuard<T, F, Always>
	where F: FnOnce(T)
{
	ScopeGuard::with_strategy(v, dropfn)
}

/// Create a new `ScopeGuard` owning `v` and with deferred closure `dropfn`.
///
/// Requires crate feature `use_std`.
#[cfg(feature = "use_std")]
#[inline]
pub fn guard_on_success<T, F>(v: T, dropfn: F) -> ScopeGuard<T, F, OnSuccess>
	where F: FnOnce(T)
{
	ScopeGuard::with_strategy(v, dropfn)
}

/// Create a new `ScopeGuard` owning `v` and with deferred closure `dropfn`.
///
/// Requires crate feature `use_std`.
///
/// ## Examples
///
/// For performance reasons, or to emulate “only run guard on unwind” in
/// no-std environments, we can also use the default guard and simply manually
/// defuse it at the end of scope like the following example. (The performance
/// reason would be if the [`OnUnwind`]'s call to [std::thread::panicking()] is
/// an issue.)
///
/// ```
/// extern crate scopeguard;
///
/// use scopeguard::ScopeGuard;
/// # fn main() {
/// {
///     let guard = scopeguard::guard((), |_| {});
///
///     // rest of the code here
///
///     // we reached the end of scope without unwinding - defuse it
///     ScopeGuard::into_inner(guard);
/// }
/// # }
/// ```
#[cfg(feature = "use_std")]
#[inline]
pub fn guard_on_unwind<T, F>(v: T, dropfn: F) -> ScopeGuard<T, F, OnUnwind>
	where F: FnOnce(T)
{
	ScopeGuard::with_strategy(v, dropfn)
}

// ScopeGuard can be Sync even if F isn't because the closure is
// not accessible from references.
// The guard does not store any instance of S, so it is also irrelevant.
unsafe impl<T, F, S> Sync for ScopeGuard<T, F, S>
	where T: Sync,
		F: FnOnce(T),
		S: Strategy
{}

impl<T, F, S> Deref for ScopeGuard<T, F, S>
	where F: FnOnce(T),
		S: Strategy
{
	type Target = T;

	fn deref(&self) -> &T {
		&*self.value
	}
}

impl<T, F, S> DerefMut for ScopeGuard<T, F, S>
	where F: FnOnce(T),
		S: Strategy
{
	fn deref_mut(&mut self) -> &mut T {
		&mut *self.value
	}
}

impl<T, F, S> Drop for ScopeGuard<T, F, S>
	where F: FnOnce(T),
		S: Strategy
{
	fn drop(&mut self) {
		// This is OK because the fields are `ManuallyDrop`s
		// which will not be dropped by the compiler.
		let (value, dropfn) = unsafe {
			(ptr::read(&*self.value), ptr::read(&*self.dropfn))
		};
		if S::should_run() {
			dropfn(value);
		}
	}
}

impl<T, F, S> fmt::Debug for ScopeGuard<T, F, S>
	where T: fmt::Debug,
		F: FnOnce(T),
		S: Strategy
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct(stringify!(ScopeGuard))
			.field("value", &*self.value)
			.finish()
	}
}
