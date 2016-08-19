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
    where F: FnOnce(T)
{
    __at_drop: Option<(T, F)>
}

/// Create a new `Guard` owning `v` and with deferred closure `dropfn`.
pub fn guard<T, F>(v: T, dropfn: F) -> Guard<T, F>
    where F: FnOnce(T)
{
    Guard{__at_drop: Some((v, dropfn))}
}

impl<T, F> Deref for Guard<T, F>
    where F: FnOnce(T)
{
    type Target = T;
    fn deref(&self) -> &T {
        &self.__at_drop.as_ref().unwrap().0
    }
}

impl<T, F> DerefMut for Guard<T, F>
    where F: FnOnce(T)
{
    fn deref_mut(&mut self) -> &mut T {
        &mut self.__at_drop.as_mut().unwrap().0
    }
}

impl<T, F> Drop for Guard<T, F>
    where F: FnOnce(T)
{
    fn drop(&mut self) {
        if let Some((value, dropfn)) = self.__at_drop.take() {
            dropfn(value);
        }
    }
}

// F might be a Fn and therefore maybe not implement Sync,
// but a &FnOnce is useless, and F is inaccessible anyway.
unsafe impl<T, F> Sync for Guard<T, F>
    where T: Sync, F: Send+FnOnce(T)
{}

#[test]
fn test_defer() {
    use std::cell::Cell;

    let drops = Cell::new(0);
    defer!(drops.set(1000));
    assert_eq!(drops.get(), 0);
}

