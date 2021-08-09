/// Macro to create a `ScopeGuard` (always run).
///
/// The macro takes statements, which are the body of a closure
/// that will run when the scope is exited.
#[macro_export]
macro_rules! defer {
	($($t:tt)*) => {
		let _guard = $crate::guard((), |()| { $($t)* });
	};
}

/// Macro to create a `ScopeGuard` (run on successful scope exit).
///
/// The macro takes statements, which are the body of a closure
/// that will run when the scope is exited.
///
/// Requires crate feature `use_std`.
#[cfg(feature = "use_std")]
#[macro_export]
macro_rules! defer_on_success {
	($($t:tt)*) => {
		let _guard = $crate::guard_on_success((), |()| { $($t)* });
	};
}

/// Macro to create a `ScopeGuard` (run on unwinding from panic).
///
/// The macro takes statements, which are the body of a closure
/// that will run when the scope is exited.
///
/// Requires crate feature `use_std`.
#[cfg(feature = "use_std")]
#[macro_export]
macro_rules! defer_on_unwind {
	($($t:tt)*) => {
		let _guard = $crate::guard_on_unwind((), |()| { $($t)* });
	};
}
