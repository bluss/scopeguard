#![cfg_attr(not(any(test, feature = "use_std")), no_std)]
#![cfg_attr(feature="cargo-clippy", allow(inline_always))]

//! A scope guard will run a given closure when it goes out of scope,
//! even if the code between panics.
//! (as long as panic doesn't abort)
//!
//! # Examples
//!
//! ## `defer!`
//!
//! Use the `defer` macro to run an operation at scope exit,
//! either regular scope exit or during unwinding from a panic.
//!
//! ```
//! #[macro_use(defer)] extern crate scopeguard;
//!
//! use std::cell::Cell;
//!
//! fn main() {
//!     // use a cell to observe drops during and after the scope guard is active
//!     let drop_counter = Cell::new(0);
//!     {
//!         // Create a scope guard using `defer!` for the current scope
//!         defer! {{
//!             drop_counter.set(1 + drop_counter.get());
//!         }};
//!
//!         // Do regular operations here in the meantime.
//!
//!         // Just before scope exit: it hasn't run yet.
//!         assert_eq!(drop_counter.get(), 0);
//!
//!         // The following scope end is where the defer closure is called
//!     }
//!     assert_eq!(drop_counter.get(), 1);
//! }
//! ```
//!
//! ## Scope Guard with Value
//!
//! If the scope guard closure needs to access an outer value that is also
//! mutated outside of the scope guard, then you may want to use the scope guard
//! with a value. The guard works like a smart pointer, so the inner value can
//! be accessed by reference or by mutable reference.
//!
//! ### 1. The guard owns a file
//!
//! In this example, the scope guard owns a file and ensures pending writes are
//! synced at scope exit.
//!
//! ```
//! extern crate scopeguard;
//! 
//! use std::fs::File;
//! use std::io::{self, Write};
//! 
//! fn try_main() -> io::Result<()> {
//!     let f = File::create("newfile.txt")?;
//!     let mut file = scopeguard::guard(f, |f| {
//!         // ensure we flush file at return or panic
//!         let _ = f.sync_all();
//!     });
//!     // Access the file through the scope guard itself
//!     file.write_all(b"test me\n").map(|_| ())
//! }
//!
//! fn main() {
//!     try_main().unwrap();
//! }
//!
//! ```
//!
//! ### 2. The guard restores an invariant on scope exit
//!
//! ```
//! extern crate scopeguard;
//!
//! use std::mem::ManuallyDrop;
//! use std::ptr;
//!
//! // This function, just for this example, takes the first element
//! // and inserts it into the assumed sorted tail of the vector.
//! //
//! // For optimization purposes we temporarily violate an invariant of the
//! // Vec, that it owns all of its elements.
//! // 
//! // The safe approach is to use swap, which means two writes to memory,
//! // the optimization is to use a “hole” which uses only one write of memory
//! // for each position it moves.
//! //
//! // We *must* use a scope guard to run this code safely. We
//! // are running arbitrary user code (comparison operators) that may panic.
//! // The scope guard ensures we restore the invariant after successful
//! // exit or during unwinding from panic.
//! fn insertion_sort_first<T>(v: &mut Vec<T>)
//!     where T: PartialOrd
//! {
//!     struct Hole<'a, T: 'a> {
//!         v: &'a mut Vec<T>,
//!         index: usize,
//!         value: ManuallyDrop<T>,
//!     }
//!
//!     unsafe {
//!         // Create a moved-from location in the vector, a “hole”.
//!         let value = ptr::read(&v[0]);
//!         let mut hole = Hole { v: v, index: 0, value: ManuallyDrop::new(value) };
//!
//!         // Use a scope guard with a value.
//!         // At scope exit, plug the hole so that the vector is fully
//!         // initialized again.
//!         // The scope guard owns the hole, but we can access it through the guard.
//!         let mut hole_guard = scopeguard::guard(hole, |hole| {
//!             // plug the hole in the vector with the value that was // taken out
//!             let index = hole.index;
//!             ptr::copy_nonoverlapping(&*hole.value, &mut hole.v[index], 1);
//!         });
//!
//!         // run algorithm that moves the hole in the vector here
//!         // move the hole until it's in a sorted position
//!         for i in 1..hole_guard.v.len() {
//!             if *hole_guard.value >= hole_guard.v[i] {
//!                 // move the element back and the hole forward
//!                 let index = hole_guard.index;
//!                 ptr::copy_nonoverlapping(&hole_guard.v[index + 1], &mut hole_guard.v[index], 1);
//!                 hole_guard.index += 1;
//!             } else {
//!                 break;
//!             }
//!         }
//!
//!         // When the scope exits here, the Vec becomes whole again!
//!     }
//! }
//!
//! fn main() {
//!     let string = String::from;
//!     let mut data = vec![string("c"), string("a"), string("b"), string("d")];
//!     insertion_sort_first(&mut data);
//!     assert_eq!(data, vec!["a", "b", "c", "d"]);
//! }
//!
//! ```
//!
//!
//! # Crate features:
//!
//! - `use_std`
//!   + Enabled by default. Enables the `OnUnwind` and `OnSuccess` strategies.
//!   + Disable to use `no_std`.

#[cfg(not(any(test, feature = "use_std")))]
extern crate core as std;

use std::fmt;
use std::marker::PhantomData;
use std::mem::{self, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::ptr;

/// Controls in which cases the associated code should be run
pub trait Strategy {
    /// Return `true` if the guard’s associated code should run
    /// (in the context where this method is called).
    fn should_run() -> bool;
}

/// Always run on scope exit.
///
/// “Always” run: on regular exit from a scope or on unwinding from a panic.
/// Can not run on abort, process exit, and other catastrophic events where
/// destructors don’t run.
#[derive(Debug)]
pub enum Always {}

/// Run on scope exit through unwinding.
///
/// Requires crate feature `use_std`.
#[cfg(feature = "use_std")]
#[derive(Debug)]
pub enum OnUnwind {}

/// Run on regular scope exit, when not unwinding.
///
/// Requires crate feature `use_std`.
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

/// Macro to create a `ScopeGuard` (always run).
///
/// The macro takes one expression `$e`, which is the body of a closure
/// that will run when the scope is exited. The expression can
/// be a whole block.
#[macro_export]
macro_rules! defer {
    ($e:expr) => {
        let _guard = $crate::guard((), |()| $e);
    }
}

/// Macro to create a `ScopeGuard` (run on successful scope exit).
///
/// The macro takes one expression `$e`, which is the body of a closure
/// that will run when the scope is exited. The expression can
/// be a whole block.
///
/// Requires crate feature `use_std`.
#[cfg(feature = "use_std")]
#[macro_export]
macro_rules! defer_on_success {
    ($e:expr) => {
        let _guard = $crate::guard_on_success((), |()| $e);
    }
}

/// Macro to create a `ScopeGuard` (run on unwinding from panic).
///
/// The macro takes one expression `$e`, which is the body of a closure
/// that will run when the scope is exited. The expression can
/// be a whole block.
///
/// Requires crate feature `use_std`.
#[cfg(feature = "use_std")]
#[macro_export]
macro_rules! defer_on_unwind {
    ($e:expr) => {
        let _guard = $crate::guard_on_unwind((), |()| $e);
    }
}

/// `ScopeGuard` is a scope guard that may own a protected value.
///
/// If you place a guard in a local variable, the closure can
/// run regardless how you leave the scope — through regular return or panic
/// (except if panic or other code aborts; so as long as destructors run).
/// It is run only once.
///
/// The `S` parameter for [`Strategy`](Strategy.t.html) determines if
/// the closure actually runs.
///
/// The guard's closure will be called with the held value in the destructor.
///
/// The `ScopeGuard` implements `Deref` so that you can access the inner value.
pub struct ScopeGuard<T, F: FnOnce(T), S: Strategy = Always> {
    value: ManuallyDrop<T>,
    dropfn: ManuallyDrop<F>,
    strategy: PhantomData<S>,
}
impl<T, F: FnOnce(T), S: Strategy> ScopeGuard<T, F, S> {
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

    // Extract the value and closure. (without calling it)
    #[inline]
    pub fn into_inner(self) -> (T,F) {
        // Cannot pattern match out of Drop-implementing types, so
        // ptr::read the types to return and forget the source.
        unsafe {
            let value = ptr::read(&*self.value);
            let dropfn = ptr::read(&*self.dropfn);
            mem::forget(self);
            (value, dropfn)
        }
    }
}


/// Create a new `ScopeGuard` owning `v` and with deferred closure `dropfn`.
#[inline]
pub fn guard<T, F: FnOnce(T)>(v: T, dropfn: F) -> ScopeGuard<T, F, Always> {
    ScopeGuard::with_strategy(v, dropfn)
}

/// Create a new `ScopeGuard` owning `v` and with deferred closure `dropfn`.
///
/// Requires crate feature `use_std`.
#[cfg(feature = "use_std")]
#[inline]
pub fn guard_on_success<T, F: FnOnce(T)>(v: T, dropfn: F)
-> ScopeGuard<T, F, OnSuccess> {
    ScopeGuard::with_strategy(v, dropfn)
}

/// Create a new `ScopeGuard` owning `v` and with deferred closure `dropfn`.
///
/// Requires crate feature `use_std`.
#[cfg(feature = "use_std")]
#[inline]
pub fn guard_on_unwind<T, F: FnOnce(T)>(v: T, dropfn: F)
-> ScopeGuard<T, F, OnUnwind> {
    ScopeGuard::with_strategy(v, dropfn)
}

// ScopeGuard can be Sync even if F isn't because the closure is
// not accessible from references.
// The guard does not store any instance of S, so it is also irellevant.
unsafe impl<T: Sync, F: FnOnce(T), S: Strategy> Sync for ScopeGuard<T, F, S> {}

impl<T, F: FnOnce(T), S: Strategy> Deref for ScopeGuard<T, F, S> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.value
    }
}

impl<T, F: FnOnce(T), S: Strategy> DerefMut for ScopeGuard<T, F, S> {
    fn deref_mut(&mut self) -> &mut T {
        &mut*self.value
    }
}

impl<T, F: FnOnce(T), S: Strategy> Drop for ScopeGuard<T, F, S> {
    fn drop(&mut self) {
        // This is OK because the fields are `ManuallyDrop`s
        // which will not be dropped by the compiler.
        let (value,dropfn) = unsafe {
            (ptr::read(&*self.value), ptr::read(&*self.dropfn))
        };
        if S::should_run() {
            dropfn(value);
        }
    }
}

impl<T: fmt::Debug, F: FnOnce(T), S: Strategy>
fmt::Debug for ScopeGuard<T, F, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ScopeGuard")
         .field("value", &*self.value)
         .finish()
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_only_dropped_by_closure_when_run() {
        let value_drops = Cell::new(0);
        let value = guard((), |()| value_drops.set(1+value_drops.get()));
        let closure_drops = Cell::new(0);
        let guard = guard(value, |_| closure_drops.set(1+closure_drops.get()));
        assert_eq!(value_drops.get(), 0);
        assert_eq!(closure_drops.get(), 0);
        drop(guard);
        assert_eq!(value_drops.get(), 1);
        assert_eq!(closure_drops.get(), 1);
    }

    #[test]
    fn test_dropped_once_when_not_run() {
        let value_drops = Cell::new(0);
        let value = guard((), |()| value_drops.set(1+value_drops.get()));
        let captured_drops = Cell::new(0);
        let captured = guard((), |()| captured_drops.set(1+captured_drops.get()));
        let closure_drops = Cell::new(0);
        let guard = guard_on_unwind(value, |value| {
            drop(value);
            drop(captured);
            closure_drops.set(1+closure_drops.get())
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
        let value = guard((), |_| dropped.set(true));
        let guard = guard(value, |_| dropped.set(true));
        let (_value, _closure) = guard.into_inner();
        assert_eq!(dropped.get(), false);
    }
}
