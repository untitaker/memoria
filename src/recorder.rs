use std::fmt;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{Error, Recorder, UseCase, UseCaseBytes};

use dashmap::DashMap;
use once_cell::sync::OnceCell;

/// A simple recorder for memory statistics that can be flushed periodically.
pub struct StatsRecorder<U: UseCase> {
    current_usecase_contention_ref_cell: AtomicUsize,
    current_usecase_contention_thread_local: AtomicUsize,
    current_usecase_bad_bytes: AtomicUsize,
    // we store UseCaseBytes so UseCase does not need to require Hash
    results: OnceCell<DashMap<UseCaseBytes, Stat>>,
    _phantom: PhantomData<U>,
}

impl<U: UseCase> StatsRecorder<U> {
    /// Construct a new recorder.
    pub const fn new() -> Self {
        StatsRecorder {
            current_usecase_contention_ref_cell: AtomicUsize::new(0),
            current_usecase_contention_thread_local: AtomicUsize::new(0),
            current_usecase_bad_bytes: AtomicUsize::new(0),
            results: OnceCell::new(),
            _phantom: PhantomData,
        }
    }

    /// Get statistics for a single usecase.
    ///
    /// This function is cheaper than `flush` but currently not by much. This may change in the
    /// future.
    pub fn get(&self, use_case: U) -> Stat {
        let results = match self.results.get() {
            Some(x) => x,
            None => return Stat::default(),
        };

        results
            .get(&use_case.into())
            .map(|stat| *stat)
            .unwrap_or_default()
    }

    fn get_mut(&self, use_case: U) -> impl DerefMut<Target = Stat> + '_ {
        self.results
            .get_or_init(DashMap::new)
            .entry(use_case.into())
            .or_insert_with(Default::default)
    }

    fn get_error_atomic(&self, code: Error) -> &AtomicUsize {
        match code {
            Error::CurrentUsecaseContentionRefCell => &self.current_usecase_contention_ref_cell,
            Error::CurrentUsecaseContentionThreadLocal => {
                &self.current_usecase_contention_thread_local
            }
            Error::CurrentUsecaseBadBytes => &self.current_usecase_bad_bytes,
        }
    }

    /// Check how often an error has occurred
    pub fn get_error(&self, code: Error) -> usize {
        self.get_error_atomic(code).load(Ordering::Relaxed)
    }

    /// Return all recorded statistics and reset internal state.
    ///
    /// This method is somewhat expensive in that it acquires global resources mutably.
    pub fn flush(&self, mut stat_fn: impl FnMut(U, Stat), mut error_fn: impl FnMut(Error, usize)) {
        if let Some(results) = self.results.get() {
            for kv in results.iter() {
                stat_fn(U::try_from(*kv.key()).unwrap_or_default(), *kv.value());
            }
            results.clear();
        }

        error_fn(
            Error::CurrentUsecaseBadBytes,
            self.get_error(Error::CurrentUsecaseBadBytes),
        );
        error_fn(
            Error::CurrentUsecaseContentionRefCell,
            self.get_error(Error::CurrentUsecaseContentionRefCell),
        );
        error_fn(
            Error::CurrentUsecaseContentionThreadLocal,
            self.get_error(Error::CurrentUsecaseContentionThreadLocal),
        );
    }
}

unsafe impl<U: UseCase> Recorder<U> for StatsRecorder<U> {
    fn on_alloc(&self, use_case: U, size: usize) -> bool {
        self.get_mut(use_case).record(size as isize);
        true
    }

    fn on_dealloc(&self, use_case: U, size: usize) {
        self.get_mut(use_case).record(-(size as isize));
    }

    fn on_error(&self, code: Error, _size: Option<usize>) {
        self.get_error_atomic(code).fetch_add(1, Ordering::Relaxed);
    }
}

/// Basic memory stats for a given usecase.
#[derive(Default, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Stat {
    /// The amount of memory currently used.
    pub current: isize,
    /// The largest amount of memory ever used at a point in time.
    pub peak: isize,
    /// The amount of memory allocated in total, regardless of whether it was deallocated or not.
    pub total: isize,
}

impl fmt::Display for Stat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "current: {}, peak: {}, total: {}",
            self.current, self.peak, self.total
        )
    }
}

impl Stat {
    fn record(&mut self, size: isize) {
        self.current += size;

        if self.current > self.peak {
            self.peak = self.current;
        }

        if size > 0 {
            self.total += size;
        }
    }
}
