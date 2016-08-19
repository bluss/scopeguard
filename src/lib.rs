#![cfg_attr(not(test), no_std)]

#[cfg(not(test))]
extern crate core as std;

use std::ops::{Deref, DerefMut};

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
/// If you place a guard value in a local variable, its destructor will
/// run regardless how you leave the function — regular return or panic
/// (barring abnormal incidents like aborts; so as long as destructors run).
///
/// The guard's closure will be called with a mut ref to the held value
/// in the destructor. It's called only once.
///
/// The `Guard` implements `Deref` so that you can access the inner value.
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

// F might be a Fn and therefore maybe not implement Sync,
// but a &FnMut is useless, and F is inaccessible anyway.
unsafe impl<T, F> Sync for Guard<T, F>
    where T: Sync, F: Send+FnMut(&mut T)
{}

#[test]
fn test_defer() {
    use std::cell::Cell;

    let drops = Cell::new(0);
    defer!(drops.set(1000));
    assert_eq!(drops.get(), 0);
}

