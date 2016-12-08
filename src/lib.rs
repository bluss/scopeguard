//! A scope guard will run a given closure when it goes out of scope,
//! even if the code between panics.
//! (as long as panic doesn't abort)

#![cfg_attr(not(any(test, feature = "use_std")), no_std)]

#[cfg(not(any(test, feature = "use_std")))]
extern crate core as std;

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub trait Strategy {
    fn should_run() -> bool;
}

/// Always run on scope exit.
///
/// Always run; with the exception of abort, process exit, and other
/// catastrophic events.
#[derive(Debug)]
pub enum Always {}

/// Run on scope exit through unwinding.
#[cfg(feature = "use_std")]
#[derive(Debug)]
pub enum OnUnwind {}

/// Run on regular scope exit, when not unwinding.
#[cfg(feature = "use_std")]
#[derive(Debug)]
pub enum OnSuccess {}

impl Strategy for Always {
    #[inline(always)]
    fn should_run() -> bool { true }
}

#[cfg(feature = "use_std")]
impl Strategy for OnUnwind {
    #[inline(always)]
    fn should_run() -> bool { std::thread::panicking() }
}

#[cfg(feature = "use_std")]
impl Strategy for OnSuccess {
    #[inline(always)]
    fn should_run() -> bool { !std::thread::panicking() }
}

/// Macro to create a `Guard` (without any owned value).
///
/// The macro takes one expression `$e`, which is the body of a closure
/// that will run when the scope is exited. The expression can
/// be a whole block.
#[macro_export]
macro_rules! defer {
    ($e:expr) => {
        let _guard = $crate::guard((), |_| $e);
    }
}

/// Macro to create a `Guard` (without any owned value).
///
/// The macro takes one expression `$e`, which is the body of a closure
/// that will run when the scope is exited. The expression can
/// be a whole block.
#[macro_export]
macro_rules! defer_on_success {
    ($e:expr) => {
        let _guard = $crate::guard_on_success((), |_| $e);
    }
}

/// Macro to create a `Guard` (without any owned value).
///
/// The macro takes one expression `$e`, which is the body of a closure
/// that will run when the scope is exited. The expression can
/// be a whole block.
#[macro_export]
macro_rules! defer_on_unwind {
    ($e:expr) => {
        let _guard = $crate::guard_on_unwind((), |_| $e);
    }
}

/// `Guard` is a scope guard that may own a protected value.
///
/// If you place a guard in a local variable, the closure will
/// run regardless how you leave the scope — through regular return or panic
/// (except if panic or other code aborts; so as long as destructors run).
/// It is run only once.
///
/// The `Guard` implements `Deref` so that you can access the inner value.
pub struct Guard<T, F, S: Strategy = Always>
    where F: FnMut(&mut T)
{
    __dropfn: F,
    __value: T,
    strategy: PhantomData<S>,
}

/// Create a new `Guard` owning `v` and with deferred closure `dropfn`.
pub fn guard<T, F>(v: T, dropfn: F) -> Guard<T, F, Always>
    where F: FnMut(&mut T)
{
    guard_strategy(v, dropfn)
}

#[cfg(feature = "use_std")]
/// Create a new `Guard` owning `v` and with deferred closure `dropfn`.
pub fn guard_on_success<T, F>(v: T, dropfn: F) -> Guard<T, F, OnSuccess>
    where F: FnMut(&mut T)
{
    guard_strategy(v, dropfn)
}

#[cfg(feature = "use_std")]
/// Create a new `Guard` owning `v` and with deferred closure `dropfn`.
pub fn guard_on_unwind<T, F>(v: T, dropfn: F) -> Guard<T, F, OnUnwind>
    where F: FnMut(&mut T)
{
    guard_strategy(v, dropfn)
}

fn guard_strategy<T, F, S: Strategy>(v: T, dropfn: F) -> Guard<T, F, S>
    where F: FnMut(&mut T)
{
    Guard {
        __value: v,
        __dropfn: dropfn,
        strategy: PhantomData,
    }
}

impl<T, F, S: Strategy> Deref for Guard<T, F, S>
    where F: FnMut(&mut T)
{
    type Target = T;
    fn deref(&self) -> &T {
        &self.__value
    }

}

impl<T, F, S: Strategy> DerefMut for Guard<T, F, S>
    where F: FnMut(&mut T)
{
    fn deref_mut(&mut self) -> &mut T {
        &mut self.__value
    }
}

impl<T, F, S: Strategy> Drop for Guard<T, F, S>
    where F: FnMut(&mut T)
{
    fn drop(&mut self) {
        if S::should_run() {
            (self.__dropfn)(&mut self.__value)
        }
    }
}

impl<T, F, S> fmt::Debug for Guard<T, F, S>
    where T: fmt::Debug,
          F: FnMut(&mut T),
          S: Strategy + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Guard")
         .field("value", &self.__value)
         .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::panic::catch_unwind;
    use std::panic::AssertUnwindSafe;

    #[test]
    fn test_defer() {
        let drops = Cell::new(0);
        defer!(drops.set(1000));
        assert_eq!(drops.get(), 0);
    }

    #[test]
    fn test_defer_success_1() {
        let drops = Cell::new(0);
        {
            defer_on_success!(drops.set(1));
            assert_eq!(drops.get(), 0);
        }
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn test_defer_success_2() {
        let drops = Cell::new(0);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            defer_on_success!(drops.set(1));
            panic!("failure")
        }));
        assert_eq!(drops.get(), 0);
    }

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

    #[test]
    fn test_defer_unwind_2() {
        let drops = Cell::new(0);
        {
            defer_on_unwind!(drops.set(1));
        }
        assert_eq!(drops.get(), 0);
    }
}
