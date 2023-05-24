#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::marker::PhantomData;

use dashmap::DashMap;

mod types;
pub use types::{Error, Recorder, UseCase, UseCaseBytes};

mod recorder;
pub use recorder::{Stat, StatsRecorder};

mod utils;

type IntPointer = usize;

lazy_static::lazy_static! {
    static ref TRACKED_POINTERS: DashMap<IntPointer, UseCaseBytes> = DashMap::new();
}

thread_local! {
    static CURRENT_USECASE: RefCell<Option<UseCaseBytes>> = RefCell::new(None);
}


/// A drop-guard for setting and resetting the current usecase.
///
/// Returned by [Alloc::with_usecase].
pub struct Guard {
    old_value: Option<UseCaseBytes>,
    // Guard needs to be dropped in the same thread again in order to unset the usecase.
    _unsend: utils::PhantomUnsend,
    _unsync: utils::PhantomUnsync,
}

impl Drop for Guard {
    fn drop(&mut self) {
        CURRENT_USECASE
            .try_with(|current_value| {
                *current_value.borrow_mut() = self.old_value.take();
            })
            .ok();
    }
}

/// A wrapper around another allocator `A` that records memory usage statistics into `R`.
pub struct Alloc<U: UseCase, R: Recorder<U> = StatsRecorder<U>, A: GlobalAlloc = System> {
    alloc: A,
    recorder: R,
    #[doc(hidden)]
    inner: PhantomData<U>,
}

impl<U: UseCase> Alloc<U> {
    /// Instantiate memento while wrapping the system allocator, and [StatsRecorder] as recorder.
    ///
    /// ```ignore
    /// #[global_allocator]
    /// static ALLOCATOR: memento::Alloc<MyUseCase> = memento::Alloc::new();
    /// ```
    ///
    ///
    /// A shortcut for `Alloc::new_with(StatsRecorder::new(), System)`
    pub const fn new() -> Self {
        Alloc::new_with(StatsRecorder::new(), System)
    }
}

impl<R: Recorder<U>, U: UseCase, A: GlobalAlloc> Alloc<U, R, A> {
    /// Instantiate memento with custom memory allocator to wrap and a custom recorder.
    pub const fn new_with(recorder: R, alloc: A) -> Self {
        Alloc {
            alloc,
            recorder,
            inner: std::marker::PhantomData,
        }
    }

    /// Switch usecase for the current thread.
    ///
    /// For as long as the guard is alive, memory allocations are attributed to the given usecase.
    ///
    /// This function can fail to return a guard in case you are trying to switch usecases from
    /// within the allocator itself.
    pub fn with_usecase(&self, use_case: U) -> Option<Guard> {
        self.synchronized(None, |current_value| {
            let rv = Guard {
                old_value: current_value.take(),
                _unsend: PhantomData,
                _unsync: PhantomData,
            };
            *current_value = Some(use_case.into());
            Ok(rv)
        })
        .ok()
    }

    /// Call the given function with the given usecase.
    ///
    /// If synchronized is called from within itself (possibly indirectly through the global
    /// allocator), the function is not called in order to prevent deadlocks in other code. This
    /// can happen quite often when trying to allocate while recording an allocation.
    fn synchronized<R2>(
        &self,
        size: Option<usize>,
        f: impl FnOnce(&mut Option<UseCaseBytes>) -> Result<R2, Error>,
    ) -> Result<R2, Error> {
        CURRENT_USECASE
            .try_with(|value| {
                if let Ok(mut value) = value.try_borrow_mut() {
                    f(&mut *value)
                } else {
                    Err(Error::CurrentUsecaseContentionRefCell)
                }
            })
            .map_err(|_| Error::CurrentUsecaseContentionThreadLocal)
            .and_then(|x| x)
            .map_err(|e| {
                self.recorder.on_error(e, size);
                e
            })
    }

    fn handle_on_alloc(&self, ptr: usize, layout: Layout) {
        self.synchronized(Some(layout.size()), |use_case_bytes| {
            let use_case = use_case_bytes.and_then(|x| U::try_from(x).ok()).unwrap_or_default();
            if self.recorder.on_alloc(use_case, layout.size()) {
                TRACKED_POINTERS.insert(ptr, use_case_bytes.unwrap_or_else(|| U::default().into()));
            }
            Ok(())
        })
        .ok();
    }

    fn handle_on_dealloc(&self, ptr: usize, layout: Layout) {
        self.synchronized(Some(layout.size()), |_| {
            if let Some((_, use_case_bytes)) = TRACKED_POINTERS.remove(&ptr) {
                self.recorder
                    .on_dealloc(U::try_from(use_case_bytes).unwrap_or_default(), layout.size());
            }
            Ok(())
        })
        .ok();
    }

    /// Try to grab the current recorder such that statistics can be read and reset. Call the
    /// closure with the recorder if successful.
    ///
    /// This function can fail if there is too much contention on the allocator, or if it is called
    /// from within itself.
    #[must_use]
    pub fn with_recorder<R2>(&self, f: impl FnOnce(&R) -> Result<R2, Error>) -> Result<R2, Error> {
        self.synchronized(None, |_| f(&self.recorder))
    }
}

unsafe impl<R: Recorder<U>, U: UseCase, A: GlobalAlloc> GlobalAlloc for Alloc<U, R, A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.alloc.alloc(layout);
        self.handle_on_alloc(ptr as usize, layout);
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.handle_on_dealloc(ptr as usize, layout);
        self.alloc.dealloc(ptr, layout);
    }
}
