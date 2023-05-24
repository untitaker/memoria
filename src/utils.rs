use std::sync::MutexGuard;
use std::cell::Cell;
use std::marker::PhantomData;

// https://stackoverflow.com/a/71945606/1544347
pub type PhantomUnsend = PhantomData<MutexGuard<'static, ()>>;
pub type PhantomUnsync = PhantomData<Cell<()>>;
