use std::hash::Hash;

/// The internal representation memento uses to represent instances of `UseCase`.
pub type UseCaseBytes = u32;

/// A `UseCase` is a struct describing what the application is currently doing. Memory statistics
/// are recorded per distinct value of `UseCase`.
///
/// Usually this trait is implemented by a flat, C-style enum.
///
/// For example, if an application has two processing stages called `Download` and `Process`, one
/// would define:
///
/// ```
/// use num_enum::{TryFromPrimitive, IntoPrimitive};
///
/// use memento::UseCase;
///
/// #[derive(TryFromPrimitive, IntoPrimitive, Default)]
/// #[repr(u32)]
/// pub enum ApplicationStage {
///     #[default]
///     Unknown,
///     Download,
///     Process,
/// }
///
/// impl UseCase for ApplicationStage {}
/// ```
pub trait UseCase:
    Default
    + TryFrom<UseCaseBytes>
    + Into<UseCaseBytes>
    + 'static
{
}

/// A recorder is a structure collecting statistics about memory usage. You might also call it a
/// "metrics sink".
///
/// Everytime there is an allocation or deallocation, methods here get called.
///
/// This trait is unsafe to impl because it is called from within `GlobalAlloc`, which is also
/// unsafe. All methods, at minimum, must not panic or unwind the stack. See the standard library
/// documentation on custom allocators for more information.
pub unsafe trait Recorder<U: UseCase> {
    /// Record an allocation of size `size` for a given usecase.
    ///
    /// This function is allowed to allocate further data, but must not panic/unwind.
    fn on_alloc(&self, _use_case: U, _size: usize) -> bool {
        false
    }

    /// Record freed memory of size `size` for a given usecase.
    ///
    /// This function is allowed to allocate further data, but must not panic/unwind.
    fn on_dealloc(&self, _use_case: U, _size: usize) {}

    /// Record an error encountered by memento that caused it to drop stats, such as a detected
    /// deadlock that caused it to drop metrics.
    ///
    /// Deadlocks happen all the time when recording metrics.
    ///
    /// This function must not allocate or panic/unwind.
    fn on_error(&self, _code: Error, _size: Option<usize>) {}
}

/// an error encountered by memento that caused it to drop stats, such as a detected deadlock that
/// caused it to drop metrics.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Error {
    /// This error happens potentially when memento allocates internally.
    CurrentUsecaseContentionRefCell,

    /// This error happens potentially when memento allocates internally.
    CurrentUsecaseContentionThreadLocal,

    /// A `UseCase` was converted to `UseCaseBytes`, and later failed to parse back into `UseCase`.
    ///
    /// Most likely your `TryFrom<UseCaseBytes>` and `Into<UseCaseBytes>` implementations don't
    /// match, and are not isomorphic.
    CurrentUsecaseBadBytes,
}
