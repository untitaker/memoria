use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::MutexGuard;

// https://stackoverflow.com/a/71945606/1544347
pub type PhantomUnsend = PhantomData<MutexGuard<'static, ()>>;
pub type PhantomUnsync = PhantomData<Cell<()>>;
