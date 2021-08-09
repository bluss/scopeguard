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
	#[inline]
	fn should_run() -> bool { std::thread::panicking() }
}

#[cfg(feature = "use_std")]
impl Strategy for OnSuccess {
	#[inline]
	fn should_run() -> bool { !std::thread::panicking() }
}
