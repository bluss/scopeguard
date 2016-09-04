//! A scope guard will run a given closure when it goes out of scope,
//! even if the code between panics.
//! (as long as panic doesn't abort)

#![cfg_attr(not(test), no_std)]

#[cfg(not(test))]
extern crate core as std;

use std::ops::{Deref, DerefMut};
use std::mem::forget;

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

/// `Guard` is a scope guard that may own a protected value.
///
/// If you place a guard in a local variable, the closure will
/// run regardless how you leave the scope — through regular return or panic
/// (except if panic or other code aborts; so as long as destructors run).
/// It is run only once.
///
/// The guard's closure will be called with a mut ref to the held value;
/// While the closure could just capture it, by placing the value in the guard
/// the rest of the function can access it too through the `Deref` and `DerefMut` impl.
pub struct Guard<T, F>
    where F: FnMut(&mut T)
{
    __dropfn: F,
    __value: T,
}

/// Create a new `Guard` owning `v` and with deferred closure `dropfn`.
pub fn guard<T, F>(v: T, dropfn: F) -> Guard<T, F>
    where F: FnMut(&mut T)
{
    Guard{__value: v, __dropfn: dropfn}
}

/// Execute `cleanup` only if `might_panic` panics and unwind.
pub fn handle_panic_of<R, F, D>(might_panic: F,  mut cleanup: D) -> R
    where F: FnOnce()->R, D: FnMut()
{
    let guard = guard((), |_| cleanup() );
    let r = might_panic();
    forget(guard);
    r
}

impl<T, F> Deref for Guard<T, F>
    where F: FnMut(&mut T)
{
    type Target = T;
    fn deref(&self) -> &T
    {
        &self.__value
    }

}

impl<T, F> DerefMut for Guard<T, F>
    where F: FnMut(&mut T)
{
    fn deref_mut(&mut self) -> &mut T
    {
        &mut self.__value
    }
}

impl<T, F> Drop for Guard<T, F>
    where F: FnMut(&mut T)
{
    fn drop(&mut self) {
        (self.__dropfn)(&mut self.__value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn test_defer() {
        let drops = Cell::new(0);
        defer!(drops.set(1000));
        assert_eq!(drops.get(), 0);
    }

    #[test]
    #[should_panic(expected="might_panic paniced")]
    fn test_handle_panic() {
        handle_panic_of(|| (), || panic!("cleanup executed when it should'nt") );

        let panicing = Cell::new(false);
        defer!(assert!(panicing.get()));
        handle_panic_of(|| panic!("might_panic paniced"), || panicing.set(true) );
    }
}
