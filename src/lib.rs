#![cfg_attr(not(any(test, feature = "use_std")), no_std)]
#![doc(html_root_url = "https://docs.rs/scopeguard/1/")]

//! A scope guard will run a given closure when it goes out of scope,
//! even if the code between panics.
//! (as long as panic doesn't abort)
//!
//! # Examples
//!
//! ## Hello World
//!
//! This example creates a scope guard with an example function:
//!
//! ```
//! extern crate scopeguard;
//!
//! fn f() {
//!     let _guard = scopeguard::guard((), |_| {
//!         println!("Hello Scope Exit!");
//!     });
//!
//!     // rest of the code here.
//!
//!     // Here, at the end of `_guard`'s scope, the guard's closure is called.
//!     // It is also called if we exit this scope through unwinding instead.
//! }
//! # fn main() {
//! #    f();
//! # }
//! ```
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
//!         defer! {
//!             drop_counter.set(1 + drop_counter.get());
//!         }
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
//! use std::fs::*;
//! use std::io::{self, Write};
//! # // Mock file so that we don't actually write a file
//! # struct MockFile;
//! # impl MockFile {
//! #     fn create(_s: &str) -> io::Result<Self> { Ok(MockFile) }
//! #     fn write_all(&self, _b: &[u8]) -> io::Result<()> { Ok(()) }
//! #     fn sync_all(&self) -> io::Result<()> { Ok(()) }
//! # }
//! # use self::MockFile as File;
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
//! # Crate Features
//!
//! - `use_std`
//!   + Enabled by default. Enables the `OnUnwind` and `OnSuccess` strategies.
//!   + Disable to use `no_std`.
//!
//! # Rust Version
//!
//! This version of the crate requires Rust 1.20 or later.
//!
//! The scopeguard 1.x release series will use a carefully considered version
//! upgrade policy, where in a later 1.x version, we will raise the minimum
//! required Rust version.

mod macros;

mod strategy;
pub use strategy::{Always, OnSuccess, OnUnwind, Strategy};

mod scope_guard;
pub use scope_guard::{ScopeGuard, guard, guard_on_success, guard_on_unwind};

#[cfg(test)]
mod tests;
